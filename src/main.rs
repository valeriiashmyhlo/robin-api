use axum::{
    debug_handler,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    serve, Json, Router,
};
use dotenv::dotenv;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
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

use crate::app_error::AppError;

mod app_error;
mod db;
mod model;

#[derive(Eq, Hash, PartialEq, Serialize)]
struct User {
    id: u64,
    first_name: String,
    token: Uuid,
}

#[derive(Deserialize)]
struct UserMessage {
    first_name: String,
    token: Uuid,
    content: Option<String>,
}

struct AppState {
    user_set: Mutex<HashSet<String>>,
    tx: broadcast::Sender<String>,
    db: Pool<Postgres>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let pool = db::connect_db().await;

    set_max_level(LevelFilter::Debug);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "app,sqlx,info,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer().compact().pretty())
        .init();

    let user_set = Mutex::new(HashSet::<String>::new());
    let (tx, _rx) = broadcast::channel(100);
    let db = pool.clone();

    let app_state = Arc::new(AppState { user_set, tx, db });

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
    let result = model::User::get(&state.db, props).await?;

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

    // read username and send to all subscribers;
    let Some(Ok(Message::Text(text))) = receiver.next().await else {
        panic!("Username not provided");
    };
    let message: UserMessage = match async { from_str(&text) }.await {
        Ok(result) => result,
        Err(err) => {
            panic!("Error parsing message: {}", err);
        }
    };
    let username = message.first_name;

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let tx_clone = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            async {
                let message: UserMessage = from_str(&text)?;
                let first_name = message.first_name;
                let content = match message.content {
                    Some(content) => content,
                    None => {
                        let msg = format!("{first_name} joined.");
                        tracing::debug!("{msg}");
                        tx_clone.send(msg);
                        return Ok::<(), AppError>(());
                    }
                };

                tx_clone.send(format!("{first_name}: {content}"));
                Ok(())
            }
            .await;
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    let msg = format!("{username} left.");
    tracing::debug!("{msg}");
    state.tx.send(msg);

    state.user_set.lock().unwrap().remove(&username);
}
