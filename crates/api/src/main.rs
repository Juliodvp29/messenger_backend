use shared::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load and validate configuration
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        }
    };

    println!("Configuration loaded successfully!");
    println!("App Environment: {:?}", config.app_env);
    println!(
        "Server listening on {}:{}",
        config.server.host, config.server.port
    );

    Ok(())
}
