use anyhow::{Context, Result};
use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

use crate::services::ServiceContext;

mod handlers;
mod middleware;
mod models;
mod websocket;

/// API server handle for shutdown
pub struct ApiServer {
    server: axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>,
}

impl ApiServer {
    /// Gracefully shutdown the API server
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down API server");
        // This would be implemented with a graceful shutdown mechanism
        // In a real application, we would use the GracefulShutdown trait
        Ok(())
    }
}

/// Start the API server
pub async fn start_server(
    bind_address: String,
    services: Arc<ServiceContext>,
) -> Result<ApiServer> {
    // Create router
    let router = create_router(services);
    
    // Create server
    let addr = bind_address.parse()
        .context(format!("Failed to parse bind address: {}", bind_address))?;
    
    let server = axum::Server::bind(&addr)
        .serve(router.into_make_service());
    
    info!("API server listening on {}", bind_address);
    
    Ok(ApiServer { server })
}

/// Create the API router
fn create_router(services: Arc<ServiceContext>) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Middleware stack
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(Extension(services))
        .timeout(Duration::from_secs(30));
    
    // Main router
    Router::new()
        // API endpoints
        .route("/api/health", get(handlers::health::health_check))
        .route("/api/metrics", get(handlers::metrics::metrics))
        
        // Block building endpoints
        .route("/api/blocks/latest", get(handlers::blocks::get_latest_block))
        .route("/api/blocks/:block_number", get(handlers::blocks::get_block_by_number))
        .route("/api/blocks/simulate", post(handlers::blocks::simulate_block))
        
        // Transaction endpoints
        .route("/api/transactions", post(handlers::transactions::submit_transaction))
        .route("/api/transactions/:tx_hash", get(handlers::transactions::get_transaction))
        .route("/api/transactions/:tx_hash/receipt", get(handlers::transactions::get_transaction_receipt))
        
        // Liquid staking endpoints
        .route("/api/staking/validators", get(handlers::staking::get_validators))
        .route("/api/staking/stake", post(handlers::staking::stake))
        .route("/api/staking/unstake", post(handlers::staking::unstake))
        .route("/api/staking/rewards", get(handlers::staking::get_rewards))
        
        // WebSocket endpoints
        .route("/ws", get(websocket::handler))
        
        // Apply middleware
        .layer(middleware)
} 