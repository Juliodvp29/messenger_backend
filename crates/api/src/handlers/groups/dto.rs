use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---- Request DTOs ----

#[derive(Debug, Deserialize)]
pub struct AddParticipantRequest {
    pub user_id: Uuid,
    /// The new group key encrypted for this user (optional at add-time;
    /// a key rotation call must follow if the group had a previous key).
    pub encryption_key_enc: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    /// Accepted values: "member" | "moderator" | "admin"
    /// (cannot promote to "owner" — use transfer-ownership instead)
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct RotateKeyRequest {
    /// List of (user_id, new_encryption_key_enc) pairs for all remaining members.
    pub keys: Vec<KeyEntry>,
}

#[derive(Debug, Deserialize)]
pub struct KeyEntry {
    pub user_id: Uuid,
    pub encryption_key_enc: String,
}

#[derive(Debug, Deserialize)]
pub struct TransferOwnershipRequest {
    pub new_owner_id: Uuid,
}

// ---- Response DTOs ----

#[derive(Debug, Serialize)]
pub struct ParticipantDetailResponse {
    pub user_id: Uuid,
    pub chat_id: Uuid,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_key_enc: Option<String>,
    pub added_by: Option<Uuid>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AddParticipantResponse {
    pub participant: ParticipantDetailResponse,
    /// True when the caller should follow up with POST /chats/:id/rotate-key.
    pub key_rotation_required: bool,
}

#[derive(Debug, Serialize)]
pub struct RemoveParticipantResponse {
    pub removed: bool,
    /// Always true — the group key must be rotated after any member leaves.
    pub key_rotation_required: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateRoleResponse {
    pub participant: ParticipantDetailResponse,
}

#[derive(Debug, Serialize)]
pub struct ListParticipantsResponse {
    pub participants: Vec<ParticipantDetailResponse>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct InviteLinkResponse {
    pub invite_link: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteInviteLinkResponse {
    pub deleted: bool,
}

#[derive(Debug, Serialize)]
pub struct JoinGroupResponse {
    pub participant: ParticipantDetailResponse,
    /// True when the group uses E2E encryption and the caller needs a key from an admin.
    pub key_rotation_required: bool,
}

#[derive(Debug, Serialize)]
pub struct RotateKeyResponse {
    pub updated_count: usize,
}

#[derive(Debug, Serialize)]
pub struct TransferOwnershipResponse {
    pub transferred: bool,
}
