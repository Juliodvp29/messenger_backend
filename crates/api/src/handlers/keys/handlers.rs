use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
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

    if let Err(e) = verify_signed_prekey(
        &req.identity_key,
        &req.signed_prekey.key,
        &req.signed_prekey.signature,
    ) {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("invalid signed_prekey signature: {}", e)})),
        )
            .into_response());
    }

    if !req.one_time_prekeys.is_empty() {
        if let Err(e) = validate_one_time_prekeys(&req.one_time_prekeys) {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid one_time_prekeys: {}", e)})),
            )
                .into_response());
        }

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

    let mut response = (
        StatusCode::OK,
        Json(UploadKeysResponse {
            prekey_count: count,
        }),
    );

    if count < 20 {
        response.0 = StatusCode::OK;
    }

    Ok(with_prekey_header(response, count).into_response())
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

    let response = (
        StatusCode::OK,
        Json(UploadKeysResponse {
            prekey_count: count,
        }),
    );

    Ok(with_prekey_header(response, count).into_response())
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

    let bundle = state
        .key_repo
        .get_public_key_bundle(target_user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    let bundle =
        bundle.ok_or_else(|| ApiError(DomainError::NotFound("user keys not found".to_string())))?;

    let count = state
        .key_repo
        .get_prekey_count(target_user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    if count < 20 {
        tracing::warn!("User {} has low prekey count: {}", target_user_id, count);
    }

    let response = Json(KeyBundleResponse {
        identity_key: bundle.identity_key,
        signed_prekey: SignedPrekeyResponse {
            id: bundle.signed_prekey_id,
            key: bundle.signed_prekey,
            signature: bundle.signed_prekey_sig,
        },
        one_time_prekey: bundle.one_time_prekey.map(|key| OneTimePrekeyResponse {
            id: bundle.one_time_prekey_id.unwrap_or(0),
            key,
        }),
    });

    Ok(with_prekey_header((StatusCode::OK, response), count).into_response())
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

    let count = state
        .key_repo
        .get_prekey_count(auth.user_id)
        .await
        .map_err(|e| ApiError(DomainError::Internal(e.to_string())))?;

    let response = Json(FingerprintResponse {
        fingerprint,
        your_key,
        their_key,
        key_changed: None,
        changed_at: None,
    });

    Ok(with_prekey_header((StatusCode::OK, response), count).into_response())
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

    let response = Json(PrekeyCountResponse { count });

    Ok(with_prekey_header((StatusCode::OK, response), count).into_response())
}

fn with_prekey_header(response: impl IntoResponse, prekey_count: i32) -> impl IntoResponse {
    if prekey_count < 20 {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::HeaderName::from_static("x-prekey-count"),
            prekey_count.to_string().parse().unwrap(),
        );
        (headers, response)
    } else {
        let headers = HeaderMap::new();
        (headers, response)
    }
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

fn verify_signed_prekey(
    identity_key_b64: &str,
    signed_prekey_b64: &str,
    signature_b64: &str,
) -> Result<(), String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let identity_key_bytes = BASE64
        .decode(identity_key_b64)
        .map_err(|e| format!("invalid identity_key base64: {}", e))?;

    if identity_key_bytes.len() != 32 {
        return Err("identity_key must be 32 bytes".to_string());
    }

    let identity_key = VerifyingKey::from_bytes(
        identity_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "invalid identity_key length")?,
    )
    .map_err(|e| format!("invalid identity_key: {}", e))?;

    let spk_bytes = BASE64
        .decode(signed_prekey_b64)
        .map_err(|e| format!("invalid signed_prekey base64: {}", e))?;

    if spk_bytes.len() != 32 {
        return Err("signed_prekey must be 32 bytes".to_string());
    }

    let sig_bytes = BASE64
        .decode(signature_b64)
        .map_err(|e| format!("invalid signature base64: {}", e))?;

    if sig_bytes.len() != 64 {
        return Err("signature must be 64 bytes".to_string());
    }

    let signature = Signature::from_bytes(
        sig_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "invalid signature length")?,
    );

    identity_key
        .verify(spk_bytes.as_slice(), &signature)
        .map_err(|e| format!("signature verification failed: {}", e))?;

    Ok(())
}

fn validate_one_time_prekeys(prekeys: &[OneTimePrekeyUpload]) -> Result<(), String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    if prekeys.len() > 200 {
        return Err("maximum 200 one-time prekeys allowed".to_string());
    }

    let mut seen_ids = std::collections::HashSet::new();

    for prekey in prekeys {
        if prekey.id < 0 {
            return Err(format!(
                "invalid prekey id: {} (must be positive)",
                prekey.id
            ));
        }

        if !seen_ids.insert(prekey.id) {
            return Err(format!("duplicate prekey id: {}", prekey.id));
        }

        let key_bytes = BASE64
            .decode(&prekey.key)
            .map_err(|e| format!("invalid prekey key base64: {}", e))?;

        if key_bytes.len() != 32 {
            return Err(format!("prekey {} must be 32 bytes", prekey.id));
        }
    }

    Ok(())
}
