use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContactId(pub Uuid);

impl ContactId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ContactId {
    fn default() -> Self {
        Self::new()
    }
}

pub use super::super::user::value_objects::PhoneNumber as ContactPhoneNumber;
pub use super::super::user::value_objects::UserId as ContactUserId;
