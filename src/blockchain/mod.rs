use anyhow::{Context, Result};
use ethers::{
    providers::{Http, Provider, Ws},
    signers::LocalWallet,
};
use std::sync::Arc;
use tracing::info;

use crate::config::BlockchainConfig;

pub mod client;
pub mod monitor;
pub mod transaction;
pub mod block;
pub mod simulator;

pub use client::BlockchainClient;

/// Create a new blockchain client from configuration
pub async fn create_client(config: &BlockchainConfig) -> Result<Arc<BlockchainClient>> {
    info!("Initializing blockchain client");
    
    // Create HTTP provider
    let http_provider = Provider::<Http>::try_from(&config.rpc_url)
        .context("Failed to create HTTP provider")?;
    
    // Create WebSocket provider
    let ws_provider = Provider::<Ws>::connect(&config.ws_url)
        .await
        .context("Failed to connect to WebSocket endpoint")?;
    
    // Create client
    let client = BlockchainClient::new(
        http_provider,
        ws_provider,
        config.chain_id,
        config.confirmation_blocks,
    );
    
    info!("Blockchain client initialized successfully");
    
    Ok(Arc::new(client))
} 