use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use domain::chat::entity::ParticipantDetail;
use domain::chat::repository::ChatRepository;
use redis::AsyncCommands;
use shared::error::DomainError;

use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::chats::handlers::ChatsState;
use crate::handlers::groups::dto::{
    AddParticipantRequest, AddParticipantResponse, DeleteInviteLinkResponse, InviteLinkResponse,
    JoinGroupResponse, KeyEntry, ListParticipantsResponse, ParticipantDetailResponse,
    RemoveParticipantResponse, RotateKeyRequest, RotateKeyResponse, TransferOwnershipRequest,
    TransferOwnershipResponse, UpdateRoleRequest, UpdateRoleResponse,
};
use crate::middleware::auth::AuthenticatedUser;

// ---- Helpers ----------------------------------------------------------------

fn participant_to_response(p: ParticipantDetail) -> ParticipantDetailResponse {
    ParticipantDetailResponse {
        user_id: p.user_id,
        chat_id: p.chat_id,
        role: p.role.as_db_str().to_string(),
        encryption_key_enc: p.encryption_key_enc,
        added_by: p.added_by,
        joined_at: p.joined_at,
    }
}

async fn publish_group_event(
    redis: &redis::aio::ConnectionManager,
    chat_id: Uuid,
    event_type: &str,
    extra: serde_json::Value,
) {
    let mut conn = redis.clone();
    let channel = format!("chat:{}:events", chat_id);
    let payload = serde_json::json!({
        "type": event_type,
        "chat_id": chat_id,
        "payload": extra,
    })
    .to_string();
    let _: Result<i64, _> = conn.publish(channel, payload).await;
}

// ---- Handlers ---------------------------------------------------------------

