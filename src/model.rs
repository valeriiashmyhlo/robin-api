use anyhow::Result;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::Login;
use uuid::Uuid;

type PostgresResult<T> = Result<T>;

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]

pub struct User {
    //TODO: Id as uuid
    pub id: i32,
    pub first_name: String,
    pub password: String,
    pub token: Uuid,
    // #[serde(rename = "createdAt")]
    // pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    // #[serde(rename = "updatedAt")]
    // pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl User {
    pub async fn get(pool: &PgPool, login: Login) -> PostgresResult<User> {
        let user = sqlx::query_as!(
            User,
            "SELECT * from users WHERE password = $1 AND first_name = $2",
            login.password,
            login.first_name
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }
}
