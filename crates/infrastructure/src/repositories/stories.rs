use async_trait::async_trait;
use domain::stories::entity::{
    Story, StoryPrivacyException, StoryView, StoryViewWithUser, StoryWithUser,
};
use domain::stories::repository::{ActiveStoryRepository, StoryRepository};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresStoryRepository {
    pool: PgPool,
}

impl PostgresStoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StoryRepository for PostgresStoryRepository {
    async fn create(
        &self,
        user_id: Uuid,
        content_url: String,
        content_type: String,
        caption: Option<String>,
        privacy: String,
    ) -> Result<Story, Box<dyn std::error::Error + Send + Sync>> {
        let row = sqlx::query(
            r#"INSERT INTO stories (user_id, content_url, content_type, caption, privacy) 
            VALUES ($1, $2, $3, $4, $5) RETURNING id, user_id, content_url, content_type, caption, privacy::text, created_at, expires_at, deleted_at"#)
            .bind(user_id).bind(content_url).bind(content_type).bind(caption).bind(privacy)
            .fetch_one(&self.pool).await?;

        Ok(Story {
            id: row.get("id"),
            user_id: row.get("user_id"),
            content_url: row.get("content_url"),
            content_type: row.get("content_type"),
            caption: row.get("caption"),
            privacy: row.get::<String, _>("privacy"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
            deleted_at: row.get("deleted_at"),
        })
    }

    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Story>, Box<dyn std::error::Error + Send + Sync>> {
        let row = sqlx::query(
            r#"SELECT id, user_id, content_url, content_type, caption, privacy::text as privacy, 
            created_at, expires_at, deleted_at FROM stories WHERE id = $1 AND deleted_at IS NULL AND expires_at > NOW()"#)
            .bind(id).fetch_optional(&self.pool).await?;

        Ok(row.map(|r| Story {
            id: r.get("id"),
            user_id: r.get("user_id"),
            content_url: r.get("content_url"),
            content_type: r.get("content_type"),
            caption: r.get("caption"),
            privacy: r.get::<String, _>("privacy"),
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
            deleted_at: r.get("deleted_at"),
        }))
    }

    async fn find_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Story>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, content_url, content_type, caption, privacy::text as privacy,
            created_at, expires_at, deleted_at FROM stories WHERE user_id = $1 AND deleted_at IS NULL AND expires_at > NOW()
            ORDER BY created_at DESC"#)
            .bind(user_id).fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|r| Story {
                id: r.get("id"),
                user_id: r.get("user_id"),
                content_url: r.get("content_url"),
                content_type: r.get("content_type"),
                caption: r.get("caption"),
                privacy: r.get::<String, _>("privacy"),
                created_at: r.get("created_at"),
                expires_at: r.get("expires_at"),
                deleted_at: r.get("deleted_at"),
            })
            .collect())
    }

    async fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query("UPDATE stories SET deleted_at = NOW() WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_privacy_exception(
        &self,
        story_id: Uuid,
        user_id: Uuid,
        is_excluded: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query("INSERT INTO story_privacy_exceptions (story_id, user_id, is_excluded) VALUES ($1, $2, $3) ON CONFLICT DO UPDATE SET is_excluded = $3")
            .bind(story_id).bind(user_id).bind(is_excluded).execute(&self.pool).await?;
        Ok(())
    }

