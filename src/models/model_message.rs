use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::PostgresResult;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserMessage {
    user_id: Uuid,
    username: String,
    content: String,
}

pub struct ModelMessage {
    id: Uuid,
    chat_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
}

impl ModelMessage {
    pub async fn new(
        pool: &PgPool,
        chat_id: Uuid,
        user_id: Uuid,
        content: String,
    ) -> PostgresResult<ModelMessage> {
        Ok(sqlx::query_as!(
            ModelMessage,
            "INSERT INTO messages (id, chat_id, user_id, content) VALUES (gen_random_uuid(), $1, $2, $3) RETURNING *",
            chat_id,
            user_id,
            content
        ).fetch_one(pool)
        .await?)
    }

    pub async fn get_chat_history(
        pool: &PgPool,
        chat_id: Uuid,
    ) -> PostgresResult<Vec<UserMessage>> {
        Ok(sqlx::query_as!(
            UserMessage,
            "SELECT users.username, users.id as user_id, messages.content from messages
            INNER JOIN users ON messages.user_id = users.id
            WHERE chat_id = $1",
            chat_id
        )
        .fetch_all(pool)
        .await?)
    }
}
