use axum::{
    Router, middleware,
    routing::{delete, get, patch, post},
};

use crate::handlers::attachments::{AttachmentsState, confirm_attachment, create_upload_url};
use crate::handlers::auth::{
    delete_session, list_sessions, login, login_verify, logout, recover, recover_verify, refresh,
    register, two_fa_setup, two_fa_setup_verify, two_fa_verify, verify_phone,
};
use crate::handlers::chats::{
    ChatsState, add_reaction, create_chat, delete_chat, delete_message, edit_message, get_chat,
    list_chats, list_messages, mark_messages_read, remove_reaction, send_message, update_chat,
};
use crate::handlers::keys::{
    KeysState, get_fingerprint, get_key_bundle, get_my_prekey_count, upload_keys, upload_prekeys,
};
use crate::middleware::auth::{AuthMiddlewareState, auth_middleware};
use crate::services::jwt::JwtService;
use crate::services::otp::OtpService;
use crate::services::storage::S3StorageService;
use infrastructure::repositories::chat::PostgresChatRepository;
use infrastructure::repositories::keys::PostgresKeyRepository;
use infrastructure::repositories::user::PostgresUserRepository;
use redis::aio::ConnectionManager;
use shared::config::Config;
use std::sync::Arc;

pub fn create_router(
    config: &Config,
    db_pool: sqlx::PgPool,
    redis_manager: ConnectionManager,
) -> Router {
    let user_repo = Arc::new(PostgresUserRepository::new(db_pool.clone()));
    let chat_repo = Arc::new(PostgresChatRepository::new(db_pool.clone()));
    let key_repo = Arc::new(PostgresKeyRepository::new(db_pool.clone()));

    let otp_service = Arc::new(OtpService::new(redis_manager.clone(), 600));

    let jwt_service = Arc::new(JwtService::new(
        config.jwt.private_key.clone(),
        config.jwt.public_key.clone(),
        config.jwt.access_ttl_seconds,
        config.jwt.refresh_ttl_seconds,
        Some(redis_manager.clone()),
    ));

    let auth_middleware_state = Arc::new(AuthMiddlewareState {
        jwt_service: jwt_service.clone(),
        redis: redis_manager.clone(),
        user_repo: user_repo.clone(),
    });

    let auth_state = crate::handlers::auth::AuthState {
        user_repo: user_repo.clone(),
        otp_service,
        jwt_service: jwt_service.clone(),
    };

    let keys_state = KeysState {
        key_repo: key_repo.clone(),
    };

    let chats_state = ChatsState {
        chat_repo: chat_repo.clone(),
        redis: redis_manager.clone(),
    };

    let attachments_state = AttachmentsState {
        chat_repo: chat_repo.clone(),
        storage: Arc::new(S3StorageService::new(&config.s3)),
    };

    let protected_auth_routes = Router::new()
        .route("/auth/2fa/setup", post(two_fa_setup))
        .route("/auth/2fa/setup/verify", post(two_fa_setup_verify))
        .route("/auth/sessions", get(list_sessions))
        .route("/auth/sessions/:session_id", delete(delete_session))
        .route("/auth/logout", post(logout))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware_state.clone(),
            auth_middleware,
        ));

    let protected_keys_routes = Router::new()
        .route("/keys/upload", post(upload_keys))
        .route("/keys/upload-prekeys", post(upload_prekeys))
        .route("/keys/me/count", get(get_my_prekey_count))
        .route("/keys/:user_id", get(get_key_bundle))
        .route("/keys/:user_id/fingerprint", get(get_fingerprint))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(keys_state);

    let protected_chat_routes = Router::new()
        .route("/chats", post(create_chat).get(list_chats))
        .route(
            "/chats/:id",
            get(get_chat).patch(update_chat).delete(delete_chat),
        )
        .route("/chats/:id/messages", post(send_message).get(list_messages))
        .route(
            "/chats/:id/messages/:message_id",
            patch(edit_message).delete(delete_message),
        )
        .route("/chats/:id/messages/read", post(mark_messages_read))
        .route(
            "/chats/:id/messages/:message_id/reactions",
            post(add_reaction),
        )
        .route(
            "/chats/:id/messages/:message_id/reactions/remove",
            post(remove_reaction),
        )
        .route_layer(middleware::from_fn_with_state(
            auth_middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(chats_state);

    let protected_attachment_routes = Router::new()
        .route("/attachments/upload-url", post(create_upload_url))
        .route("/attachments/confirm", post(confirm_attachment))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware_state.clone(),
            auth_middleware,
        ))
        .with_state(attachments_state);

    Router::new()
        .route("/health", get(health))
        .route("/auth/register", post(register))
        .route("/auth/verify-phone", post(verify_phone))
        .route("/auth/login", post(login))
        .route("/auth/login/verify", post(login_verify))
        .route("/auth/recover", post(recover))
        .route("/auth/recover/verify", post(recover_verify))
        .route("/auth/2fa/verify", post(two_fa_verify))
        .route("/auth/refresh", post(refresh))
        .merge(protected_auth_routes)
        .merge(protected_keys_routes)
        .merge(protected_chat_routes)
        .merge(protected_attachment_routes)
        .with_state(auth_state)
}

async fn health() -> &'static str {
    "OK"
}
