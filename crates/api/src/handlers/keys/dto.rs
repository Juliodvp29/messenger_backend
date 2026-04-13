use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct UploadKeysRequest {
    pub identity_key: String,
    pub signed_prekey: SignedPrekeyUpload,
    #[serde(default)]
    pub one_time_prekeys: Vec<OneTimePrekeyUpload>,
}

#[derive(Debug, Deserialize)]
pub struct SignedPrekeyUpload {
    pub id: i32,
    pub key: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct OneTimePrekeyUpload {
    pub id: i32,
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct UploadKeysResponse {
    pub prekey_count: i32,
}

#[derive(Debug, Serialize)]
pub struct KeyBundleResponse {
    pub identity_key: String,
    pub signed_prekey: SignedPrekeyResponse,
    pub one_time_prekey: Option<OneTimePrekeyResponse>,
}

#[derive(Debug, Serialize)]
pub struct SignedPrekeyResponse {
    pub id: i32,
    pub key: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct OneTimePrekeyResponse {
    pub id: i32,
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct PrekeyCountResponse {
    pub count: i32,
}

#[derive(Debug, Serialize)]
pub struct FingerprintResponse {
    pub fingerprint: String,
    pub your_key: String,
    pub their_key: String,
    #[serde(default)]
    pub key_changed: Option<bool>,
    #[serde(default)]
    pub changed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}
