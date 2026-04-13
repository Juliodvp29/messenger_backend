use super::value_objects::*;
use crate::user::value_objects::{PhoneNumber, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: ContactId,
    pub owner_id: UserId,
    pub contact_id: Option<UserId>,
    pub phone: PhoneNumber,
    pub nickname: Option<String>,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
}

impl Contact {
    pub fn new(owner_id: UserId, phone: PhoneNumber, contact_id: Option<UserId>) -> Self {
        Self {
            id: ContactId::new(),
            owner_id,
            contact_id,
            phone,
            nickname: None,
            is_favorite: false,
            created_at: Utc::now(),
        }
    }

    pub fn with_nickname(mut self, nickname: String) -> Self {
        self.nickname = Some(nickname);
        self
    }

    pub fn set_favorite(&mut self, is_favorite: bool) {
        self.is_favorite = is_favorite;
    }
}
