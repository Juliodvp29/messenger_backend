use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::error::ServiceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub sid: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshData {
    pub sub: String,
    pub sid: String,
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    pub push_token: Option<String>,
}

#[derive(Clone)]
pub struct JwtService {
    secret: Vec<u8>,
    refresh_secret: Vec<u8>,
    access_ttl: u64,
    refresh_ttl: u64,
}

impl JwtService {
    pub fn new(secret: String, refresh_secret: String, access_ttl: u64, refresh_ttl: u64) -> Self {
        Self {
            secret: secret.into_bytes(),
            refresh_secret: refresh_secret.into_bytes(),
            access_ttl,
            refresh_ttl,
        }
    }

    pub fn generate_access_token(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
    ) -> Result<String, ServiceError> {
        let now = chrono::Utc::now().timestamp();
        let claims = AccessClaims {
            sub: user_id.to_string(),
            sid: session_id.to_string(),
            exp: now + self.access_ttl as i64,
            iat: now,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(token)
    }

    pub fn generate_temp_token(&self, user_id: &Uuid) -> Result<String, ServiceError> {
        let now = chrono::Utc::now().timestamp();
        let claims = AccessClaims {
            sub: user_id.to_string(),
            sid: "temp".to_string(),
            exp: now + 300,
            iat: now,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(token)
    }

    pub fn validate_access_token(&self, token: &str) -> Result<AccessClaims, ServiceError> {
        let claims = decode::<AccessClaims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Validation::default(),
        )
        .map_err(|e| ServiceError::Unauthorized(e.to_string()))?
        .claims;

        let now = chrono::Utc::now().timestamp();
        if claims.exp < now {
            return Err(ServiceError::Unauthorized("Token expired".to_string()));
        }

        Ok(claims)
    }

    pub fn generate_refresh_token(&self, data: &RefreshData) -> Result<String, ServiceError> {
        let token = Uuid::new_v4().to_string();
        let json =
            serde_json::to_string(data).map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(format!("{}:{}", token, json))
    }

    pub fn validate_refresh_token(&self, token: &str) -> Result<RefreshData, ServiceError> {
        let parts: Vec<&str> = token.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ServiceError::Unauthorized(
                "Invalid token format".to_string(),
            ));
        }

        let data: RefreshData =
            serde_json::from_str(parts[1]).map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(data)
    }

    pub fn access_token_ttl(&self) -> u64 {
        self.access_ttl
    }

    pub fn refresh_token_ttl(&self) -> u64 {
        self.refresh_ttl
    }
}