    async fn list_privacy_exceptions(
        &self,
        story_id: Uuid,
    ) -> Result<Vec<StoryPrivacyException>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query_as!(StoryPrivacyException,
            "SELECT story_id, user_id, is_excluded FROM story_privacy_exceptions WHERE story_id = $1", story_id)
            .fetch_all(&self.pool).await?;
        Ok(rows)
    }

    async fn remove_privacy_exception(
        &self,
        story_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query("DELETE FROM story_privacy_exceptions WHERE story_id = $1 AND user_id = $2")
            .bind(story_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn mark_viewed(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query("INSERT INTO story_views (story_id, viewer_id) VALUES ($1, $2) ON CONFLICT DO UPDATE SET viewed_at = NOW()")
            .bind(story_id).bind(viewer_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn add_reaction(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
        reaction: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query("INSERT INTO story_views (story_id, viewer_id, reaction) VALUES ($1, $2, $3) ON CONFLICT DO UPDATE SET reaction = $3, viewed_at = NOW()")
            .bind(story_id).bind(viewer_id).bind(reaction).execute(&self.pool).await?;
        Ok(())
    }

    async fn remove_reaction(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::query("UPDATE story_views SET reaction = NULL, viewed_at = NOW() WHERE story_id = $1 AND viewer_id = $2")
            .bind(story_id).bind(viewer_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_views(
        &self,
        story_id: Uuid,
    ) -> Result<Vec<StoryView>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query_as!(StoryView,
            "SELECT id, story_id, viewer_id, reaction, viewed_at FROM story_views WHERE story_id = $1 ORDER BY viewed_at DESC", story_id)
            .fetch_all(&self.pool).await?;
        Ok(rows)
    }

    async fn get_views_with_user(
        &self,
        story_id: Uuid,
    ) -> Result<Vec<StoryViewWithUser>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query_as!(StoryViewWithUser,
            r#"SELECT sv.viewer_id, up.display_name, u.avatar_url, sv.reaction, sv.viewed_at
            FROM story_views sv JOIN users u ON u.id = sv.viewer_id
            LEFT JOIN user_profiles up ON up.user_id = sv.viewer_id WHERE sv.story_id = $1 ORDER BY sv.viewed_at DESC"#, story_id)
            .fetch_all(&self.pool).await?;
        Ok(rows)
    }

    async fn has_viewed(
        &self,
        story_id: Uuid,
        viewer_id: Uuid,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let exists = sqlx::query_scalar!(
            "SELECT TRUE FROM story_views WHERE story_id = $1 AND viewer_id = $2",
            story_id,
            viewer_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(exists.is_some())
    }
}

#[async_trait]
impl ActiveStoryRepository for PostgresStoryRepository {
    async fn list_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<StoryWithUser>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query(
            r#"SELECT s.id, s.user_id, s.content_url, s.content_type, s.caption, s.privacy::text as privacy, s.created_at, s.expires_at,
            u.username, up.display_name, u.avatar_url, FALSE as has_viewed
            FROM stories s JOIN users u ON u.id = s.user_id LEFT JOIN user_profiles up ON up.user_id = s.user_id
            WHERE s.deleted_at IS NULL AND s.expires_at > NOW() AND s.user_id != $1
            AND (s.privacy = 'everyone' OR (s.privacy = 'contacts' AND EXISTS(
                SELECT 1 FROM contacts c WHERE c.owner_id = s.user_id AND c.contact_id = $1)))
            ORDER BY s.created_at DESC"#)
            .bind(user_id).fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|r| StoryWithUser {
                id: r.get("id"),
                user_id: r.get("user_id"),
                content_url: r.get("content_url"),
                content_type: r.get("content_type"),
                caption: r.get("caption"),
                privacy: r.get::<String, _>("privacy"),
                created_at: r.get("created_at"),
                expires_at: r.get("expires_at"),
                username: r.get("username"),
                display_name: r.get("display_name"),
                avatar_url: r.get("avatar_url"),
                has_viewed: r.get("has_viewed"),
            })
            .collect())
    }

    async fn list_my_stories(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<StoryWithUser>, Box<dyn std::error::Error + Send + Sync>> {
        let rows = sqlx::query(
            r#"SELECT s.id, s.user_id, s.content_url, s.content_type, s.caption, s.privacy::text as privacy, s.created_at, s.expires_at,
            u.username, up.display_name, u.avatar_url, FALSE as has_viewed
            FROM stories s JOIN users u ON u.id = s.user_id LEFT JOIN user_profiles up ON up.user_id = s.user_id
            WHERE s.user_id = $1 AND s.deleted_at IS NULL AND s.expires_at > NOW()
            ORDER BY s.created_at DESC"#)
            .bind(user_id).fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|r| StoryWithUser {
                id: r.get("id"),
                user_id: r.get("user_id"),
                content_url: r.get("content_url"),
                content_type: r.get("content_type"),
                caption: r.get("caption"),
                privacy: r.get::<String, _>("privacy"),
                created_at: r.get("created_at"),
                expires_at: r.get("expires_at"),
                username: r.get("username"),
                display_name: r.get("display_name"),
                avatar_url: r.get("avatar_url"),
                has_viewed: r.get("has_viewed"),
            })
            .collect())
    }
}
