pub mod dto;
pub mod handlers;

use infrastructure::repositories::stories::PostgresStoryRepository;
use std::sync::Arc;

pub use handlers::*;

#[derive(Clone)]
pub struct StoriesState {
    pub story_repo: Arc<PostgresStoryRepository>,
}

pub use dto::*;
