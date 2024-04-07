use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::{from_str, to_string};
use std::sync::Arc;

use crate::{
    models::{ModelChat, ModelChatUser, ModelMessage, ModelUser},
    AppState, RequestMessage, ResponseMessage, User,
};

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
        user: User::from_model_user(user.clone()),
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
                ModelMessage::new(&db_clone, chat_id, id, content.clone(), Utc::now())
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
        user: User::from_model_user(user),
    });

    // state.user_set.lock().unwrap().remove(&user);
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}
