use anyhow::Error;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use futures::SinkExt;
use serde_json::{from_str, to_string};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::{sync::broadcast::Sender, task::JoinHandle};
use uuid::Uuid;

use crate::{
    app_error::AppError,
    models::{ModelChat, ModelChatUser, ModelMessage, ModelUser},
    AppState, RequestMessage, ResponseMessage, User,
};

async fn websocket(ws: WebSocket, state: Arc<AppState>) {
    websocket_result(ws, state).await.unwrap()
}

async fn websocket_result(ws: WebSocket, state: Arc<AppState>) -> Result<(), AppError> {
    // Client specific channel
    let (sender, receiver) = ws.split();
    let mut client_receiver = ClientReceiver::new(receiver).await;
    let mut client_sender = ClientSender::new(sender).await;

    // Broadcast channel
    let mut broadcast_receiver = state.broadcast_sender.subscribe();

    let RequestMessage::Join { token } = client_receiver.next().await? else {
        return Err(AppError::from(Error::msg("Invalid message")));
    };

    let user = ModelUser::get_by_token(&state.db, token).await?;
    tracing::warn!("{} joined", user.username);

    let chat_id = ModelChat::get_id()?;
    state
        .controller
        .join_user(chat_id, User::from_model_user(user.clone()))
        .await?;

    let connected_users = ModelUser::get_users_in_chat(&state.db, chat_id)
        .await?
        .into_iter()
        .map(User::from_model_user)
        .collect::<Vec<_>>();

    // Send a history of a chat to a newly joined user
    let chat_history = ModelMessage::get_chat_history(&state.db, chat_id).await?;

    client_sender
        .send(ResponseMessage::History {
            messages: chat_history,
            users: connected_users,
        })
        .await?;

    // Forward messages from broadcast(global) to client specific channel
    let mut send_task: JoinHandle<Result<(), AppError>> = tokio::spawn(async move {
        loop {
            let msg = broadcast_receiver.recv().await?;
            client_sender.send(msg).await?;
        }
    });

    // Receive message from a user and broadcast it to all users
    let state_clone = state.clone();
    let user_id = user.id;
    let username = user.username.clone();
    let mut recv_task: JoinHandle<Result<(), AppError>> = tokio::spawn(async move {
        while let RequestMessage::Message { content } = client_receiver.next().await? {
            state_clone
                .controller
                .send_message(chat_id, user_id, username.clone(), content.clone())
                .await?;
        }
        Ok(())
    });

    /* If receiver task finishes - abort sender task and vice versa */
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    state
        .controller
        .remove_user(chat_id, User::from_model_user(user.clone()))
        .await?;

    tracing::warn!("left {}", user.username);

    Ok(())
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

struct ClientSender {
    sender: SplitSink<WebSocket, Message>,
}

impl ClientSender {
    async fn new(sender: SplitSink<WebSocket, Message>) -> Self {
        Self { sender }
    }

    async fn send(&mut self, message: ResponseMessage) -> Result<(), AppError> {
        self.sender
            .send(Message::Text(to_string(&message)?))
            .await?;
        Ok(())
    }
}

struct ClientReceiver {
    receiver: SplitStream<WebSocket>,
}

impl ClientReceiver {
    async fn new(receiver: SplitStream<WebSocket>) -> Self {
        Self { receiver }
    }

    async fn next(&mut self) -> Result<RequestMessage, AppError> {
        let receive_event = self.receiver.next().await;
        match receive_event {
            Some(Ok(Message::Text(text))) => Ok(from_str(&text)?),
            _ => Err(AppError::from(Error::msg("Invalid message"))),
        }
    }
}

pub struct Controller {
    db: Pool<Postgres>,
    broadcast_sender: Sender<ResponseMessage>,
}

impl Controller {
    pub fn new(db: Pool<Postgres>, broadcast_sender: Sender<ResponseMessage>) -> Self {
        Self {
            db,
            broadcast_sender,
        }
    }

    async fn join_user(&self, chat_id: Uuid, user: User) -> Result<(), AppError> {
        ModelChatUser::create(&self.db, chat_id, user.id).await?;

        // Send message to all users that a new user has joined
        self.broadcast_sender.send(ResponseMessage::Join { user })?;

        Ok(())
    }

    async fn remove_user(&self, chat_id: Uuid, user: User) -> Result<(), AppError> {
        ModelChatUser::delete(&self.db, chat_id, user.id).await?;
        self.broadcast_sender
            .send(ResponseMessage::Leave { user })?;

        Ok(())
    }

    async fn send_message(
        &self,
        chat_id: Uuid,
        id: Uuid,
        username: String,
        content: String,
    ) -> Result<(), AppError> {
        ModelMessage::create(&self.db, chat_id, id, content.clone(), Utc::now()).await?;

        self.broadcast_sender.send(ResponseMessage::Message {
            username: username.clone(),
            content,
        })?;

        Ok(())
    }
}

// 1. Wrapper for sender and receiver to use RequestMessage
