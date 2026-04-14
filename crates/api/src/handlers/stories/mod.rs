pub mod dto;
pub mod handlers;

use domain::stories::repository::{ActiveStoryRepository, StoryRepository};
use std::sync::Arc;

pub use handlers::*;

/// Combines both StoryRepository and ActiveStoryRepository behind a single Arc.
/// The concrete implementation (PostgresStoryRepository) implements both traits,
/// so we store it as Arc<dyn StoryRepo> where StoryRepo is a supertrait.
#[derive(Clone)]
pub struct StoriesState {
    pub story_repo: Arc<dyn StoryRepo>,
}

/// Supertrait combining StoryRepository + ActiveStoryRepository for ergonomic
/// dependency injection. The api layer depends only on domain traits, not
/// on the concrete PostgresStoryRepository.
pub trait StoryRepo: StoryRepository + ActiveStoryRepository {}
impl<T: StoryRepository + ActiveStoryRepository> StoryRepo for T {}

pub use dto::*;
