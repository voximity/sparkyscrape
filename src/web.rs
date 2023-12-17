use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json,
};
use bytes::Bytes;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;
use serde_json::{json, Value};
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use tokio::sync::mpsc;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

use crate::{
    handler::{LevelDatabase, DCT_PLAN},
    level::{Coefficients, LevelDifficulty},
    CHANNELS,
};

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

async fn get_multipart_bytes(
    multipart: &mut Multipart,
    field_name: &str,
) -> Result<Bytes, anyhow::Error> {
    while let Some(field) = multipart.next_field().await? {
        if matches!(field.name(), Some(f) if f == field_name) {
            return Ok(field.bytes().await?);
        }
    }

    anyhow::bail!("field {} not found", field_name)
}

async fn api_guess(
    State(state): State<Arc<AppState>>,
    Path(difficulty): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    let difficulty: LevelDifficulty = difficulty
        .parse()
        .map_err(|_| anyhow::anyhow!("failed to parse level"))?;

    let data = get_multipart_bytes(&mut multipart, "data").await?;
    let coefficients = Coefficients::new(&data, DCT_PLAN.to_owned())?;

    let level_state = state.database.get(&difficulty).unwrap().read().await;
    let mut guesses = level_state
        .par_iter()
        .map(|(_, level)| (level, level.euclidean_distance_to(&coefficients)))
        .collect::<Vec<_>>();

    guesses.sort_by(|(_, a), (_, b)| a.total_cmp(b));
    let (level, dist) = guesses
        .first()
        .ok_or(anyhow::anyhow!("no guesses for this difficulty"))?;

    Ok(Json(json!({
        "level": level.name.to_owned(),
        "distance": dist,
    })))
}

pub struct Init {
    pub database: LevelDatabase,
}

pub struct AppState {
    pub database: LevelDatabase,
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self(value.into())
    }
}

pub async fn init(data: Init) -> anyhow::Result<mpsc::UnboundedSender<WebMessage>> {
    let (tx, mut rx) = mpsc::unbounded_channel::<WebMessage>();

    let (layer, io) = SocketIo::new_layer();

    io.ns("/", on_connect);

    // message emit loop
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            io.emit(msg.event(), msg).unwrap();
        }
    });

    let app_state = Arc::new(AppState {
        database: data.database,
    });

    let app = axum::Router::new()
        .nest(
            "/api",
            axum::Router::new().route("/guess/:difficulty", post(api_guess)),
        )
        .nest_service("/levels", ServeDir::new("levels"))
        .nest_service("/static", ServeDir::new("frontend/build/static"))
        .fallback_service(ServeDir::new("frontend/build"))
        .fallback_service(ServeFile::new("frontend/build/index.html"))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(layer),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3579").await.unwrap();

    // listen in another thread
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Ok(tx)
}
