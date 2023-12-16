use std::collections::HashMap;

use axum::routing::get;
use serde::Serialize;
use serde_json::Value;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use tokio::sync::mpsc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::{level::LevelDifficulty, CHANNELS};

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum WebMessage {
    Hello {
        channels: HashMap<String, String>,
    },
    GuessStart {
        channel_id: String,
        difficulty: LevelDifficulty,
    },
    GuessData {
        channel_id: String,
        guess: Option<(String, f32)>,
    },
    GuessWin {
        channel_id: String,
        answer: String,
        incorrect: bool,
    },
    GuessTimeout {
        channel_id: String,
    },
}

impl WebMessage {
    pub fn event(&self) -> &'static str {
        match self {
            Self::Hello { .. } => "hello",
            Self::GuessStart { .. } => "guess/start",
            Self::GuessData { .. } => "guess/data",
            Self::GuessWin { .. } => "guess/win",
            Self::GuessTimeout { .. } => "guess/timeout",
        }
    }

    pub fn emit(&self, socket: SocketRef) -> Result<(), socketioxide::SendError> {
        socket.emit(self.event(), self)
    }
}

fn on_connect(socket: SocketRef, Data(_): Data<Value>) {
    println!("socket.io connected: {} {}", socket.ns(), socket.id);

    // send the hello message
    WebMessage::Hello {
        channels: CHANNELS
            .iter()
            .enumerate()
            .map(|(i, id)| (id.to_string(), format!("Channel {}", i + 1)))
            .collect::<HashMap<_, _>>(),
    }
    .emit(socket)
    .unwrap();
}

pub async fn init() -> anyhow::Result<mpsc::UnboundedSender<WebMessage>> {
    let (tx, mut rx) = mpsc::unbounded_channel::<WebMessage>();

    let (layer, io) = SocketIo::new_layer();

    io.ns("/", on_connect);

    // message emit loop
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            io.emit(msg.event(), msg).unwrap();
        }
    });

    let app = axum::Router::new()
        .route("/", get(|| async { "Hello, world!" }))
        .nest_service("/levels", ServeDir::new("levels"))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(layer),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3579").await.unwrap();

    // listen in another thread
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Ok(tx)
}
