use sqlx::PgPool;
use uuid::Uuid;

use super::PostgresResult;

pub struct ModelChat {
    id: Uuid,
}

impl ModelChat {
    // pub fn new(pool: &PgPool) -> Self {
    //     sqlx::query_as!(
    //         ModelChat,
    //         "INSERT INTO chats (id) VALUES (gen_random_uuid()) RETURNING *",
    //     )
    // }

    pub fn get_id() -> PostgresResult<Uuid> {
        // self.id
        Ok(Uuid::parse_str("d58535ec-fe54-4d30-9808-94af7d6dc1bf")?)
    }
}

pub struct ModelChatUser {
    pub chat_id: Uuid,
    pub user_id: Uuid,
}

impl ModelChatUser {
    pub async fn new(pool: &PgPool, chat_id: Uuid, user_id: Uuid) -> PostgresResult<ModelChatUser> {
        Ok(sqlx::query_as!(
            ModelChatUser,
            "INSERT INTO chat_user (chat_id, user_id) VALUES ($1, $2) RETURNING *",
            chat_id,
            user_id
        )
        .fetch_one(pool)
        .await?)
    }
}
