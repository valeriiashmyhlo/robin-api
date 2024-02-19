use axum::http::StatusCode;
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
use sqlx::{Pool, Postgres};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::{net::TcpListener, sync::broadcast};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod model;

#[derive(Eq, Hash, PartialEq, Serialize)]
struct User {
    id: u64,
    first_name: String,
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

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "robin_chat=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let user_set = Mutex::new(HashSet::<String>::new());
    let (tx, _rx) = broadcast::channel(100);
    let db = pool.clone();

    let app_state = Arc::new(AppState { user_set, tx, db });

    let app = Router::new()
        .route("/login", post(login))
        .route("/websocket", get(websocket_handler))
        .with_state(app_state);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
pub struct Login {
    pub password: String,
}

#[debug_handler]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(props): Json<Login>,
) -> Result<impl IntoResponse, StatusCode> {
    let result = model::User::get(&state.db, props)
        .await
        .map_err(|e| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(User {
        id: result.id as u64,
        first_name: result.first_name,
    }))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<AppState>) {
    let mut username = String::new();

    let (mut sender, mut receiver) = stream.split();

    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(name) = message {
            check_username(&state, &mut username, &name);

            if !username.is_empty() {
                break;
            } else {
                let _ = sender
                    .send(Message::Text(String::from("Username already taken.")))
                    .await;

                return;
            }
        }
    }

    let mut rx = state.tx.subscribe();
    let msg = format!("{username} joined.");
    tracing::debug!("{msg}");
    let _ = state.tx.send(msg);

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let tx = state.tx.clone();
    let name = username.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let _ = tx.send(format!("{name}: {text}"));
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    let msg = format!("{username} left.");
    tracing::debug!("{msg}");
    let _ = state.tx.send(msg);

    state.user_set.lock().unwrap().remove(&username);
}

fn check_username(state: &AppState, string: &mut String, name: &str) {
    let mut user_set = state.user_set.lock().unwrap();

    if !user_set.contains(name) {
        user_set.insert(name.to_owned());

        string.push_str(name);
    }
}

// TODO: Remove below fn
// async fn index() -> Html<&'static str> {
//     Html(std::include_str!("../chat.html"))
// }
