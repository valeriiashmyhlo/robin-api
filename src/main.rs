use axum::{
    debug_handler,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Request, State,
    },
    response::IntoResponse,
    routing::{get, post},
    serve, Json, Router,
};
use dotenv::dotenv;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use sqlx::{Pool, Postgres};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::cors::CorsLayer;
use tracing::log::{set_max_level, LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::{app_error::AppError, model::ModelUser};

mod app_error;
mod db;
mod model;

#[derive(Eq, Hash, PartialEq, Serialize)]
struct User {
    id: u64,
    first_name: String,
    token: Uuid,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum RequestMessage {
    Join { token: Uuid },
    Message { content: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
enum ResponseMessage {
    Join { first_name: String },
    Leave { first_name: String },
    Message { first_name: String, content: String },
}

struct AppState {
    // user_set: Mutex<HashSet<ModelUser>>,
    tx: broadcast::Sender<ResponseMessage>,
    db: Pool<Postgres>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let pool = db::connect_db().await;
    set_max_level(LevelFilter::Debug);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "app,sqlx,info,axum::rejection=trace".into());
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().compact().pretty())
        .init();

    // let user_set = Mutex::new(HashSet::<ModelUser>::new());
    let (tx, _rx) = broadcast::channel(100);
    let db = pool.clone();

    let app_state = Arc::new(AppState { tx, db });

    let app = Router::new()
        .route("/login", post(login))
        .route("/websocket", get(websocket_handler))
        .with_state(app_state)
        .layer(CorsLayer::permissive());

    let listener = TcpListener::bind("127.0.0.1:3001").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    serve(listener, app).await.unwrap();
}

#[derive(Deserialize, Debug)]
pub struct Login {
    pub first_name: String,
    pub password: String,
}

#[debug_handler]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(props): Json<Login>,
) -> Result<impl IntoResponse, AppError> {
    let result = ModelUser::get(&state.db, props).await?;

    Ok(Json(User {
        id: result.id as u64,
        first_name: result.first_name,
        token: result.token,
    }))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();

    let mut rx = state.tx.subscribe();

    let Some(Ok(Message::Text(text))) = receiver.next().await else {
        panic!("Invalid message");
    };

    let token = match async { from_str(&text) }.await {
        Ok(RequestMessage::Join { token }) => token,
        _ => {
            panic!("Invalid message2");
        }
    };

    let user = ModelUser::get_by_token(&state.db, token).await.unwrap();
    let _ = state.tx.send(ResponseMessage::Join {
        first_name: user.first_name.clone(),
    });

    let mut send_task = tokio::spawn(async move {
        while let msg = rx.recv().await {
            if sender
                .send(Message::Text(to_string(&msg.unwrap()).unwrap()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let tx_clone = state.tx.clone();
    let first_name = user.first_name.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(RequestMessage::Message { content }) = from_str(&text) {
                tx_clone
                    .send(ResponseMessage::Message {
                        first_name: first_name.clone(),
                        content,
                    })
                    .unwrap();
            } else {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // let msg = format!("{first_name_clone} left.");
    // tracing::debug!("{msg}");
    let _ = state.tx.send(ResponseMessage::Leave {
        first_name: user.first_name.clone(),
    });

    // state.user_set.lock().unwrap().remove(&user);
}

// 1. Add a history of messages after joined
// 2. Add a list of connected users
// 3. Refactor
// 4. Add useReducer to handle messages from back
