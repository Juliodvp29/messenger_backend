use axum::{
    extract::{
        Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use super::WsState;
use super::dto::{WsClientMessage, WsParams};
use crate::error::ApiError;
use crate::routes::WsRouterState;
use shared::error::DomainError;

pub async fn ws_handler(
    Query(params): Query<WsParams>,
    State(state): State<WsRouterState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let claims = match state.jwt_service.validate_access_token(&params.token) {
        Ok(c) => c,
        Err(_) => {
            return Err(ApiError(DomainError::Unauthorized(
                "Invalid or expired token".to_string(),
            )));
        }
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return Err(ApiError(DomainError::Unauthorized(
                "Invalid token format".to_string(),
            )));
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, user_id, state.ws_state.clone()));

    Ok(())
}

async fn handle_socket(socket: WebSocket, user_id: Uuid, ws_state: Arc<WsState>) {
    let (mut write, mut read) = socket.split();

    let (tx, mut rx) = mpsc::channel::<String>(100);

    {
        let mut user_connections = ws_state.connections.entry(user_id).or_insert_with(Vec::new);
        user_connections.push(Arc::new(Mutex::new(tx.clone())));
    }

    let redis = ws_state.redis.clone();
    let redis_url = ws_state.redis_url.clone();
    let _: Result<(), redis::RedisError> = redis::pipe()
        .set_ex(format!("presence:{}", user_id), "1", 65_u64)
        .query_async(&mut redis.clone())
        .await;

    let channel = format!("user:{}:events", user_id);
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let client = match redis::Client::open(redis_url.as_str()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut pub_sub = match client.get_async_pubsub().await {
            Ok(p) => p,
            Err(_) => return,
        };
        if pub_sub.subscribe(&channel).await.is_err() {
            return;
        }

        let mut stream = pub_sub.on_message();
        while let Some(msg) = stream.next().await {
            let payload: String = msg.get_payload().unwrap_or_default();
            let _ = tx_clone.send(payload).await;
        }
    });

    let user_id_for_read = user_id;
    let connections_for_cleanup = ws_state.connections.clone();
    let redis_for_cleanup = redis.clone();

    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(axum::extract::ws::Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
                        handle_client_message(client_msg, &tx, &redis, user_id_for_read).await;
                    }
                }
                Ok(axum::extract::ws::Message::Ping(_)) => {}
                Ok(axum::extract::ws::Message::Pong(_)) => {}
                Ok(axum::extract::ws::Message::Close(_)) | Err(_) => {
                    break;
                }
                _ => {}
            }
        }

        if let Some(entry) = connections_for_cleanup.get(&user_id_for_read) {
            let mut conns = entry.value().clone();
            conns.retain(|conn| {
                let guard = conn.blocking_lock();
                !guard.is_closed()
            });
        }

        let redis = redis_for_cleanup.clone();
        let _: Result<(), _> = redis::pipe()
            .del(format!("presence:{}", user_id_for_read))
            .query_async(&mut redis.clone())
            .await;

        tracing::info!("WebSocket disconnected for user {}", user_id_for_read);
    });

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write
                .send(axum::extract::ws::Message::Text(msg))
                .await
                .is_err()
            {
                break;
            }
        }
    });
}

async fn handle_client_message(
    msg: WsClientMessage,
    tx: &mpsc::Sender<String>,
    redis: &redis::aio::ConnectionManager,
    user_id: Uuid,
) {
    match msg {
        WsClientMessage::TypingStart { chat_id } => {
            let payload = json!({
                "type": "typing_start",
                "payload": {
                    "chat_id": chat_id,
                    "user_id": user_id
                },
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            let redis = redis.clone();
            let payload_str = payload.to_string();
            tokio::spawn(async move {
                let _: Result<(), _> = redis::pipe()
                    .publish(format!("chat:{}:events", chat_id), payload_str)
                    .query_async(&mut redis.clone())
                    .await;
            });
        }
        WsClientMessage::TypingStop { chat_id } => {
            let payload = json!({
                "type": "typing_stop",
                "payload": {
                    "chat_id": chat_id,
                    "user_id": user_id
                },
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            let redis = redis.clone();
            let payload_str = payload.to_string();
            tokio::spawn(async move {
                let _: Result<(), _> = redis::pipe()
                    .publish(format!("chat:{}:events", chat_id), payload_str)
                    .query_async(&mut redis.clone())
                    .await;
            });
        }
        WsClientMessage::SyncRequest { since: _ } => {
            let payload = json!({
                "type": "sync_response",
                "payload": {},
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            let _ = tx.send(payload.to_string()).await;
        }
    }
}
