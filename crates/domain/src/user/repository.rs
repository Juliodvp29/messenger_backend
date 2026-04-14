use super::entity::User;
use super::value_objects::{PhoneNumber, UserId, Username};
use chrono::{DateTime, Utc};
use shared::error::DomainResult;

#[allow(async_fn_in_trait)]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &User) -> DomainResult<()>;
    async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>>;
    async fn find_by_phone(&self, phone: &PhoneNumber) -> DomainResult<Option<User>>;
    async fn find_by_username(&self, username: &Username) -> DomainResult<Option<User>>;
    async fn update(&self, user: &User) -> DomainResult<()>;
    async fn delete_soft(&self, id: &UserId) -> DomainResult<()>;
    async fn update_last_seen(
        &self,
        user_id: &UserId,
        timestamp: DateTime<Utc>,
    ) -> DomainResult<()>;
}
