use serde::Serialize;
use std::path::Path;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, writer::MakeWriterExt},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AppEnv {
    Development,
    Staging,
    Production,
}

impl From<&str> for AppEnv {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "staging" => AppEnv::Staging,
            "production" => AppEnv::Production,
            _ => AppEnv::Development,
        }
    }
}

pub fn init_logging(app_env: AppEnv) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    match app_env {
        AppEnv::Development => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().pretty().with_target(true))
                .init();
        }
        _ => {
            let file_appender =
                tracing_appender::rolling::daily(Path::new("logs"), "messenger.log");
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

            // Keep guard alive - leak it to avoid dropping
            Box::leak(Box::new(_guard));

            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .json()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(non_blocking.and(std::io::stdout)),
                )
                .init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_env_from_str() {
        assert_eq!(AppEnv::from("development"), AppEnv::Development);
        assert_eq!(AppEnv::from("Development"), AppEnv::Development);
        assert_eq!(AppEnv::from("staging"), AppEnv::Staging);
        assert_eq!(AppEnv::from("production"), AppEnv::Production);
        assert_eq!(AppEnv::from("unknown"), AppEnv::Development);
    }
}
