use sqlx::PgPool;
use uuid::Uuid;

use super::{DatabaseResult, PostgresResult};

pub struct ModelChat {
    pub id: Uuid,
}

#[derive(thiserror::Error, Debug)]
pub enum ChatError {
    #[error("Chat not found")]
    ChatNotFound,
}

impl From<uuid::Error> for ChatError {
    fn from(_: uuid::Error) -> Self {
        ChatError::ChatNotFound
    }
}

impl ModelChat {
    pub async fn new(pool: &PgPool) -> PostgresResult<Self> {
        let new_uuid = Uuid::new_v4();
        Ok(sqlx::query_as!(
            ModelChat,
            "INSERT INTO chats (id) VALUES ($1) RETURNING *",
            new_uuid
        )
        .fetch_one(pool)
        .await?)
    }

    pub fn get_id() -> Result<Uuid, ChatError> {
        // 1. Handle chart creation
        // 2. Handle getting chat id
        Ok(Uuid::parse_str("d58535ec-fe54-4d30-9808-94af7d6dc1bf")?)
    }
}

pub struct ModelChatUser {
    pub chat_id: Uuid,
    pub user_id: Uuid,
}

impl Default for ModelChatUser {
    fn default() -> Self {
        ModelChatUser {
            chat_id: Uuid::nil(),
            user_id: Uuid::nil(),
        }
    }
}

impl ModelChatUser {
    pub async fn create(pool: &PgPool, chat_id: Uuid, user_id: Uuid) -> DatabaseResult<Self> {
        sqlx::query_as!(
            ModelChatUser,
            "INSERT INTO chat_user (chat_id, user_id) VALUES ($1, $2) RETURNING *",
            chat_id,
            user_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, chat_id: Uuid, user_id: Uuid) -> DatabaseResult<()> {
        sqlx::query!(
            "DELETE FROM chat_user WHERE chat_id = $1 AND user_id = $2",
            chat_id,
            user_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
