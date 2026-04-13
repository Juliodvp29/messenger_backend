use api::routes::create_router;
use shared::config::Config;
use sqlx::postgres::PgPoolOptions;
use redis::aio::ConnectionManager;
use redis::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::load()
        .map_err(|e| {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        })?;

    let db_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(600))
        .test_before_acquire(true)
        .connect(&config.database.url)
        .await
        .expect("No se pudo conectar a PostgreSQL");

    let redis_client = Client::open(config.redis.url.clone())
        .expect("Invalid Redis URL");
    let redis_manager = ConnectionManager::new(redis_client)
        .await
        .expect("No se pudo conectar a Redis");

    let app = create_router(&config, db_pool, redis_manager);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    println!("Servidor escuchando en {}", addr);
    println!("Ambiente: {:?}", config.app_env);

    axum::serve(listener, app).await?;

    Ok(())
}