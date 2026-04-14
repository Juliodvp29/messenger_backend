use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content_url: String,
    pub content_type: String,
    pub caption: Option<String>,
    pub privacy: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryWithUser {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content_url: String,
    pub content_type: String,
    pub caption: Option<String>,
    pub privacy: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub has_viewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryPrivacyException {
    pub story_id: Uuid,
    pub user_id: Uuid,
    pub is_excluded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryView {
    pub id: Uuid,
    pub story_id: Uuid,
    pub viewer_id: Uuid,
    pub reaction: Option<String>,
    pub viewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryViewWithUser {
    pub viewer_id: Uuid,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub reaction: Option<String>,
    pub viewed_at: DateTime<Utc>,
}
