use axum::{
    extract::{
        Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use domain::user::repository::UserRepository;
use futures_util::{SinkExt, StreamExt};
use infrastructure::repositories::chat::PostgresChatRepository;
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
    let user_repo = ws_state.user_repo.clone();
    let ws_state_clone = ws_state.clone();

    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(axum::extract::ws::Message::Text(text)) => {
                    if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
                        handle_client_message(client_msg, &tx, &redis, user_id_for_read, ws_state_clone.clone()).await;
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

        let user_id = domain::user::value_objects::UserId(user_id_for_read);
        let _ = user_repo.update_last_seen(&user_id, chrono::Utc::now()).await;

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
    ws_state: Arc<WsState>,
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

            let redis1 = redis.clone();
            let redis2 = redis.clone();
            let payload_str = payload.to_string();
            let chat_id1 = chat_id;
            tokio::spawn(async move {
                let mut redis = redis1.clone();
                let _: Result<(), _> = redis::pipe()
                    .publish(format!("chat:{}:events", chat_id1), payload_str)
                    .query_async(&mut redis)
                    .await;
            });

            let user_id_for_timer = user_id;
            let chat_id_for_timer = chat_id;
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                let payload = json!({
                    "type": "typing_stop",
                    "payload": {
                        "chat_id": chat_id_for_timer,
                        "user_id": user_id_for_timer
                    },
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                let payload_str = payload.to_string();
                let mut redis = redis2.clone();
                let _: Result<(), _> = redis::pipe()
                    .publish(format!("chat:{}:events", chat_id_for_timer), payload_str)
                    .query_async(&mut redis)
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
                let mut redis = redis.clone();
                let _: Result<(), _> = redis::pipe()
                    .publish(format!("chat:{}:events", chat_id), payload_str)
                    .query_async(&mut redis)
                    .await;
            });
        }
        WsClientMessage::SyncRequest { since } => {
            let chat_repo = ws_state.chat_repo.clone();
            let tx = tx.clone();
            let user_id = user_id;

            tokio::spawn(async move {
                if let Some(since) = since {
                    match sync_messages_after(chat_repo.as_ref(), user_id, since).await {
                        Ok(messages) => {
                            let payload = json!({
                                "type": "sync_response",
                                "payload": {
                                    "messages": messages
                                },
                                "timestamp": Utc::now().to_rfc3339()
                            });
                            let _ = tx.send(payload.to_string()).await;
                        }
                        Err(_) => {
                            let payload = json!({
                                "type": "sync_response",
                                "payload": {
                                    "messages": Vec::<serde_json::Value>::new()
                                },
                                "timestamp": Utc::now().to_rfc3339()
                            });
                            let _ = tx.send(payload.to_string()).await;
                        }
                    }
                } else {
                    let payload = json!({
                        "type": "sync_response",
                        "payload": {
                            "messages": Vec::<serde_json::Value>::new()
                        },
                        "timestamp": Utc::now().to_rfc3339()
                    });
                    let _ = tx.send(payload.to_string()).await;
                }
            });
        }
    }
}

async fn sync_messages_after(
    chat_repo: &PostgresChatRepository,
    user_id: Uuid,
    since: DateTime<Utc>,
) -> Result<Vec<serde_json::Value>, ()> {
    use domain::chat::repository::{ChatRepository, MessageDirection};
    
    let chat_previews = chat_repo
        .list_chats_for_user(user_id, None, 50)
        .await
        .map_err(|_| ())?;

    let mut all_messages = Vec::new();

    for preview in chat_previews {
        let messages = chat_repo
            .list_messages(user_id, preview.chat_id, None, MessageDirection::Before, 50)
            .await
            .map_err(|_| ())?;

        for msg in messages {
            if msg.created_at > since {
                all_messages.push(serde_json::json!({
                    "chat_id": msg.chat_id,
                    "message_id": msg.id,
                    "sender_id": msg.sender_id,
                    "content_encrypted": msg.content_encrypted,
                    "content_iv": msg.content_iv,
                    "message_type": msg.message_type,
                    "created_at": msg.created_at,
                }));
            }
        }
    }

    Ok(all_messages)
}
