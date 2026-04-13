use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::error::ApiError;
use crate::handlers::keys::dto::*;
use crate::middleware::auth::AuthenticatedUser;
use domain::keys::{KeyRepository, OneTimePrekey};
use infrastructure::repositories::keys::PostgresKeyRepository;
use shared::error::DomainError;

#[derive(Clone)]
pub struct KeysState {
    pub key_repo: Arc<PostgresKeyRepository>,
}

pub async fn upload_keys(
    State(state): State<KeysState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Json(req): Json<UploadKeysRequest>,
) -> Result<Response, ApiError> {
    if req.identity_key.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "identity_key is required"})),
        )
            .into_response());
    }

    if req.signed_prekey.key.is_empty() || req.signed_prekey.signature.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "signed_prekey is incomplete"})),
        )
            .into_response());
    }

    state
        .key_repo
        .upsert_keys(
            auth.user_id,
            &req.identity_key,
            &domain::keys::SignedPrekey {
                id: req.signed_prekey.id,
                key: req.signed_prekey.key.clone(),
                signature: req.signed_prekey.signature.clone(),
            },
        )
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    if !req.one_time_prekeys.is_empty() {
        let prekeys: Vec<OneTimePrekey> = req
            .one_time_prekeys
            .into_iter()
            .map(|p| OneTimePrekey {
                id: p.id,
                key: p.key,
            })
            .collect();

        state
            .key_repo
            .add_one_time_prekeys(auth.user_id, prekeys)
            .await
            .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;
    }

    let count = state
        .key_repo
        .get_prekey_count(auth.user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    Ok((
        StatusCode::OK,
        Json(UploadKeysResponse {
            prekey_count: count,
        }),
    )
        .into_response())
}

pub async fn upload_prekeys(
    State(state): State<KeysState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Json(req): Json<Vec<OneTimePrekeyUpload>>,
) -> Result<Response, ApiError> {
    if req.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "no prekeys provided"})),
        )
            .into_response());
    }

    let prekeys: Vec<OneTimePrekey> = req
        .into_iter()
        .map(|p| OneTimePrekey {
            id: p.id,
            key: p.key,
        })
        .collect();

    state
        .key_repo
        .add_one_time_prekeys(auth.user_id, prekeys)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    let count = state
        .key_repo
        .get_prekey_count(auth.user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    Ok((
        StatusCode::OK,
        Json(UploadKeysResponse {
            prekey_count: count,
        }),
    )
        .into_response())
}

pub async fn get_key_bundle(
    State(state): State<KeysState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(target_user_id): Path<uuid::Uuid>,
) -> Result<Response, ApiError> {
    if target_user_id == auth.user_id {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "cannot get own key bundle"})),
        )
            .into_response());
    }

    let blocked = state
        .key_repo
        .is_blocked(auth.user_id, target_user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    if blocked {
        return Ok((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "user is blocked"})),
        )
            .into_response());
    }

    let blocked_by_target = state
        .key_repo
        .is_blocked(target_user_id, auth.user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    if blocked_by_target {
        return Ok((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "you are blocked by this user"})),
        )
            .into_response());
    }

    let keys = state
        .key_repo
        .get_keys(target_user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    let keys =
        keys.ok_or_else(|| ApiError(DomainError::NotFound("user keys not found".to_string())))?;

    let count = state
        .key_repo
        .get_prekey_count(target_user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    if count < 20 {
        tracing::warn!("User {} has low prekey count: {}", target_user_id, count);
    }

    Ok(Json(KeyBundleResponse {
        identity_key: keys.identity_key,
        signed_prekey: SignedPrekeyResponse {
            id: keys.signed_prekey.id,
            key: keys.signed_prekey.key,
            signature: keys.signed_prekey.signature,
        },
        one_time_prekey: None,
    })
    .into_response())
}

pub async fn get_fingerprint(
    State(state): State<KeysState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(target_user_id): Path<uuid::Uuid>,
) -> Result<Response, ApiError> {
    let your_keys = state
        .key_repo
        .get_keys(auth.user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?
        .ok_or_else(|| ApiError(DomainError::NotFound("your keys not found".to_string())))?;

    let their_keys = state
        .key_repo
        .get_keys(target_user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?
        .ok_or_else(|| {
            ApiError(DomainError::NotFound(
                "target user keys not found".to_string(),
            ))
        })?;

    let (fingerprint, your_key, their_key) =
        compute_fingerprint(&your_keys.identity_key, &their_keys.identity_key);

    Ok(Json(FingerprintResponse {
        fingerprint,
        your_key,
        their_key,
        key_changed: None,
        changed_at: None,
    })
    .into_response())
}

pub async fn get_my_prekey_count(
    State(state): State<KeysState>,
    Extension(auth): Extension<AuthenticatedUser>,
) -> Result<Response, ApiError> {
    let count = state
        .key_repo
        .get_prekey_count(auth.user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    Ok(Json(PrekeyCountResponse { count }).into_response())
}

fn compute_fingerprint(key1: &str, key2: &str) -> (String, String, String) {
    use sha2::{Digest, Sha256};

    let (min_key, max_key) = if key1 < key2 {
        (key1, key2)
    } else {
        (key2, key1)
    };

    let mut hasher = Sha256::new();
    hasher.update(min_key.as_bytes());
    hasher.update(max_key.as_bytes());
    let result = hasher.finalize();

    let fingerprint = format!("{:x}", result);
    let fingerprint = format!("{} {}", &fingerprint[..30], &fingerprint[30..60]);

    (fingerprint, key1.to_string(), key2.to_string())
}
