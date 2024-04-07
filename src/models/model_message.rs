use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::PostgresResult;

/* History message structure of a Response event */
// TODO: Doesn't belong to Model
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryMessage {
    user_id: Uuid,
    username: String,
    content: String,
    #[serde(rename = "timestamp")]
    created_at: DateTime<Utc>,
}

impl Default for HistoryMessage {
    fn default() -> Self {
        HistoryMessage {
            user_id: Uuid::nil(),
            username: String::from(""),
            content: String::from(""),
            created_at: Utc::now(),
        }
    }
}

/* Message structure in a Messages table */
pub struct ModelMessage {
    id: Uuid,
    chat_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl Default for ModelMessage {
    fn default() -> Self {
        ModelMessage {
            id: Uuid::nil(),
            chat_id: Uuid::nil(),
            user_id: Uuid::nil(),
            content: String::from(""),
            created_at: Utc::now(),
        }
    }
}

impl ModelMessage {
    pub async fn new(
        pool: &PgPool,
        chat_id: Uuid,
        user_id: Uuid,
        content: String,
        created_at: DateTime<Utc>,
    ) -> PostgresResult<ModelMessage> {
        Ok(
            sqlx::query_as!(
            ModelMessage,
            "INSERT INTO messages (id, chat_id, user_id, content, created_at) VALUES (gen_random_uuid(), $1, $2, $3, $4) RETURNING *",
            chat_id,
            user_id,
            content,
            created_at
        ).fetch_one(pool)
        .await?)

        // Ok(ModelMessage::default())
    }

    pub async fn get_chat_history(
        pool: &PgPool,
        chat_id: Uuid,
    ) -> PostgresResult<Vec<HistoryMessage>> {
        Ok(sqlx::query_as!(
            HistoryMessage,
            "SELECT users.username, users.id as user_id, messages.content, messages.created_at FROM messages
            INNER JOIN users ON messages.user_id = users.id
            WHERE chat_id = $1",
            chat_id
        )
        .fetch_all(pool)
        .await?)

        // Ok(vec![HistoryMessage::default()])
    }
}
