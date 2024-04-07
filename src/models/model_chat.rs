use sqlx::PgPool;
use uuid::Uuid;

use super::PostgresResult;

pub struct ModelChat {
    pub id: Uuid,
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

    pub fn get_id() -> PostgresResult<Uuid> {
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
    pub async fn new(pool: &PgPool, chat_id: Uuid, user_id: Uuid) -> PostgresResult<Self> {
        Ok(sqlx::query_as!(
            ModelChatUser,
            "INSERT INTO chat_user (chat_id, user_id) VALUES ($1, $2) RETURNING *",
            chat_id,
            user_id
        )
        .fetch_one(pool)
        .await?)

        // Ok(ModelChatUser::default())
    }
}
