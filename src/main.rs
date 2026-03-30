mod config;
mod types;

use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ergo-proxy.toml".to_string());

    let config = Config::load(&config_path)?;
    tracing::info!(network = ?config.proxy.network, "Ergo proxy node starting");

    Ok(())
}
