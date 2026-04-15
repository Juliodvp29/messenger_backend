use axum::{
    body::Body,
    extract::{ConnectInfo, Extension},
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use std::time::Instant;
use uuid::Uuid;

use crate::services::metrics::MetricsExtension;

pub async fn logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(metrics): Extension<MetricsExtension>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let user_id = request
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut response = next.run(request).await;

    let latency = start.elapsed();
    let latency_ms = latency.as_millis() as u64;
    let status = response.status();

    // Record HTTP metric
    {
        let metrics = metrics.0.read();
        metrics.http_request_duration.observe(latency.as_secs_f64());
    }

    if status.is_server_error() || status.as_u16() >= 500 {
        tracing::error!(
            request_id = %request_id,
            method = %method,
            path = %uri,
            status = %status,
            latency_ms = %latency_ms,
            user_id = ?user_id,
            ip = %addr,
            "request failed"
        );
    } else {
        tracing::info!(
            request_id = %request_id,
            method = %method,
            path = %uri,
            status = %status,
            latency_ms = %latency_ms,
            user_id = ?user_id,
            ip = %addr,
            "request completed"
        );
    }

    // Add request ID to response header
    if let Ok(id_val) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", id_val);
    }

    response
}
