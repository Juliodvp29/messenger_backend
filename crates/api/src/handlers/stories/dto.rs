use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateStoryRequest {
    pub content_url: String,
    pub content_type: String,
    pub caption: Option<String>,
    pub privacy: String,
    pub exceptions: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize)]
pub struct CreateStoryResponse {
    pub id: Uuid,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct StoryResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content_url: String,
    pub content_type: String,
    pub caption: Option<String>,
    pub privacy: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct StoryWithUserResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub content_url: String,
    pub content_type: String,
    pub caption: Option<String>,
    pub privacy: String,
    pub created_at: String,
    pub expires_at: String,
    pub username: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub has_viewed: bool,
}

#[derive(Debug, Deserialize)]
pub struct ViewStoryRequest {
    pub reaction: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StoryViewResponse {
    pub viewer_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub reaction: Option<String>,
    pub viewed_at: String,
}
