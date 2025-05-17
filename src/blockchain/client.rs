use anyhow::{anyhow, Context, Result};
use ethers::{
    abi::Address,
    prelude::*,
    providers::{Http, Middleware, Provider, PubsubClient, Ws},
    types::{
        Block, BlockNumber, Bytes, Filter, Transaction, TransactionReceipt, TransactionRequest, H256, U256,
    },
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::utils::metrics::MetricsTimer;

/// Client for interacting with the blockchain
pub struct BlockchainClient {
    /// HTTP provider for RPC calls
    http_provider: Provider<Http>,
    /// WebSocket provider for subscriptions
    ws_provider: Provider<Ws>,
    /// Chain ID
    chain_id: u64,
    /// Number of confirmations to wait for transactions
    confirmations: u64,
    /// Currently used gas price
    current_gas_price: AtomicU64,
    /// Cache for contract ABIs
    abi_cache: RwLock<HashMap<Address, ethers::abi::Contract>>,
}

impl BlockchainClient {
    /// Create a new blockchain client
    pub fn new(
        http_provider: Provider<Http>,
        ws_provider: Provider<Ws>,
        chain_id: u64,
        confirmations: u64,
    ) -> Self {
        Self {
            http_provider,
            ws_provider,
            chain_id,
            confirmations,
            current_gas_price: AtomicU64::new(0),
            abi_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get the current chain ID
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get the current block number
    pub async fn get_block_number(&self) -> Result<u64> {
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let block_number = self.http_provider.get_block_number().await?;
        timer.stop();
        
        Ok(block_number.as_u64())
    }

    /// Get block by number
    pub async fn get_block(&self, block_number: u64, with_txs: bool) -> Result<Option<Block<H256>>> {
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let block = self
            .http_provider
            .get_block(BlockNumber::Number(block_number.into()))
            .await?;
        timer.stop();
        
        Ok(block)
    }

    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: H256) -> Result<Option<Transaction>> {
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let tx = self.http_provider.get_transaction(tx_hash).await?;
        timer.stop();
        
        Ok(tx)
    }

    /// Get transaction receipt
    pub async fn get_transaction_receipt(&self, tx_hash: H256) -> Result<Option<TransactionReceipt>> {
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let receipt = self.http_provider.get_transaction_receipt(tx_hash).await?;
        timer.stop();
        
        Ok(receipt)
    }

    /// Send raw transaction
    pub async fn send_raw_transaction(&self, tx_bytes: Bytes) -> Result<H256> {
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let tx_hash = self.http_provider.send_raw_transaction(tx_bytes).await?;
        timer.stop();
        
        Ok(tx_hash)
    }

    /// Send transaction
    pub async fn send_transaction(&self, tx: TransactionRequest) -> Result<PendingTransaction<Http>> {
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let pending_tx = self.http_provider.send_transaction(tx, None).await?;
        timer.stop();
        
        Ok(pending_tx)
    }

    /// Wait for transaction to be confirmed
    pub async fn wait_for_transaction(&self, tx_hash: H256) -> Result<TransactionReceipt> {
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let receipt = self
            .http_provider
            .get_transaction_receipt(tx_hash)
            .await?
            .ok_or_else(|| anyhow!("Transaction receipt not found"))?;
        timer.stop();
        
        // Check confirmation count
        let current_block = self.get_block_number().await?;
        let tx_block = receipt.block_number.ok_or_else(|| anyhow!("Block number missing"))?.as_u64();
        
        if current_block < tx_block + self.confirmations {
            debug!(
                "Waiting for {} confirmations (current: {}, tx: {}, required: {})",
                self.confirmations,
                current_block,
                tx_block,
                tx_block + self.confirmations
            );
            
            let pending_tx = PendingTransaction::new(tx_hash, self.http_provider.clone());
            let receipt = pending_tx
                .confirmations(self.confirmations)
                .await?
                .ok_or_else(|| anyhow!("Transaction was dropped from mempool"))?;
            
            return Ok(receipt);
        }
        
        Ok(receipt)
    }

    /// Subscribe to new blocks
    pub async fn subscribe_blocks(&self) -> Result<ethers::providers::SubscriptionStream<Ws, Block<Transaction>>> {
        Ok(self.ws_provider.subscribe_blocks().await?)
    }

    /// Subscribe to pending transactions
    pub async fn subscribe_pending_txs(&self) -> Result<ethers::providers::SubscriptionStream<Ws, H256>> {
        Ok(self.ws_provider.subscribe_pending_txs().await?)
    }

    /// Get the current gas price
    pub async fn get_gas_price(&self) -> Result<U256> {
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let gas_price = self.http_provider.get_gas_price().await?;
        timer.stop();
        
        // Cache the gas price
        self.current_gas_price.store(gas_price.as_u64(), Ordering::Relaxed);
        
        Ok(gas_price)
    }

    /// Get the cached gas price, falling back to a fresh query if not available
    pub async fn get_cached_gas_price(&self) -> Result<U256> {
        let cached = self.current_gas_price.load(Ordering::Relaxed);
        if cached > 0 {
            return Ok(U256::from(cached));
        }
        
        self.get_gas_price().await
    }

    /// Call a contract function
    pub async fn call_contract<T: ethers::abi::Tokenize>(
        &self,
        address: Address,
        function_name: &str,
        args: T,
        block: Option<BlockNumber>,
    ) -> Result<ethers::abi::Bytes> {
        // Function call data
        let contract = self.get_contract(address).await?;
        let call_data = contract
            .function(function_name)
            .map_err(|e| anyhow!("Function not found: {}", e))?
            .encode_input(&args.into_tokens())
            .map_err(|e| anyhow!("Failed to encode function input: {}", e))?;
        
        // Create call request
        let tx = TransactionRequest {
            to: Some(ethers::types::NameOrAddress::Address(address)),
            data: Some(call_data.clone().into()),
            ..Default::default()
        };
        
        // Execute call
        let timer = MetricsTimer::new("blockchain_request_duration_seconds");
        let result = self.http_provider.call(&tx, block).await?;
        timer.stop();
        
        Ok(result)
    }

    /// Get a contract instance with ABI
    async fn get_contract(&self, address: Address) -> Result<ethers::abi::Contract> {
        // Check cache first
        {
            let cache = self.abi_cache.read().await;
            if let Some(contract) = cache.get(&address) {
                return Ok(contract.clone());
            }
        }
        
        // If not in cache, fetch the ABI
        // In a real implementation, this would use a registry or fetch from Etherscan
        // For this example, we'll just create a dummy ABI
        let contract = ethers::abi::Contract::load(
            &[] as &[u8], // This would be the real ABI in production
        )
        .map_err(|e| anyhow!("Failed to load contract ABI: {}", e))?;
        
        // Cache the contract
        {
            let mut cache = self.abi_cache.write().await;
            cache.insert(address, contract.clone());
        }
        
        Ok(contract)
    }
} 