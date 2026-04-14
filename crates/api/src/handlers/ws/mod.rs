pub mod dto;
pub mod handlers;

pub use handlers::ws_handler;

use dashmap::DashMap;
use infrastructure::repositories::chat::PostgresChatRepository;
use infrastructure::repositories::user::PostgresUserRepository;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct WsState {
    pub connections: Arc<DashMap<Uuid, Vec<Arc<Mutex<tokio::sync::mpsc::Sender<String>>>>>>,
    pub redis: ConnectionManager,
    pub redis_url: String,
    pub user_repo: Arc<PostgresUserRepository>,
    pub chat_repo: Arc<PostgresChatRepository>,
}
