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
use serde_json::{from_str, to_string};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::cors::CorsLayer;
use tracing::log::{set_max_level, LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::{
    app_error::AppError,
    models::{ModelChat, ModelChatUser, ModelMessage, ModelUser, UserMessage},
};

mod app_error;
mod db;
mod models;

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
        user_id: Uuid,
        username: String,
    },
    Leave {
        user_id: Uuid,
        username: String,
    },
    Message {
        username: String,
        content: String,
    },
    History {
        messages: Vec<UserMessage>,
        users: Vec<User>,
    },
}

pub struct AppState {
    // user_set: Arc<Mutex<HashSet<User>>>,
    broadcast_sender: broadcast::Sender<ResponseMessage>,
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

    // let user_set = Arc::new(Mutex::new(HashSet::<User>::new()));
    let (broadcast_sender, _broadcast_receiver) = broadcast::channel(100);
    let db = pool.clone();

    let app_state = Arc::new(AppState {
        broadcast_sender,
        db,
    });

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
    pub username: String,
    pub password: String,
}

#[debug_handler]
async fn login(
    State(state): State<Arc<AppState>>,
    Json(props): Json<Login>,
) -> Result<impl IntoResponse, AppError> {
    let result = ModelUser::get(&state.db, props).await?;

    Ok(Json(AuthorisedUser {
        id: result.id,
        username: result.username,
        token: result.token,
    }))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(ws: WebSocket, state: Arc<AppState>) {
    // Client specific channel
    let (mut ws_sender, mut ws_receiver) = ws.split();

    // Broadcast channel
    let mut broadcast_receiver = state.broadcast_sender.subscribe();

    let Some(Ok(Message::Text(text))) = ws_receiver.next().await else {
        panic!("Invalid message");
    };
    let token = match async { from_str(&text) }.await {
        Ok(RequestMessage::Join { token }) => token,
        _ => {
            panic!("Invalid message2");
        }
    };
    let user = ModelUser::get_by_token(&state.db, token).await.unwrap();
    tracing::warn!("{} joined", user.username);
    // Send message to all users that a new user has joined
    let _ = state.broadcast_sender.send(ResponseMessage::Join {
        user_id: user.id,
        username: user.username.clone(),
    });

    let chat_id = ModelChat::get_id().unwrap();
    ModelChatUser::new(&state.db, chat_id, user.id)
        .await
        .unwrap();
    let connected_users = ModelUser::get_users_in_chat(&state.db, chat_id)
        .await
        .unwrap()
        .into_iter()
        .map(User::from_model_user)
        .collect::<Vec<User>>();

    // Send a history of a chat to a newly joined user
    let chat_history = ModelMessage::get_chat_history(&state.db, chat_id)
        .await
        .unwrap();

    ws_sender
        .send(Message::Text(
            to_string(&ResponseMessage::History {
                messages: chat_history,
                users: connected_users,
            })
            .unwrap(),
        ))
        .await
        .unwrap();

    // Forward messages from broadcast(global) to client specific channel
    let mut send_task = tokio::spawn(async move {
        while let msg = broadcast_receiver.recv().await {
            if ws_sender
                .send(Message::Text(to_string(&msg.unwrap()).unwrap()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Receive message from a user and broadcast it to all users
    let broadcast_sender_clone = state.broadcast_sender.clone();
    let db_clone = state.db.clone();
    let id = user.id;
    let username = user.username.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = ws_receiver.next().await {
            if let Ok(RequestMessage::Message { content }) = from_str(&text) {
                ModelMessage::new(&db_clone, chat_id, id, content.clone())
                    .await
                    .unwrap();
                broadcast_sender_clone
                    .send(ResponseMessage::Message {
                        username: username.clone(),
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

    ModelUser::remove_user_from_chat(&state.db, user.id)
        .await
        .unwrap();

    tracing::warn!("left {}", user.username);
    let _ = state.broadcast_sender.send(ResponseMessage::Leave {
        user_id: user.id,
        username: user.username.clone(),
    });

    // state.user_set.lock().unwrap().remove(&user);
}

// 1. Add a history of messages after joined
// 2. Add a list of connected users
// 3. Refactor
// 4. Add elasticsearch over messages
// 5. Remove unwrap()
// 6. Handle state Mutex properly
