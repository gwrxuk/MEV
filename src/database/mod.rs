use anyhow::{Context, Result};
use redis::{Client as RedisClient, aio::ConnectionManager as RedisConnectionManager};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tracing::info;

use crate::config::{DatabaseConfig, RedisConfig};

pub mod migrations;
pub mod models;
pub mod repositories;

pub type DbPool = Pool<Postgres>;
pub type RedisPool = RedisConnectionManager;

/// Connect to the PostgreSQL database
pub async fn connect(config: &DatabaseConfig) -> Result<DbPool> {
    info!("Connecting to database at {}", config.url);
    
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .idle_timeout(std::time::Duration::from_secs(config.idle_timeout_seconds))
        .connect_timeout(std::time::Duration::from_secs(config.connect_timeout_seconds))
        .connect(&config.url)
        .await
        .context("Failed to connect to database")?;
    
    info!("Successfully connected to database");
    
    Ok(pool)
}

/// Connect to Redis
pub async fn connect_redis(config: &RedisConfig) -> Result<RedisPool> {
    info!("Connecting to Redis at {}", config.url);
    
    let client = RedisClient::open(config.url.as_str())
        .context("Failed to create Redis client")?;
    
    let manager = RedisConnectionManager::new(client)
        .await
        .context("Failed to create Redis connection manager")?;
    
    info!("Successfully connected to Redis");
    
    Ok(manager)
}

/// Run database migrations
pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    info!("Running database migrations");
    
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("Failed to run database migrations")?;
    
    info!("Database migrations completed successfully");
    
    Ok(())
} 