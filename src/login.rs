use std::sync::Arc;

use axum::{debug_handler, extract::State, response::IntoResponse, Json};
use serde::Deserialize;

use crate::{app_error::AppError, models::ModelUser, AppState, AuthorisedUser};

#[derive(Deserialize, Debug)]
pub struct Login {
    pub username: String,
    pub password: String,
}

#[debug_handler]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(props): Json<Login>,
) -> Result<impl IntoResponse, AppError> {
    let result = ModelUser::get(&state.db, props).await?;

    Ok(Json(AuthorisedUser {
        id: result.id,
        username: result.username,
        token: result.token,
    }))
}
