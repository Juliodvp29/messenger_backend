use super::super::user::value_objects::UserId;
use super::entity::Contact;
use super::value_objects::{ContactId, ContactPhoneNumber};
use shared::error::DomainResult;

pub trait ContactRepository: Send + Sync {
    async fn create(&self, contact: &Contact) -> DomainResult<()>;
    async fn find_by_id(&self, id: &ContactId) -> DomainResult<Option<Contact>>;
    async fn find_by_owner_and_phone(
        &self,
        owner_id: &UserId,
        phone: &ContactPhoneNumber,
    ) -> DomainResult<Option<Contact>>;
    async fn find_all_by_owner(&self, owner_id: &UserId) -> DomainResult<Vec<Contact>>;
    async fn find_favorites(&self, owner_id: &UserId) -> DomainResult<Vec<Contact>>;
    async fn update(&self, contact: &Contact) -> DomainResult<()>;
    async fn delete(&self, id: &ContactId) -> DomainResult<()>;
    async fn delete_by_owner(&self, owner_id: &UserId) -> DomainResult<()>;
}
