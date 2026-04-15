use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BlockResponse {
    pub id: String,
    pub blocked_id: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct BlockListResponse {
    pub blocks: Vec<BlockResponse>,
}
