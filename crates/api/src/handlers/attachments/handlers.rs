use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use domain::chat::repository::{ChatRepository, ConfirmAttachmentInput, NewPendingAttachment};
use shared::error::DomainError;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::attachments::dto::{
    ConfirmAttachmentRequest, CreateUploadUrlRequest, CreateUploadUrlResponse,
};
use crate::middleware::auth::AuthenticatedUser;
use crate::services::storage::S3StorageService;
use infrastructure::repositories::chat::PostgresChatRepository;

#[derive(Clone)]
pub struct AttachmentsState {
    pub chat_repo: Arc<PostgresChatRepository>,
    pub storage: Arc<S3StorageService>,
}

pub async fn create_upload_url(
    State(state): State<AttachmentsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Json(req): Json<CreateUploadUrlRequest>,
) -> Result<Response, ApiError> {
    validate_file_constraints(&req.file_type, req.file_size)?;

    let attachment_id = Uuid::new_v4();
    let object_key = format!(
        "attachments/{}/{}/{}",
        req.chat_id,
        attachment_id,
        sanitize_file_name(req.file_name.as_deref())
    );

    state
        .chat_repo
        .get_chat_for_user(auth.user_id, req.chat_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("chat not found or not a participant".to_string()))?;

    let upload_url = state
        .storage
        .presign_put_url(&object_key, &req.file_type, 300)
        .await
        .map_err(ApiError::from)?;

    let file_url = state.storage.bucket_object_url(&object_key);
    state
        .chat_repo
        .create_pending_attachment(NewPendingAttachment {
            attachment_id,
            uploader_id: auth.user_id,
            chat_id: req.chat_id,
            object_key,
            file_url,
            file_type: req.file_type,
            file_size: req.file_size,
            file_name: req.file_name,
        })
        .await?;

    let response = CreateUploadUrlResponse {
        upload_url,
        attachment_id,
        expires_at: Utc::now() + Duration::minutes(5),
    };
    Ok((StatusCode::OK, Json(response)).into_response())
}

pub async fn confirm_attachment(
    State(state): State<AttachmentsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Json(req): Json<ConfirmAttachmentRequest>,
) -> Result<Response, ApiError> {
    let attachment = state
        .chat_repo
        .get_pending_attachment_for_user(req.attachment_id, auth.user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("pending attachment not found".to_string()))?;

    state
        .storage
        .head_object(&attachment.object_key)
        .await
        .map_err(ApiError::from)?;

    state
        .chat_repo
        .confirm_attachment(ConfirmAttachmentInput {
            attachment_id: req.attachment_id,
            message_id: req.message_id,
            uploader_id: auth.user_id,
            encryption_key_enc: req.encryption_key_enc,
            encryption_iv: req.encryption_iv,
        })
        .await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"message": "attachment confirmed"})),
    )
        .into_response())
}

fn sanitize_file_name(file_name: Option<&str>) -> String {
    file_name
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(|name| {
            name.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
        })
        .unwrap_or_else(|| "file.bin".to_string())
}

fn validate_file_constraints(file_type: &str, file_size: i64) -> Result<(), ApiError> {
    if file_size <= 0 {
        return Err(ApiError(DomainError::Validation(
            "file_size must be positive".to_string(),
        )));
    }

    let image_types = ["image/jpeg", "image/png", "image/webp", "image/gif"];
    let video_types = ["video/mp4", "video/quicktime", "video/webm"];
    let audio_types = ["audio/mpeg", "audio/ogg", "audio/aac", "audio/wav"];

    let max_size = if image_types.contains(&file_type) {
        25 * 1024 * 1024
    } else if video_types.contains(&file_type) {
        100 * 1024 * 1024
    } else if audio_types.contains(&file_type) {
        25 * 1024 * 1024
    } else {
        100 * 1024 * 1024
    };

    if file_size > max_size {
        return Err(ApiError(DomainError::Validation(format!(
            "file_size exceeds maximum allowed for mime type: {file_type}"
        ))));
    }

    Ok(())
}
