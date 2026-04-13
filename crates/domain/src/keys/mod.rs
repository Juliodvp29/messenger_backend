use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPrekey {
    pub id: i32,
    pub key: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimePrekey {
    pub id: i32,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserKeys {
    pub user_id: uuid::Uuid,
    pub identity_key: String,
    pub signed_prekey: SignedPrekey,
    pub signed_prekey_id: i32,
    pub prekey_count: i32,
}

pub trait KeyRepository: Send + Sync {
    async fn upsert_keys(
        &self,
        user_id: uuid::Uuid,
        identity_key: &str,
        signed_prekey: &SignedPrekey,
    ) -> Result<(), shared::error::DomainError>;

    async fn add_one_time_prekeys(
        &self,
        user_id: uuid::Uuid,
        prekeys: Vec<OneTimePrekey>,
    ) -> Result<(), shared::error::DomainError>;

    async fn get_keys(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Option<UserKeys>, shared::error::DomainError>;

    async fn get_prekey_count(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<i32, shared::error::DomainError>;

    async fn is_blocked(
        &self,
        user_id: uuid::Uuid,
        target_user_id: uuid::Uuid,
    ) -> Result<bool, shared::error::DomainError>;

    async fn get_public_key_bundle(
        &self,
        target_user_id: uuid::Uuid,
    ) -> Result<
        Option<(String, String, i32, String, Option<i32>, Option<String>)>,
        shared::error::DomainError,
    >;
}
