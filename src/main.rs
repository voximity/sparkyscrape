mod handler;
mod level;

use std::{collections::HashMap, io::Cursor, path::Path, sync::Arc, time::Duration};

use byteorder::{ReadBytesExt, LE};
use colored::Colorize;
use handler::{ChannelStateData, Handler, LevelDatabaseData, RawHandler};
use lazy_static::lazy_static;
use level::Level;
use serde::{Deserialize, Serialize};
use serenity::{all::ChannelId, Client};
use tokio::sync::RwLock;

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

    #[serde(default)]
    pub save_images: bool,
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
            // create image difficulty folders
            if CONFIG.save_images {
                tokio::fs::create_dir_all(format!("images/{}", difficulty.directory()))
                    .await
                    .unwrap();
            }

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
