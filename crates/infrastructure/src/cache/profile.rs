use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

const PROFILE_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedProfile {
    pub id: Uuid,
    pub username: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub status_text: Option<String>,
}

pub struct ProfileCache {
    redis: ConnectionManager,
}

impl ProfileCache {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    pub async fn get(&self, user_id: &Uuid) -> Result<Option<CachedProfile>, redis::RedisError> {
        let mut conn = self.redis.clone();
        let key = format!("profile:{}", user_id);
        let cached: Option<String> = conn.get(&key).await?;

        if let Some(json) = cached {
            let profile: CachedProfile = serde_json::from_str(&json).map_err(|e| {
                redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Failed to deserialize cached profile",
                    e.to_string(),
                ))
            })?;
            Ok(Some(profile))
        } else {
            Ok(None)
        }
    }

    pub async fn set(&self, profile: &CachedProfile) -> Result<(), redis::RedisError> {
        let mut conn = self.redis.clone();
        let key = format!("profile:{}", profile.id);
        let json = serde_json::to_string(profile).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Failed to serialize profile",
                e.to_string(),
            ))
        })?;
        let _: () = conn.set_ex(&key, json, PROFILE_TTL_SECONDS).await?;
        Ok(())
    }

    pub async fn invalidate(&self, user_id: &Uuid) -> Result<(), redis::RedisError> {
        let mut conn = self.redis.clone();
        let key = format!("profile:{}", user_id);
        let _: () = conn.del(&key).await?;
        Ok(())
    }
}

pub type ProfileCacheRef = Arc<ProfileCache>;
