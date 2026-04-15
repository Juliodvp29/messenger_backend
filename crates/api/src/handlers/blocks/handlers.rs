use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::auth::AuthenticatedUser;
use infrastructure::repositories::user::PostgresUserRepository;
use shared::error::DomainError;

use super::dto::{BlockListResponse, BlockResponse};

#[derive(Clone)]
pub struct BlocksState {
    pub user_repo: Arc<PostgresUserRepository>,
}

pub async fn block_user(
    State(state): State<BlocksState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(user_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    if user_id == auth.user_id {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Cannot block yourself" })),
        )
            .into_response());
    }

    let existing = state
        .user_repo
        .is_user_blocked(&auth.user_id, &user_id)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    if existing {
        return Ok((
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "User already blocked" })),
        )
            .into_response());
    }

    let block_id = state.user_repo.block_user(&auth.user_id, &user_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(BlockResponse {
            id: block_id.to_string(),
            blocked_id: user_id.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }),
    )
        .into_response())
}

pub async fn unblock_user(
    State(state): State<BlocksState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(user_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let deleted = state
        .user_repo
        .unblock_user(&auth.user_id, &user_id)
        .await?;

    if !deleted {
        return Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Block not found" })),
        )
            .into_response());
    }

    Ok((StatusCode::NO_CONTENT, Json(serde_json::json!({}))).into_response())
}

pub async fn list_blocked(
    State(state): State<BlocksState>,
    Extension(auth): Extension<AuthenticatedUser>,
) -> Result<Response, ApiError> {
    let blocked = state.user_repo.list_blocked_users(&auth.user_id).await?;

    let response: Vec<BlockResponse> = blocked
        .into_iter()
        .map(|(id, blocked_id)| BlockResponse {
            id: id.to_string(),
            blocked_id: blocked_id.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
        .collect();

    Ok((StatusCode::OK, Json(BlockListResponse { blocks: response })).into_response())
}
