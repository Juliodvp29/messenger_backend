use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::auth::AuthenticatedUser;
use crate::services::otp::OtpService;
use domain::user::value_objects::UserId;
use infrastructure::repositories::user::PostgresUserRepository;
use shared::error::DomainError;

use super::dto::{SearchQuery, UserProfileResponse, UserSearchResult};

#[derive(Clone)]
pub struct UsersState {
    pub user_repo: Arc<PostgresUserRepository>,
    pub otp_service: Arc<OtpService>,
}

pub async fn search_users(
    State(state): State<UsersState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Query(query): Query<SearchQuery>,
) -> Result<Response, ApiError> {
    let limit = query.limit.unwrap_or(20).min(50);

    let search_term = match query.q {
        Some(q) if !q.trim().is_empty() => q,
        _ => {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Query parameter 'q' is required" })),
            )
                .into_response());
        }
    };

    let rate_key = format!("search:{}", auth.user_id);
    let allowed = state
        .otp_service
        .check_rate_limit(&rate_key, 30, 60)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    if !allowed {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": "Too many search requests" })),
        )
            .into_response());
    }

    let results = state
        .user_repo
        .search_users(&search_term, limit, auth.user_id)
        .await?;

    let response: Vec<UserSearchResult> = results
        .into_iter()
        .map(
            |(id, username, display_name, avatar_url)| UserSearchResult {
                id: id.to_string(),
                username,
                display_name,
                avatar_url,
            },
        )
        .collect();

    Ok((StatusCode::OK, Json(response)).into_response())
}

pub async fn get_user_profile(
    State(state): State<UsersState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(user_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let target_user_id = UserId(user_id);

    let is_blocked = state
        .user_repo
        .is_user_blocked(&auth.user_id, &user_id)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    if is_blocked {
        return Err(DomainError::NotFound("User not found".to_string()).into());
    }

    let is_blocking = state
        .user_repo
        .is_user_blocked(&user_id, &auth.user_id)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

    if is_blocking {
        return Err(DomainError::NotFound("User not found".to_string()).into());
    }

    let profile = state
        .user_repo
        .find_profile_by_id(&user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("User not found".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(UserProfileResponse {
            id: profile.0.to_string(),
            username: profile.1,
            display_name: profile.2,
            bio: profile.3,
            avatar_url: profile.4,
            status_text: profile.5,
        }),
    )
        .into_response())
}

pub async fn get_my_profile(
    State(state): State<UsersState>,
    Extension(auth): Extension<AuthenticatedUser>,
) -> Result<Response, ApiError> {
    let profile = state
        .user_repo
        .find_profile_by_id(&auth.user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("User not found".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(UserProfileResponse {
            id: profile.0.to_string(),
            username: profile.1,
            display_name: profile.2,
            bio: profile.3,
            avatar_url: profile.4,
            status_text: profile.5,
        }),
    )
        .into_response())
}
