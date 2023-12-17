use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    level::{Coefficients, Level},
    web::WebMessage,
    WebMessageTxData,
};
use bytes::Bytes;
use colored::Colorize;
use lazy_static::lazy_static;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use rustdct::{DctPlanner, TransformType2And3};
use serde::{Deserialize, Serialize};
use serenity::{
    all::{ChannelId, Embed, Event, MessageUpdateEvent, UnknownEvent, UserId},
    async_trait,
    client::{Context, EventHandler, RawEventHandler},
    json::json,
    model::channel::Message,
    prelude::TypeMapKey,
};
use tokio::{io::AsyncWriteExt, sync::RwLock};

use crate::{
    level::{LevelDifficulty, IMAGE_DIM},
    CHANNELS, CONFIG,
};

lazy_static! {
    static ref MENTION_REGEX: Regex = Regex::new(r"<@!?(\d+)>").unwrap();
    static ref DCT_PLAN: Arc<dyn TransformType2And3<f32>> =
        DctPlanner::new().plan_dct2(IMAGE_DIM * IMAGE_DIM);
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub token: String,
    pub server_id: String,
    pub bot_id: String,
    pub channels: Vec<String>,
    pub unprotected_ip: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChannelState {
    pub url: String,
    pub bytes: Option<Bytes>,
    pub guess: Option<(String, f32)>,
    pub coefficients: Option<Coefficients>,
    pub difficulty: LevelDifficulty,
    pub guesses: HashMap<UserId, String>,
}

pub struct ChannelStateData;
impl TypeMapKey for ChannelStateData {
    type Value = Arc<RwLock<HashMap<ChannelId, ChannelState>>>;
}

pub struct LevelDatabaseData;
impl TypeMapKey for LevelDatabaseData {
    type Value = Arc<HashMap<LevelDifficulty, RwLock<HashMap<String, Level>>>>;
}

pub async fn save_levels<'a, I: 'a + Iterator<Item = &'a Level>>(
    difficulty: LevelDifficulty,
    num_levels: usize,
    levels: I,
) -> tokio::io::Result<()> {
    let mut file = tokio::fs::File::create(difficulty.filename()).await?;
    file.write_u64_le(num_levels as u64).await?;
    for level in levels {
        level.write(&mut file).await?;
    }
    Ok(())
}

pub struct Handler;

async fn send_web_message(ctx: &Context, message: WebMessage) {
    ctx.data
        .read()
        .await
        .get::<WebMessageTxData>()
        .unwrap()
        .clone()
        .send(message)
        .unwrap();
}

