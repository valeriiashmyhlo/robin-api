use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use uuid::Uuid;

use crate::login::Login;

use super::PostgresResult;

#[derive(Debug, FromRow, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[allow(non_snake_case)]
pub struct ModelUser {
    pub id: Uuid,
    pub username: String,
    pub password: String,
    pub token: Uuid,
    // #[serde(deserialize_with = "time::serde::deserialize")]
    // pub created_at: OffsetDateTime,
    // #[serde(rename = "updatedAt")]
    // pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for ModelUser {
    fn default() -> Self {
        ModelUser {
            id: Uuid::nil(),
            username: String::from(""),
            password: String::from(""),
            token: Uuid::nil(),
        }
    }
}

impl ModelUser {
    pub async fn get(pool: &PgPool, login: Login) -> PostgresResult<ModelUser> {
        let user = sqlx::query_as!(
            ModelUser,
            "SELECT * FROM users WHERE password = $1 AND username = $2",
            login.password,
            login.username
        )
        .fetch_one(pool)
        .await?;

        // let user = ModelUser::default();

        Ok(user)
    }

    pub async fn get_by_token(pool: &PgPool, token: Uuid) -> PostgresResult<ModelUser> {
        let user = sqlx::query_as!(ModelUser, "SELECT * FROM users WHERE token = $1", token,)
            .fetch_one(pool)
            .await?;

        // let user = ModelUser::default();

        Ok(user)
    }

    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> PostgresResult<ModelUser> {
        let user = sqlx::query_as!(ModelUser, "SELECT * FROM users WHERE id = $1", id,)
            .fetch_one(pool)
            .await?;

        // let user = ModelUser::default();

        Ok(user)
    }

    pub async fn get_users_in_chat(pool: &PgPool, chat_id: Uuid) -> PostgresResult<Vec<ModelUser>> {
        Ok(sqlx::query_as!(
            ModelUser,
            "SELECT users.* FROM users
                JOIN chat_user AS cu
                ON cu.user_id = users.id
                WHERE cu.chat_id = $1",
            chat_id
        )
        .fetch_all(pool)
        .await?)

        // Ok(vec![ModelUser::default()])
    }

    pub async fn remove_user_from_chat(pool: &PgPool, user_id: Uuid) -> PostgresResult<()> {
        sqlx::query!("DELETE FROM chat_user WHERE user_id = $1", user_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    // pub async fn get_users_in_chat(pool: &PgPool, chat_id: Uuid) -> PostgresResult<Vec<ModelUser>> {
    //     Ok(sqlx::query_as!(
    //         ModelUser,
    //         "SELECT * from users WHERE id IN (SELECT user_id from chat_users WHERE chat_id = $1)",
    //         chat_id
    //     )
    //     .fetch_all(pool)
    //     .await?)
    // }
}
