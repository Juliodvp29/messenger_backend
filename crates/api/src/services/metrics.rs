use axum::response::IntoResponse;
use parking_lot::RwLock;
use prometheus::{Encoder, Registry, TextEncoder};
use std::sync::Arc;

pub struct Metrics {
    pub registry: Registry,

    pub messages_sent_total: prometheus::IntCounter,
    pub auth_attempts_total: prometheus::IntCounter,
    pub push_notifications_sent: prometheus::IntCounter,

    pub active_ws_connections: prometheus::IntGauge,
    pub db_pool_idle: prometheus::IntGauge,
    pub db_pool_active: prometheus::IntGauge,
    pub redis_connected_clients: prometheus::IntGauge,

    pub http_request_duration: prometheus::Histogram,
    pub db_query_duration: prometheus::Histogram,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let messages_sent_total = prometheus::IntCounter::with_opts(prometheus::Opts::new(
            "messages_sent_total",
            "Total number of messages sent by type",
        ))?;

        let auth_attempts_total = prometheus::IntCounter::with_opts(prometheus::Opts::new(
            "auth_attempts_total",
            "Total number of authentication attempts",
        ))?;

        let push_notifications_sent = prometheus::IntCounter::with_opts(prometheus::Opts::new(
            "push_notifications_sent",
            "Total number of push notifications sent",
        ))?;

        let active_ws_connections = prometheus::IntGauge::with_opts(prometheus::Opts::new(
            "active_ws_connections",
            "Number of active WebSocket connections",
        ))?;

        let db_pool_idle = prometheus::IntGauge::with_opts(prometheus::Opts::new(
            "db_pool_idle",
            "Number of idle database connections",
        ))?;

        let db_pool_active = prometheus::IntGauge::with_opts(prometheus::Opts::new(
            "db_pool_active",
            "Number of active database connections",
        ))?;

        let redis_connected_clients = prometheus::IntGauge::with_opts(prometheus::Opts::new(
            "redis_connected_clients",
            "Number of connected Redis clients",
        ))?;

        let http_request_duration = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
        )?;

        let db_query_duration = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "db_query_duration_seconds",
                "Database query duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
        )?;

        let registry = Registry::new();

        registry.register(Box::new(messages_sent_total.clone()))?;
        registry.register(Box::new(auth_attempts_total.clone()))?;
        registry.register(Box::new(push_notifications_sent.clone()))?;
        registry.register(Box::new(active_ws_connections.clone()))?;
        registry.register(Box::new(db_pool_idle.clone()))?;
        registry.register(Box::new(db_pool_active.clone()))?;
        registry.register(Box::new(redis_connected_clients.clone()))?;
        registry.register(Box::new(http_request_duration.clone()))?;
        registry.register(Box::new(db_query_duration.clone()))?;

        Ok(Self {
            registry,
            messages_sent_total,
            auth_attempts_total,
            push_notifications_sent,
            active_ws_connections,
            db_pool_idle,
            db_pool_active,
            redis_connected_clients,
            http_request_duration,
            db_query_duration,
        })
    }
}

pub type SharedMetrics = Arc<RwLock<Metrics>>;

pub fn create_metrics() -> Result<SharedMetrics, prometheus::Error> {
    let metrics = Metrics::new()?;
    Ok(Arc::new(RwLock::new(metrics)))
}

#[derive(Clone)]
pub struct MetricsExtension(pub SharedMetrics);

pub async fn metrics_handler(
    axum::extract::Extension(state): axum::extract::Extension<MetricsExtension>,
) -> impl IntoResponse {
    let metrics = state.0.read();
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    let metric_families = metrics.registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let body = String::from_utf8(buffer).unwrap();
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::HeaderName::from_static("content-type"),
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}
