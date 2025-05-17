use axum::{
    extract::Extension,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::services::ServiceContext;

#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
    blockchain_connected: bool,
    database_connected: bool,
    redis_connected: bool,
}

/// Health check endpoint
pub async fn health_check(
    Extension(services): Extension<Arc<ServiceContext>>,
) -> Result<Json<HealthResponse>, StatusCode> {
    // Check database connection
    let db_connected = sqlx::query("SELECT 1")
        .fetch_one(&services.db_pool)
        .await
        .is_ok();
    
    // Check Redis connection
    let redis_connected = redis::cmd("PING")
        .query_async::<_, String>(&mut services.redis.clone())
        .await
        .is_ok();
    
    // Check blockchain connection
    let blockchain_connected = services
        .blockchain_client
        .get_block_number()
        .await
        .is_ok();
    
    // Get service uptime
    let uptime = services.start_time.elapsed();
    
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime.as_secs(),
        blockchain_connected,
        database_connected: db_connected,
        redis_connected,
    };
    
    Ok(Json(response))
} 