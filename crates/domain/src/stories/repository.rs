use crate::stories::entity::{
    Story, StoryPrivacyException, StoryView, StoryViewWithUser, StoryWithUser,
};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait StoryRepository: Send + Sync {
    async fn create(
        &self,
        user_id: Uuid,
        content_url: String,
        content_type: String,
        caption: Option<String>,
        privacy: String,
    ) -> Result<Story, Box<dyn std::error::Error + Send + Sync>>;

    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Story>, Box<dyn std::error::Error + Send + Sync>>;

    async fn find_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Story>, Box<dyn std::error::Error + Send + Sync>>;

    async fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn add_privacy_exception(
        &self,
        story_id: Uuid,
        user_id: Uuid,
        is_excluded: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn list_privacy_exceptions(
        &self,
        story_id: Uuid,
    ) -> Result<Vec<StoryPrivacyException>, Box<dyn std::error::Error + Send + Sync>>;

    async fn remove_privacy_exception(
        &self,
        story_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn mark_viewed(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn add_reaction(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
        reaction: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn remove_reaction(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn get_views(
        &self,
        story_id: Uuid,
    ) -> Result<Vec<StoryView>, Box<dyn std::error::Error + Send + Sync>>;

    async fn get_views_with_user(
        &self,
        story_id: Uuid,
    ) -> Result<Vec<StoryViewWithUser>, Box<dyn std::error::Error + Send + Sync>>;

    async fn has_viewed(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    /// Check if a user has permission to view a story based on its privacy settings.
    /// Returns true if the viewer is allowed to see the story.
    async fn can_user_view_story(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait]
pub trait ActiveStoryRepository: Send + Sync {
    async fn list_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<StoryWithUser>, Box<dyn std::error::Error + Send + Sync>>;

    async fn list_my_stories(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<StoryWithUser>, Box<dyn std::error::Error + Send + Sync>>;
}
