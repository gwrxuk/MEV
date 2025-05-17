use anyhow::Result;
use ethers::types::{Block, Transaction, H256};
use futures::stream::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::interval,
};
use tracing::{debug, error, info, warn};

use crate::{
    blockchain::BlockchainClient,
    services::ServiceContext,
    utils::metrics::MetricsTimer,
};

/// Handle for the blockchain monitor
pub struct BlockchainMonitorHandle {
    shutdown_sender: mpsc::Sender<()>,
    tasks: Vec<JoinHandle<()>>,
}

impl BlockchainMonitorHandle {
    /// Shutdown the blockchain monitor
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down blockchain monitor");
        
        // Send shutdown signal
        let _ = self.shutdown_sender.send(()).await;
        
        // Wait for tasks to complete
        for task in self.tasks {
            if let Err(e) = task.await {
                warn!("Error waiting for task to complete: {}", e);
            }
        }
        
        info!("Blockchain monitor shut down successfully");
        Ok(())
    }
}

/// Start the blockchain monitor
pub async fn start(
    blockchain_client: Arc<BlockchainClient>,
    services: Arc<ServiceContext>,
) -> Result<BlockchainMonitorHandle> {
    info!("Starting blockchain monitor");
    
    // Channel for shutdown signal
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
    
    // Start block monitor
    let block_task = spawn_block_monitor(blockchain_client.clone(), services.clone(), shutdown_rx.clone());
    
    // Start transaction monitor
    let tx_task = spawn_transaction_monitor(blockchain_client.clone(), services.clone(), shutdown_rx.clone());
    
    // Start gas price monitor
    let gas_task = spawn_gas_price_monitor(blockchain_client.clone(), services.clone(), shutdown_rx.clone());
    
    info!("Blockchain monitor started successfully");
    
    // Return handle for shutdown
    Ok(BlockchainMonitorHandle {
        shutdown_sender: shutdown_tx,
        tasks: vec![block_task, tx_task, gas_task],
    })
}

/// Spawn a task to monitor for new blocks
fn spawn_block_monitor(
    blockchain_client: Arc<BlockchainClient>,
    services: Arc<ServiceContext>,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!("Block monitor started");
        
        let mut retry_count = 0;
        let max_retries = 10;
        
        'outer: loop {
            match blockchain_client.subscribe_blocks().await {
                Ok(mut stream) => {
                    retry_count = 0;
                    info!("Successfully subscribed to new blocks");
                    
                    loop {
                        tokio::select! {
                            Some(block) = stream.next() => {
                                let timer = MetricsTimer::new("block_processing_time_seconds");
                                if let Err(e) = process_new_block(blockchain_client.as_ref(), services.as_ref(), block).await {
                                    error!("Error processing new block: {}", e);
                                }
                                timer.stop();
                            }
                            _ = shutdown_rx.recv() => {
                                info!("Received shutdown signal, stopping block monitor");
                                break 'outer;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to subscribe to blocks: {}", e);
                    retry_count += 1;
                    
                    if retry_count > max_retries {
                        error!("Exceeded maximum retry count for block subscription, stopping monitor");
                        break;
                    }
                    
                    // Exponential backoff
                    let delay = Duration::from_secs(2u64.pow(retry_count.min(6) as u32));
                    warn!("Retrying block subscription in {:?}", delay);
                    tokio::time::sleep(delay).await;
                }
            }
        }
        
        info!("Block monitor stopped");
    })
}

/// Process a new block
async fn process_new_block(
    blockchain_client: &BlockchainClient,
    services: &ServiceContext,
    block: Block<Transaction>,
) -> Result<()> {
    let block_number = block.number.unwrap_or_default().as_u64();
    let block_hash = block.hash.unwrap_or_default();
    let tx_count = block.transactions.len();
    
    info!("New block: #{} ({}), containing {} transactions", block_number, block_hash, tx_count);
    
    // Update block metrics
    metrics::gauge!("blockchain_current_block", block_number as f64);
    
    // Process transactions in the block
    for tx in block.transactions {
        if let Err(e) = services.transaction_service.process_confirmed_transaction(tx).await {
            warn!("Failed to process confirmed transaction: {}", e);
        }
    }
    
    // Trigger block processing in services
    services.block_building_service.process_new_block(block).await?;
    
    Ok(())
}

/// Spawn a task to monitor for new transactions
fn spawn_transaction_monitor(
    blockchain_client: Arc<BlockchainClient>,
    services: Arc<ServiceContext>,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!("Transaction monitor started");
        
        let mut retry_count = 0;
        let max_retries = 10;
        
        'outer: loop {
            match blockchain_client.subscribe_pending_txs().await {
                Ok(mut stream) => {
                    retry_count = 0;
                    info!("Successfully subscribed to pending transactions");
                    
                    loop {
                        tokio::select! {
                            Some(tx_hash) = stream.next() => {
                                let timer = MetricsTimer::new("transaction_processing_time_seconds");
                                if let Err(e) = process_pending_transaction(blockchain_client.as_ref(), services.as_ref(), tx_hash).await {
                                    debug!("Error processing pending transaction {}: {}", tx_hash, e);
                                }
                                timer.stop();
                            }
                            _ = shutdown_rx.recv() => {
                                info!("Received shutdown signal, stopping transaction monitor");
                                break 'outer;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to subscribe to pending transactions: {}", e);
                    retry_count += 1;
                    
                    if retry_count > max_retries {
                        error!("Exceeded maximum retry count for transaction subscription, stopping monitor");
                        break;
                    }
                    
                    // Exponential backoff
                    let delay = Duration::from_secs(2u64.pow(retry_count.min(6) as u32));
                    warn!("Retrying transaction subscription in {:?}", delay);
                    tokio::time::sleep(delay).await;
                }
            }
        }
        
        info!("Transaction monitor stopped");
    })
}

/// Process a pending transaction
async fn process_pending_transaction(
    blockchain_client: &BlockchainClient,
    services: &ServiceContext,
    tx_hash: H256,
) -> Result<()> {
    // Get the full transaction
    if let Some(tx) = blockchain_client.get_transaction(tx_hash).await? {
        // Process the transaction
        services.transaction_service.process_pending_transaction(tx).await?;
    }
    
    Ok(())
}

/// Spawn a task to monitor gas prices
fn spawn_gas_price_monitor(
    blockchain_client: Arc<BlockchainClient>,
    services: Arc<ServiceContext>,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!("Gas price monitor started");
        
        // Get refresh interval from configuration
        let refresh_interval = Duration::from_secs(
            services.config.blockchain.gas_price_refresh_seconds,
        );
        
        let mut interval = interval(refresh_interval);
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = update_gas_price(blockchain_client.as_ref(), services.as_ref()).await {
                        warn!("Failed to update gas price: {}", e);
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal, stopping gas price monitor");
                    break;
                }
            }
        }
        
        info!("Gas price monitor stopped");
    })
}

/// Update the current gas price
async fn update_gas_price(blockchain_client: &BlockchainClient, services: &ServiceContext) -> Result<()> {
    let gas_price = blockchain_client.get_gas_price().await?;
    debug!("Updated gas price: {} gwei", gas_price.as_u64() / 1_000_000_000);
    
    // Update gas price in services
    services.transaction_service.update_gas_price(gas_price).await?;
    
    Ok(())
} 