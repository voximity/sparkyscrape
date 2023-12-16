mod handler;
mod level;
mod web;

use std::{
    collections::{hash_map::Entry, HashMap},
    process,
    sync::Arc,
    time::Duration,
};

use clap::{Parser, Subcommand};
use colored::Colorize;
use handler::{save_levels, ChannelStateData, Handler, LevelDatabaseData, RawHandler};
use lazy_static::lazy_static;
use level::read_levels;
use serde::{Deserialize, Serialize};
use serenity::{all::ChannelId, prelude::TypeMapKey, Client};
use tokio::sync::{mpsc, RwLock};

use crate::level::LevelDifficulty;

lazy_static! {
    static ref CONFIG: Config =
        serde_json::from_reader(std::fs::File::open("config.json").unwrap()).unwrap();
    static ref CHANNELS: Vec<ChannelId> = CONFIG
        .channels
        .iter()
        .map(|c| ChannelId::new(c.parse().unwrap()))
        .collect::<Vec<_>>();
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub token: String,
    pub server_id: String,
    pub bot_id: String,
    pub channels: Vec<String>,
    pub unprotected_ip: Option<String>,
}

pub struct WebMessageTxData;
impl TypeMapKey for WebMessageTxData {
    type Value = Arc<mpsc::UnboundedSender<web::WebMessage>>;
}

#[derive(Parser)]
struct Cli {
    /// skips the unprotected IP check
    #[arg(long)]
    skip_ip_check: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// rename a level in the database
    Rename {
        /// the level's difficulty to rename
        #[arg(short, long, required = true)]
        difficulty: String,

        /// the old level name
        #[arg(short, long, required = true)]
        from: String,

        /// the new level name
        #[arg(short, long, required = true)]
        to: String,
    },

    /// remove a level from the database
    Remove {
        /// the level's difficulty to remove
        #[arg(short, long, required = true)]
        difficulty: String,

        /// the level's name to remove
        #[arg(short, long, required = true)]
        level: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    #[allow(clippy::single_match)]
    match cli.command {
        Some(Command::Rename {
            difficulty,
            from,
            to,
        }) => {
            let difficulty = difficulty.parse().unwrap();
            let mut levels = read_levels(difficulty).await;

            if let Entry::Occupied(mut entry) = levels.entry(from.to_owned()) {
                entry.get_mut().name = to.to_owned();
                save_levels(difficulty, levels.len(), levels.values())
                    .await
                    .unwrap();
                println!(
                    "renamed {} to {}",
                    difficulty.colorize(from.as_str()),
                    difficulty.colorize(to.as_str())
                );
            } else {
                println!("could not find a level with the name {}", from.red());
                process::exit(1);
            }

            process::exit(0);
        }

        Some(Command::Remove { difficulty, level }) => {
            let difficulty = difficulty.parse().unwrap();
            let mut levels = read_levels(difficulty).await;

            if let Entry::Occupied(entry) = levels.entry(level.to_owned()) {
                entry.remove();
                save_levels(difficulty, levels.len(), levels.values())
                    .await
                    .unwrap();
                println!("removed {}", difficulty.colorize(level.as_str()),);
            } else {
                println!("could not find a level with the name {}", level.red());
                process::exit(1);
            }

            process::exit(0);
        }

        _ => (),
    }

    if !cli.skip_ip_check {
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
    }

    // start the web app
    let web_tx = web::init().await.unwrap();

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
        data.insert::<WebMessageTxData>(Arc::new(web_tx));

        // read levels
        let mut map = HashMap::new();

        for difficulty in &[
            LevelDifficulty::Easy,
            LevelDifficulty::Medium,
            LevelDifficulty::Hard,
            LevelDifficulty::Legendary,
        ] {
            map.insert(*difficulty, RwLock::new(read_levels(*difficulty).await));
        }

        data.insert::<LevelDatabaseData>(Arc::new(map));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
