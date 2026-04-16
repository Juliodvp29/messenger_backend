use api::routes::create_router;
use api::services::create_metrics;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use redis::Client;
use redis::aio::ConnectionManager;
use shared::config::Config;
use shared::logging::{AppEnv, init_logging};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config::load().map_err(|e| {
        eprintln!("Error loading configuration: {}", e);
        std::process::exit(1);
    })?;

    init_logging(AppEnv::from(config.app_env.as_str()));

    tracing::info!("Starting messenger backend");
    tracing::info!("Environment: {:?}", config.app_env);

    let db_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(600))
        .test_before_acquire(true)
        .connect(&config.database.url)
        .await
        .expect("Could not connect to PostgreSQL");

    let redis_client = Client::open(config.redis.url.clone()).expect("Invalid Redis URL");
    let redis_manager = ConnectionManager::new(redis_client.clone())
        .await
        .expect("Could not connect to Redis");

    let metrics = create_metrics().expect("Failed to create metrics");

    // Start background metrics worker
    let metrics_clone = metrics.clone();
    let db_pool_clone = db_pool.clone();
    let redis_client_clone = redis_client.clone();
    tokio::spawn(async move {
        loop {
            {
                let m = metrics_clone.read();
                // SQLx metrics
                m.db_pool_active.set(db_pool_clone.size() as i64);
                m.db_pool_idle.set(db_pool_clone.num_idle() as i64);

                // Redis metrics
                if let Ok(mut conn) = redis_client_clone.get_connection() {
                    let info: Result<String, _> =
                        redis::cmd("INFO").arg("clients").query(&mut conn);
                    if let Ok(info_str) = info {
                        for line in info_str.lines() {
                            if let Some(count) = line
                                .strip_prefix("connected_clients:")
                                .and_then(|s| s.trim().parse::<i64>().ok())
                            {
                                m.redis_connected_clients.set(count);
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });

    // Start push notification worker
    let worker_redis = redis_manager.clone();
    let worker_pool = db_pool.clone();
    let worker_config = config.push.clone();
    tokio::spawn(async move {
        api::services::push::push_notification_worker(worker_redis, worker_pool, worker_config)
            .await;
    });

    let app: IntoMakeServiceWithConnectInfo<_, SocketAddr> =
        create_router(&config, db_pool, redis_manager, metrics)
            .into_make_service_with_connect_info();

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("Server listening on {}", addr);
    println!("Environment: {:?}", config.app_env);

    axum::serve(listener, app).await?;

    Ok(())
}
