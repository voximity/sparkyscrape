mod level;

use std::{collections::HashMap, io::Cursor, path::Path, sync::Arc, time::Duration};

use byteorder::{ReadBytesExt, LE};
use colored::Colorize;
use lazy_static::lazy_static;
use level::{Coefficients, Level};
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
    Client,
};
use tokio::{io::AsyncWriteExt, sync::RwLock};

use crate::level::{LevelDifficulty, IMAGE_DIM};

lazy_static! {
    static ref MENTION_REGEX: Regex = Regex::new(r"<@!?(\d+)>").unwrap();
    static ref CONFIG: Config =
        serde_json::from_reader(std::fs::File::open("config.json").unwrap()).unwrap();
    static ref CHANNELS: Vec<ChannelId> = CONFIG
        .channels
        .iter()
        .map(|c| ChannelId::new(c.parse().unwrap()))
        .collect::<Vec<_>>();
    static ref DCT_PLAN: Arc<dyn TransformType2And3<f32>> =
        DctPlanner::new().plan_dct2(IMAGE_DIM * IMAGE_DIM);
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    token: String,
    server_id: String,
    bot_id: String,
    channels: Vec<String>,
    unprotected_ip: Option<String>,
}

struct ChannelState {
    url: String,
    guess: Option<(String, f32)>,
    coefficients: Option<Coefficients>,
    difficulty: LevelDifficulty,
    guesses: HashMap<UserId, String>,
}

struct ChannelStateData;
impl TypeMapKey for ChannelStateData {
    type Value = Arc<RwLock<HashMap<ChannelId, ChannelState>>>;
}

struct LevelDatabaseData;
impl TypeMapKey for LevelDatabaseData {
    type Value = Arc<HashMap<LevelDifficulty, RwLock<HashMap<String, Level>>>>;
}

async fn save_levels<'a, I: 'a + Iterator<Item = &'a Level>>(
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

struct Handler;

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
                    guess: None,
                    coefficients: None,
                    difficulty,
                    guesses: HashMap::new(),
                },
            );
            println!("{} new {} level", channel_prefix, difficulty);

            let reqwest = reqwest::Client::new();
            let bytes = reqwest
                .get(&image.url)
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();

            // update level coefficients
            let coefficients = Coefficients::new(&bytes, DCT_PLAN.clone()).unwrap();
            state
                .write()
                .await
                .get_mut(&ev.channel_id)
                .unwrap()
                .coefficients = Some(coefficients);

            // get our best guess

            // TODO: guesses should internally be categorized by difficulty
            // TODO: so that easy guesses are not checked with medium guesses

            let level_state = {
                let data = ctx.data.read().await;
                data.get::<LevelDatabaseData>().unwrap().clone()
            };

            let levels = level_state.get(&difficulty).unwrap().read().await;
            let mut guesses = levels
                .values()
                .map(|level| (level, level.euclidean_distance_to(&coefficients)))
                .collect::<Vec<_>>();

            guesses.sort_by(|(_, a), (_, b)| a.total_cmp(b));
            if let Some((best_guess, dist)) = guesses.first() {
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

                state.write().await.get_mut(&ev.channel_id).unwrap().guess =
                    Some((best_guess.name.to_string(), *dist));
            }
        }

        Some(Embed {
            title: Some(title),
            description: Some(desc),
            ..
        }) if title == "Congratulations! You guessed the Level correctly!" => {
            // TODO: determine who wins
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
                        // our guess was correct
                        match &channel_state.guess {
                            Some((my_guess, dist)) if my_guess == &answer.to_lowercase() => {
                                println!(
                                    "{} {} my guess was correct: {} (dist {})",
                                    channel_prefix,
                                    "I was right!".bold().underline(),
                                    channel_state.difficulty.colorize(my_guess.as_str()).bold(),
                                    dist,
                                );
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

                        if level_state
                            .get(&channel_state.difficulty)
                            .unwrap()
                            .read()
                            .await
                            .get(&answer.to_lowercase())
                            .is_some()
                        {
                            println!("{} {}", channel_prefix, "I already knew that one!".red());
                        }

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

struct RawHandler;

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

#[tokio::main]
async fn main() {
    // make sure we are on VPN
    if let Some(ip) = &CONFIG.unprotected_ip {
        let reqwest = reqwest::Client::new();
        let out = reqwest
            .get("http://v4.ident.me/")
            .send()
            .await
            .expect("IPv4 from v4.ident.me")
            .text()
            .await
            .expect("IPv4 from v4.ident.me");

        if ip == &out {
            println!("{} using unprotected IP! halting...", "error!".red().bold());
            std::process::exit(1);
        }
    } else {
        println!(
            "{} unprotected_ip is not set in config! waiting 10 seconds before continuing...",
            "warning!".yellow().bold()
        );

        tokio::time::sleep(Duration::from_secs(10)).await;
    }

    let mut cache_settings = serenity::cache::Settings::default();
    cache_settings.max_messages = 200;

    let mut client = Client::builder(&CONFIG.token)
        .event_handler(Handler)
        .raw_event_handler(RawHandler)
        .cache_settings(cache_settings)
        .await
        .expect("error creating client");

    // initialize the cache
    {
        let mut data = client.data.write().await;
        data.insert::<ChannelStateData>(Arc::new(RwLock::new(HashMap::new())));

        // read levels
        let mut map = HashMap::new();

        for difficulty in &[
            LevelDifficulty::Easy,
            LevelDifficulty::Medium,
            LevelDifficulty::Hard,
            LevelDifficulty::Legendary,
        ] {
            let mut levels = vec![];

            if Path::new(difficulty.filename()).exists() {
                let mut cursor = Cursor::new(tokio::fs::read(difficulty.filename()).await.unwrap());
                let count = cursor.read_u64::<LE>().unwrap();
                for _ in 0..count {
                    let level = Level::read(&mut cursor).unwrap();
                    if level.name.starts_with("s?") {
                        continue;
                    }

                    levels.push((level.name.to_owned(), level));
                }
                println!("read in {} {} levels", levels.len(), difficulty);
            } else {
                println!("no {} levels", difficulty);
            }

            map.insert(
                *difficulty,
                RwLock::new(levels.into_iter().collect::<HashMap<_, _>>()),
            );
        }

        data.insert::<LevelDatabaseData>(Arc::new(map));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
