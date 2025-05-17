use std::sync::Arc;

use anyhow::Result;
use tokio::signal;
use tracing::{error, info};

mod api;
mod blockchain;
mod config;
mod core;
mod database;
mod models;
mod services;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize configuration
    let config = config::load()?;
    
    // Setup logging
    utils::logging::init(&config.logging)?;
    
    info!("Starting MEV Capture v{}", env!("CARGO_PKG_VERSION"));
    
    // Initialize database connections
    let db_pool = database::connect(&config.database).await?;
    let redis = database::connect_redis(&config.redis).await?;
    
    // Initialize blockchain client
    let blockchain_client = blockchain::create_client(&config.blockchain).await?;
    
    // Initialize core services
    let services = services::ServiceContext::new(
        db_pool.clone(),
        redis.clone(),
        blockchain_client.clone(),
        &config,
    ).await?;
    
    let services = Arc::new(services);
    
    // Initialize API server
    let api_server = api::start_server(
        config.api.bind_address.clone(),
        services.clone(),
    ).await?;
    
    info!("Server started on {}", config.api.bind_address);
    
    // Start blockchain monitoring
    let monitor_handle = blockchain::monitor::start(
        blockchain_client.clone(),
        services.clone(),
    ).await?;
    
    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => info!("Shutdown signal received, starting graceful shutdown"),
        Err(err) => error!("Failed to listen for shutdown signal: {}", err),
    }
    
    // Graceful shutdown
    api_server.shutdown().await?;
    monitor_handle.shutdown().await?;
    services.shutdown().await?;
    
    info!("Shutdown complete");
    Ok(())
}