/// GET /chats/:id/participants
pub async fn list_participants(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(chat_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let participants = state
        .chat_repo
        .get_participants_detail(auth.user_id, chat_id)
        .await?;

    let count = participants.len();
    let response = ListParticipantsResponse {
        participants: participants
            .into_iter()
            .map(participant_to_response)
            .collect(),
        count,
    };
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /chats/:id/participants
pub async fn add_participant(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(chat_id): Path<Uuid>,
    Json(req): Json<AddParticipantRequest>,
) -> Result<Response, ApiError> {
    if req.user_id == auth.user_id {
        return Err(ApiError(DomainError::Validation(
            "cannot add yourself as a participant".to_string(),
        )));
    }

    let key_rotation_required = req.encryption_key_enc.is_none();

    let participant = state
        .chat_repo
        .add_participant(chat_id, auth.user_id, req.user_id, req.encryption_key_enc)
        .await?;

    // Notify all participants
    publish_group_event(
        &state.redis,
        chat_id,
        "participant_added",
        serde_json::json!({ "user_id": participant.user_id }),
    )
    .await;

    // Also notify the new member directly
    {
        let mut conn = state.redis.clone();
        let user_channel = format!("user:{}:events", participant.user_id);
        let payload = serde_json::json!({
            "type": "added_to_group",
            "payload": { "chat_id": chat_id },
        })
        .to_string();
        let _: Result<i64, _> = conn.publish(user_channel, payload).await;
    }

    let response = AddParticipantResponse {
        key_rotation_required,
        participant: participant_to_response(participant),
    };
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// DELETE /chats/:id/participants/:user_id
pub async fn remove_participant(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path((chat_id, target_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    let key_rotation_required = state
        .chat_repo
        .remove_participant(chat_id, auth.user_id, target_id)
        .await?;

    publish_group_event(
        &state.redis,
        chat_id,
        "participant_removed",
        serde_json::json!({ "user_id": target_id }),
    )
    .await;

    if key_rotation_required {
        publish_group_event(
            &state.redis,
            chat_id,
            "key_rotation_required",
            serde_json::json!({ "chat_id": chat_id }),
        )
        .await;
    }

    let response = RemoveParticipantResponse {
        removed: true,
        key_rotation_required,
    };
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// PATCH /chats/:id/participants/:user_id/role
pub async fn update_participant_role(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path((chat_id, target_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Response, ApiError> {
    let new_role =
        domain::chat::entity::ParticipantRole::from_db_str(&req.role).ok_or_else(|| {
            DomainError::Validation(format!(
                "invalid role '{}'. Must be one of: member, moderator, admin",
                req.role
            ))
        })?;

    // Prevent promoting to owner via this endpoint
    if new_role == domain::chat::entity::ParticipantRole::Owner {
        return Err(ApiError(DomainError::Validation(
            "cannot promote to owner via this endpoint. Use POST /chats/:id/transfer-ownership"
                .to_string(),
        )));
    }

    let participant = state
        .chat_repo
        .update_participant_role(chat_id, auth.user_id, target_id, new_role)
        .await?;

    publish_group_event(
        &state.redis,
        chat_id,
        "participant_role_updated",
        serde_json::json!({ "user_id": target_id, "role": new_role.as_db_str() }),
    )
    .await;

    let response = UpdateRoleResponse {
        participant: participant_to_response(participant),
    };
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /chats/:id/invite-link
pub async fn create_invite_link(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(chat_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let slug = state
        .chat_repo
        .set_invite_link(chat_id, auth.user_id, None)
        .await?
        .ok_or_else(|| DomainError::Internal("failed to generate invite link".to_string()))?;

    let response = InviteLinkResponse { invite_link: slug };
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// DELETE /chats/:id/invite-link
pub async fn delete_invite_link(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(chat_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    // Pass Some("") to signal deletion
    state
        .chat_repo
        .set_invite_link(chat_id, auth.user_id, Some(String::new()))
        .await?;

    let response = DeleteInviteLinkResponse { deleted: true };
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /chats/join/:slug
pub async fn join_by_slug(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(slug): Path<String>,
) -> Result<Response, ApiError> {
    let chat = state
        .chat_repo
        .find_chat_by_slug(&slug)
        .await?
        .ok_or_else(|| DomainError::NotFound("invite link not found or expired".to_string()))?;

    let participant = state
        .chat_repo
        .join_by_invite(chat.id, auth.user_id)
        .await?;

    publish_group_event(
        &state.redis,
        chat.id,
        "participant_added",
        serde_json::json!({ "user_id": participant.user_id }),
    )
    .await;

    let response = JoinGroupResponse {
        key_rotation_required: true, // new member needs key from an admin
        participant: participant_to_response(participant),
    };
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// POST /chats/:id/rotate-key
pub async fn rotate_group_key(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(chat_id): Path<Uuid>,
    Json(req): Json<RotateKeyRequest>,
) -> Result<Response, ApiError> {
    if req.keys.is_empty() {
        return Err(ApiError(DomainError::Validation(
            "keys list cannot be empty".to_string(),
        )));
    }

    let pairs: Vec<(Uuid, String)> = req
        .keys
        .into_iter()
        .map(
            |KeyEntry {
                 user_id,
                 encryption_key_enc,
             }| (user_id, encryption_key_enc),
        )
        .collect();

    let updated_count = state
        .chat_repo
        .rotate_group_key(chat_id, auth.user_id, pairs)
        .await?;

    publish_group_event(
        &state.redis,
        chat_id,
        "key_rotated",
        serde_json::json!({ "chat_id": chat_id }),
    )
    .await;

    let response = RotateKeyResponse { updated_count };
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /chats/:id/transfer-ownership
pub async fn transfer_ownership(
    State(state): State<ChatsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(chat_id): Path<Uuid>,
    Json(req): Json<TransferOwnershipRequest>,
) -> Result<Response, ApiError> {
    if req.new_owner_id == auth.user_id {
        return Err(ApiError(DomainError::Validation(
            "new owner must be a different user".to_string(),
        )));
    }

    state
        .chat_repo
        .transfer_ownership(chat_id, auth.user_id, req.new_owner_id)
        .await?;

    publish_group_event(
        &state.redis,
        chat_id,
        "ownership_transferred",
        serde_json::json!({
            "previous_owner_id": auth.user_id,
            "new_owner_id": req.new_owner_id,
        }),
    )
    .await;

    let response = TransferOwnershipResponse { transferred: true };
    Ok((StatusCode::OK, Json(response)).into_response())
}
