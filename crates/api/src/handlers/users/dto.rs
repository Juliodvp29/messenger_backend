use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct UserSearchResult {
    pub id: String,
    pub username: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub id: String,
    pub username: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub status_text: Option<String>,
}
