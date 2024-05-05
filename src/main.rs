use axum::{
    routing::{get, post},
    serve, Router,
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::cors::CorsLayer;
use tracing::log::{set_max_level, LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;
use websocket::Controller;

use crate::models::{HistoryMessage, ModelUser};

mod app_error;
mod db;
mod login;
mod models;
mod websocket;

#[derive(Eq, Hash, PartialEq, Serialize, Deserialize, Clone, Debug)]
struct AuthorisedUser {
    id: Uuid,
    username: String,
    token: Uuid,
}

#[derive(Eq, Hash, PartialEq, Serialize, Deserialize, Clone, Debug)]
struct User {
    id: Uuid,
    username: String,
}

impl User {
    fn from_model_user(user: ModelUser) -> User {
        User {
            id: user.id,
            username: user.username,
        }
    }
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
    Join {
        user: User,
    },
    Leave {
        user: User,
    },
    Message {
        username: String,
        content: String,
    },
    History {
        messages: Vec<HistoryMessage>,
        users: Vec<User>,
    },
}

pub struct AppState {
    broadcast_sender: broadcast::Sender<ResponseMessage>,
    db: Pool<Postgres>,
    controller: Controller,
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

    // let user_set = Arc::new(Mutex::new(HashSet::<User>::new()));
    let (broadcast_sender, _broadcast_receiver) = broadcast::channel(100);
    // let db = pool.clone();

    let app_state = Arc::new(AppState {
        broadcast_sender: broadcast_sender.clone(),
        db: pool.clone(),
        controller: Controller::new(pool.clone(), broadcast_sender.clone()),
    });

    let app = Router::new()
        .route("/login", post(login::login))
        .route("/websocket", get(websocket::websocket_handler))
        .with_state(app_state)
        .layer(CorsLayer::permissive());

    let listener = TcpListener::bind("127.0.0.1:3001").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    serve(listener, app).await.unwrap();
}

// 3. Refactor <-
// 4. Remove unwrap()
// 5. Add elasticsearch over messages
// 6. Add Several chats for user
// 7. Add tests
