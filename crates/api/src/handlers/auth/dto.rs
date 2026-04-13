use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub phone: String,
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPhoneRequest {
    pub phone: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub phone: String,
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    pub push_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginVerifyRequest {
    pub phone: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub phone: String,
    pub username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub device_name: String,
    pub device_type: String,
    pub ip_address: Option<String>,
    pub last_active_at: Option<String>,
    pub is_current: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionsListResponse {
    pub sessions: Vec<SessionResponse>,
}

#[derive(Debug, Deserialize)]
pub struct TwoFactorSetupRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct TwoFactorChallengeRequest {
    pub temp_token: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct RecoverRequest {
    pub phone: String,
}

#[derive(Debug, Deserialize)]
pub struct RecoverVerifyRequest {
    pub phone: String,
    pub code: String,
}
