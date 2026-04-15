use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppEnv {
    Development,
    Staging,
    Production,
}

impl AppEnv {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppEnv::Development => "development",
            AppEnv::Staging => "staging",
            AppEnv::Production => "production",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub app_env: AppEnv,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub s3: S3Config,
    pub smtp: SmtpConfig,
    pub sms: SmsConfig,
    pub push: PushConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: String,
    pub rate_limit_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub private_key: String,
    pub public_key: String,
    pub access_ttl_seconds: u64,
    pub refresh_ttl_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3Config {
    pub endpoint: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub from: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmsConfig {
    pub provider: String,
    pub api_key: String,
    pub sender_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PushConfig {
    pub fcm_api_key: Option<String>,
    pub apns_key_path: Option<String>,
    pub apns_key_id: Option<String>,
    pub apns_team_id: Option<String>,
    pub apns_bundle_id: String,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        settings.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_load() {
        let _ = dotenvy::dotenv();
        // En CI o local con .env, esto debería cargar sin errores
        // Si faltan variables requeridas, fallará
        let config = Config::load();
        assert!(config.is_ok(), "Config load failed: {:?}", config.err());
    }
}
