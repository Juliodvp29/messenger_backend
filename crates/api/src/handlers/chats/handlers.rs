use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use domain::chat::entity::{Chat, ChatPreview};
use domain::chat::repository::{ChatCursor, ChatRepository};
use shared::error::DomainError;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::chats::dto::{
    ChatCursorDto, ChatPreviewResponse, ChatResponse, CreateChatRequest, ListChatsQuery,
    ListChatsResponse,
};
use crate::middleware::auth::AuthenticatedUser;
use infrastructure::repositories::chat::PostgresChatRepository;

#[derive(Clone)]
pub struct ChatsState {
    pub chat_repo: Arc<PostgresChatRepository>,
}

pub async fn create_chat(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Json(req): Json<CreateChatRequest>,
) -> Result<Response, ApiError> {
    let chat = match req {
        CreateChatRequest::Private { participant_id } => {
            if participant_id == auth.user_id {
                return Err(ApiError(DomainError::Validation(
                    "participant_id cannot be the authenticated user".to_string(),
                )));
            }
            state
                .chat_repo
                .create_private_chat(auth.user_id, participant_id)
                .await?
        }
        CreateChatRequest::Group {
            name,
            participant_ids,
        } => {
            if name.trim().is_empty() {
                return Err(ApiError(DomainError::Validation(
                    "group name cannot be empty".to_string(),
                )));
            }
            state
                .chat_repo
                .create_group_chat(auth.user_id, name.trim(), &participant_ids)
                .await?
        }
    };

    Ok((StatusCode::CREATED, Json(chat_to_response(chat))).into_response())
}

pub async fn get_chat(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(chat_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let chat = state
        .chat_repo
        .get_chat_for_user(auth.user_id, chat_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("chat not found".to_string()))?;

    Ok((StatusCode::OK, Json(chat_to_response(chat))).into_response())
}

pub async fn list_chats(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Query(query): Query<ListChatsQuery>,
) -> Result<Response, ApiError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let decoded_cursor = decode_cursor(query.cursor.as_deref())?;
    let fetch_limit = limit + 1;

    let mut items = state
        .chat_repo
        .list_chats_for_user(auth.user_id, decoded_cursor, fetch_limit)
        .await?;

    let has_more = items.len() as i64 > limit;
    if has_more {
        items.truncate(limit as usize);
    }

    let next_cursor = items.last().map(build_cursor).transpose()?;

    let response = ListChatsResponse {
        items: items.into_iter().map(preview_to_response).collect(),
        next_cursor,
        has_more,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

fn decode_cursor(raw: Option<&str>) -> Result<Option<ChatCursor>, ApiError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let decoded = URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| DomainError::Validation("invalid cursor encoding".to_string()))?;
    let dto: ChatCursorDto = serde_json::from_slice(&decoded)
        .map_err(|_| DomainError::Validation("invalid cursor payload".to_string()))?;

    Ok(Some(ChatCursor {
        is_pinned: dto.is_pinned,
        last_message_at: dto.last_message_at,
        chat_id: dto.chat_id,
    }))
}

fn build_cursor(item: &ChatPreview) -> Result<String, ApiError> {
    let dto = ChatCursorDto {
        is_pinned: item.is_pinned,
        last_message_at: item.last_message_at,
        chat_id: item.chat_id,
    };
    let encoded = serde_json::to_vec(&dto)
        .map_err(|e| DomainError::Internal(format!("failed to serialize cursor: {e}")))?;
    Ok(URL_SAFE_NO_PAD.encode(encoded))
}

fn chat_to_response(chat: Chat) -> ChatResponse {
    ChatResponse {
        id: chat.id,
        chat_type: chat.chat_type.as_db_str().to_string(),
        name: chat.name,
        avatar_url: chat.avatar_url,
        created_by: chat.created_by,
        created_at: chat.created_at,
    }
}

fn preview_to_response(preview: ChatPreview) -> ChatPreviewResponse {
    ChatPreviewResponse {
        chat_id: preview.chat_id,
        chat_type: preview.chat_type.as_db_str().to_string(),
        name: preview.name,
        avatar_url: preview.avatar_url,
        last_message_id: preview.last_message_id,
        last_message_encrypted: preview.last_message_encrypted,
        last_sender_id: preview.last_sender_id,
        last_message_at: preview.last_message_at,
        is_pinned: preview.is_pinned,
        pin_order: preview.pin_order,
        is_muted: preview.is_muted,
        is_archived: preview.is_archived,
        unread_count: preview.unread_count,
    }
}
