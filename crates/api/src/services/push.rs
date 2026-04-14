use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PUSH_QUEUE_KEY: &str = "push:queue";
pub const PRESENCE_KEY_PREFIX: &str = "presence:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotificationJob {
    pub user_id: Uuid,
    pub session_ids: Vec<Uuid>,
    pub notification_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushToken {
    pub token: String,
    pub device_type: DeviceType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Android,
    Ios,
    Web,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Android => "android",
            Self::Ios => "ios",
            Self::Web => "web",
        }
    }
}

impl From<&str> for DeviceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "android" => Self::Android,
            "ios" => Self::Ios,
            "web" => Self::Web,
            _ => Self::Android,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmMessage {
    pub to: String,
    pub data: serde_json::Value,
    pub notification: Option<FcmNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmNotification {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnsMessage {
    pub aps: ApnsPayload,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnsPayload {
    pub alert: ApnsAlert,
    pub sound: Option<String>,
    pub badge: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnsAlert {
    pub title: String,
    pub body: String,
}
