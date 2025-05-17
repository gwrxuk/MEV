use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

mod cli;
mod defaults;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api: ApiConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub blockchain: BlockchainConfig,
    pub logging: LoggingConfig,
    pub services: ServicesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub bind_address: String,
    pub cors_allowed_origins: Vec<String>,
    pub request_timeout_seconds: u64,
    pub max_json_payload_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub idle_timeout_seconds: u64,
    pub connect_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub chain_id: u64,
    pub max_block_history: u64,
    pub confirmation_blocks: u64,
    pub gas_price_refresh_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub json_format: bool,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub tx_ordering: TxOrderingConfig,
    pub block_building: BlockBuildingConfig,
    pub liquid_staking: LiquidStakingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOrderingConfig {
    pub worker_threads: usize,
    pub max_simulation_time_ms: u64,
    pub simulation_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockBuildingConfig {
    pub target_block_fullness: f64,
    pub max_gas_limit: u64,
    pub priority_accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidStakingConfig {
    pub validator_commission_bps: u32,
    pub withdrawal_delay_epochs: u32,
    pub min_stake_amount: String,
}

/// Loads configuration from file and environment variables
pub fn load() -> Result<Config> {
    // Initialize dotenv
    dotenv::dotenv().ok();
    
    // Parse command line arguments
    let args = cli::parse_args();
    
    // Load config from file
    let config_path = args.config.as_deref().unwrap_or("config/default.yaml");
    let mut config = load_from_file(config_path)?;
    
    // Override with environment variables
    apply_env_overrides(&mut config)?;
    
    // Validate configuration
    validate_config(&config)?;
    
    info!("Configuration loaded successfully");
    Ok(config)
}

fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
    let config_file = std::fs::File::open(path)
        .context("Failed to open configuration file")?;
    
    let config: Config = serde_yaml::from_reader(config_file)
        .context("Failed to parse configuration file")?;
    
    Ok(config)
}

fn apply_env_overrides(config: &mut Config) -> Result<()> {
    // This would use a more sophisticated approach in a real implementation
    // to recursively traverse the config structure and apply environment variables
    
    // Example for database URL override
    if let Ok(db_url) = std::env::var("DATABASE_URL") {
        config.database.url = db_url;
    }
    
    // Example for blockchain node URL override
    if let Ok(rpc_url) = std::env::var("BLOCKCHAIN_RPC_URL") {
        config.blockchain.rpc_url = rpc_url;
    }
    
    Ok(())
}

fn validate_config(config: &Config) -> Result<()> {
    // Validate API configuration
    if config.api.bind_address.is_empty() {
        anyhow::bail!("API bind address cannot be empty");
    }
    
    // Validate database configuration
    if config.database.url.is_empty() {
        anyhow::bail!("Database URL cannot be empty");
    }
    
    // Validate blockchain configuration
    if config.blockchain.rpc_url.is_empty() || config.blockchain.ws_url.is_empty() {
        anyhow::bail!("Blockchain RPC and WebSocket URLs must be provided");
    }
    
    // Additional validation for specific services could be added here
    
    Ok(())
} 