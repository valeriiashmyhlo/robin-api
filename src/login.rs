use std::sync::Arc;

use anyhow::Context;
use axum::{
    debug_handler,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{models::ModelUser, AppState, AuthorisedUser};

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    // #[error("{0}")]
    // ValidationError(String),
    #[error(transparent)]
    NotFoundError(#[from] sqlx::Error),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFoundError(e) => {
                tracing::error!("{}", e);
                match e {
                    sqlx::Error::RowNotFound => StatusCode::UNAUTHORIZED.into_response(),
                    _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[debug_handler]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(props): Json<Login>,
) -> Result<Json<AuthorisedUser>, LoginError> {
    let result = ModelUser::get(&state.db, props).await?;

    Ok(Json(AuthorisedUser {
        id: result.id,
        username: result.username,
        token: result.token,
    }))
}