async fn handle_bot_message(ctx: Context, ev: Message) {
    // remove bot messages
    if ev.content.starts_with("s?") {
        return;
    }

    // we only care about channels in the registered channel list
    if !CHANNELS.contains(&ev.channel_id) {
        return;
    }

    // if the message was not sent by sparky, we treat it as a guess
    if ev.author.id != UserId::new(CONFIG.bot_id.parse().unwrap()) {
        let state = {
            let data = ctx.data.read().await;
            data.get::<ChannelStateData>().unwrap().clone()
        };

        if let Some(state) = state.write().await.get_mut(&ev.channel_id) {
            state.guesses.insert(ev.author.id, ev.content.to_owned());
        }

        return;
    }

    let channel_prefix = format!(
        "[ #{} ]",
        CHANNELS.iter().position(|c| c == &ev.channel_id).unwrap() + 1
    )
    .white();

    match ev.embeds.first() {
        Some(Embed {
            title: Some(title),
            image: Some(image),
            description: Some(desc),
            ..
        }) if title == "Guess the Level!" => {
            {
                let state = {
                    let data = ctx.data.read().await;
                    data.get::<ChannelStateData>().unwrap().clone()
                };

                if matches!(
                    state.read().await.get(&ev.channel_id),
                    Some(ChannelState { url, .. }) if url == &image.url
                ) {
                    // this URL is already active here, unimportant message update
                    return;
                }
            }

            let state = {
                let data = ctx.data.read().await;
                data.get::<ChannelStateData>().unwrap().clone()
            };

            let difficulty = match desc.as_str() {
                "**Difficulty:** Medium" => LevelDifficulty::Medium,
                "**Difficulty:** Hard" => LevelDifficulty::Hard,
                "**Difficulty:** Legendary" => LevelDifficulty::Legendary,
                _ => LevelDifficulty::Easy,
            };

            // immediately set base channel state
            state.write().await.insert(
                ev.channel_id,
                ChannelState {
                    url: image.url.to_owned(),
                    bytes: None,
                    guess: None,
                    coefficients: None,
                    difficulty,
                    guesses: HashMap::new(),
                },
            );
            println!("{} new {} level", channel_prefix, difficulty);

            // send web message
            send_web_message(
                &ctx,
                WebMessage::GuessStart {
                    channel_id: ev.channel_id.to_string(),
                    difficulty,
                },
            )
            .await;

            let reqwest = reqwest::Client::new();
            let bytes = reqwest
                .get(&image.url)
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();

            // update level state data
            let coefficients = Coefficients::new(&bytes, DCT_PLAN.clone()).unwrap();
            {
                let mut channels = state.write().await;
                match channels.get_mut(&ev.channel_id) {
                    Some(state) if state.url == image.url => {
                        state.coefficients = Some(coefficients);
                        state.bytes = Some(bytes.to_owned());
                    }
                    _ => {
                        return;
                    }
                }
            }

            // get our best guess
            let level_state = {
                let data = ctx.data.read().await;
                data.get::<LevelDatabaseData>().unwrap().clone()
            };

            let levels = level_state.get(&difficulty).unwrap().read().await;
            let mut guesses = levels
                .par_iter()
                .map(|(_, level)| (level, level.euclidean_distance_to(&coefficients)))
                .collect::<Vec<_>>();

            guesses.sort_by(|(_, a), (_, b)| a.total_cmp(b));
            if let Some((best_guess, dist)) = guesses.first() {
                // update state, return if state has changed to a new URL
                match state.write().await.get_mut(&ev.channel_id) {
                    Some(state) if state.url == image.url => {
                        state.guess = Some((best_guess.name.to_string(), *dist));
                    }
                    _ => {
                        return;
                    }
                }

                println!(
                    "{} my best guess is {} (dist {}{})",
                    channel_prefix,
                    best_guess.difficulty.colorize(best_guess.name.as_str()),
                    dist,
                    if dist < &500f32 {
                        " !!!".bold().bright_yellow().to_string()
                    } else {
                        "".to_string()
                    }
                );
            }

            // save active image
            let filename = format!("levels/{}.png", ev.channel_id);

            tokio::spawn(async move {
                let path = Path::new(&filename);
                tokio::fs::write(path, bytes)
                    .await
                    .expect("failed to save image")
            });

            // send web message with guess
            send_web_message(
                &ctx,
                WebMessage::GuessData {
                    channel_id: ev.channel_id.to_string(),
                    guess: guesses
                        .iter()
                        .map(|(level, dist)| (level.name.to_owned(), *dist))
                        .next(),
                },
            )
            .await;
        }

        Some(Embed {
            title: Some(title),
            description: Some(desc),
            ..
        }) if title == "Congratulations! You guessed the Level correctly!" => {
            let state = {
                let data = ctx.data.read().await;
                data.get::<ChannelStateData>().unwrap().clone()
            };

            // ignore if we didn't have state for this channel
            {
                let mut whole_state = state.write().await;
                let channel_state = match whole_state.remove(&ev.channel_id) {
                    Some(c) => c,
                    None => return,
                };

                // determine the winner
                let captures = MENTION_REGEX.captures(desc.as_str());
                if let Some(captures) = captures {
                    let id: UserId =
                        UserId::new(captures.get(1).unwrap().as_str().parse().unwrap());

                    if let Some(answer) = channel_state.guesses.get(&id) {
                        // save the image in another thread if we don't already have it
                        if let Some(bytes) = channel_state.bytes {
                            let filename = format!(
                                "levels/{}/{}.png",
                                channel_state.difficulty.directory(),
                                answer.to_lowercase()
                            );

                            tokio::spawn(async move {
                                // TODO: save when we update coefficients
                                let path = Path::new(&filename);
                                if !path.exists() {
                                    tokio::fs::write(path, bytes)
                                        .await
                                        .expect("failed to save image")
                                }
                            });
                        }

                        // check if our guess was correct
                        match &channel_state.guess {
                            Some((my_guess, dist)) if my_guess == &answer.to_lowercase() => {
                                println!(
                                    "{} {} my guess was correct: {} (dist {})",
                                    channel_prefix,
                                    "I was right!".bold().underline(),
                                    channel_state.difficulty.colorize(my_guess.as_str()).bold(),
                                    dist,
                                );

                                send_web_message(
                                    &ctx,
                                    WebMessage::GuessWin {
                                        channel_id: ev.channel_id.to_string(),
                                        answer: my_guess.to_string(),
                                        incorrect: false,
                                    },
                                )
                                .await;

                                return;
                            }
                            _ => (),
                        }

                        println!(
                            "{} I was wrong, winning guess: {}",
                            channel_prefix,
                            channel_state
                                .difficulty
                                .colorize(answer.to_lowercase().bold())
                        );

                        let level_state = {
                            let data = ctx.data.read().await;
                            data.get::<LevelDatabaseData>().unwrap().clone()
                        };

                        // leave a message if we already knew the winning level
                        // (something probably went wrong, update DCT coefficients?)
                        let incorrect = level_state
                            .get(&channel_state.difficulty)
                            .unwrap()
                            .read()
                            .await
                            .get(&answer.to_lowercase())
                            .is_some();

                        if incorrect {
                            println!("{} {}", channel_prefix, "I already knew that one!".red());
                        };

                        // insert the new level into the database
                        level_state
                            .get(&channel_state.difficulty)
                            .unwrap()
                            .write()
                            .await
                            .insert(
                                answer.to_lowercase(),
                                Level {
                                    name: answer.to_lowercase(),
                                    difficulty: channel_state.difficulty,
                                    coefficients: channel_state
                                        .coefficients
                                        .expect("DCT coefficients"),
                                },
                            );

                        // notify web of result
                        send_web_message(
                            &ctx,
                            WebMessage::GuessWin {
                                channel_id: ev.channel_id.to_string(),
                                answer: answer.to_lowercase(),
                                incorrect,
                            },
                        )
                        .await;

                        // save levels to file
                        {
                            let levels = level_state
                                .get(&channel_state.difficulty)
                                .unwrap()
                                .read()
                                .await;

                            save_levels(channel_state.difficulty, levels.len(), levels.values())
                                .await
                                .expect("saved levels");
                        }
                    }
                }
            }
        }

        Some(Embed {
            title: Some(title), ..
        }) if title == "Time is up!" => {
            let state = {
                let data = ctx.data.read().await;
                data.get::<ChannelStateData>().unwrap().clone()
            };

            state.write().await.remove(&ev.channel_id);

            send_web_message(
                &ctx,
                WebMessage::GuessTimeout {
                    channel_id: ev.channel_id.to_string(),
                },
            )
            .await;
        }

        _ => (),
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, ev: Message) {
        handle_bot_message(ctx, ev).await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        _: Option<Message>,
        new: Option<Message>,
        _: MessageUpdateEvent,
    ) {
        if let Some(new) = new {
            handle_bot_message(ctx, new).await;
        }
    }
}

pub struct RawHandler;

#[async_trait]
impl RawEventHandler for RawHandler {
    async fn raw_event(&self, _ctx: Context, ev: Event) {
        match ev {
            Event::Unknown(UnknownEvent { kind, .. }) if kind == "READY" => {
                println!("ready!");

                // tell gateway we want to listen to messages
                _ctx.shard
                    .websocket_message(tokio_tungstenite::tungstenite::Message::text(
                        json!({
                            "op": 14,
                            "d": {
                                "guild_id": &CONFIG.server_id,
                                "typing": true,
                                "threads": true,
                                "activities": true,
                                "members": [],
                                "channels": {},
                                "thread_member_lists": [],
                            }
                        })
                        .to_string(),
                    ));
            }

            _ => (),
        }
    }
}
