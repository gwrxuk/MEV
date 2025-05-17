use anyhow::Result;
use std::{sync::Arc, time::Instant};

use crate::{
    blockchain::BlockchainClient,
    config::Config,
    database::{DbPool, RedisPool},
};

pub mod block_building;
pub mod transaction;
pub mod liquid_staking;
pub mod simulation;

use block_building::BlockBuildingService;
use liquid_staking::LiquidStakingService;
use transaction::TransactionService;
use simulation::SimulationService;

/// Service context containing all services
pub struct ServiceContext {
    /// PostgreSQL database pool
    pub db_pool: DbPool,
    /// Redis connection manager
    pub redis: RedisPool,
    /// Blockchain client
    pub blockchain_client: Arc<BlockchainClient>,
    /// Application configuration
    pub config: Config,
    /// Application start time
    pub start_time: Instant,
    /// Transaction service
    pub transaction_service: TransactionService,
    /// Block building service
    pub block_building_service: BlockBuildingService,
    /// Liquid staking service
    pub liquid_staking_service: LiquidStakingService,
    /// Simulation service
    pub simulation_service: SimulationService,
}

impl ServiceContext {
    /// Create a new service context
    pub async fn new(
        db_pool: DbPool,
        redis: RedisPool,
        blockchain_client: Arc<BlockchainClient>,
        config: &Config,
    ) -> Result<Self> {
        // Initialize services
        let simulation_service = SimulationService::new(
            blockchain_client.clone(),
            config.services.tx_ordering.clone(),
        )?;
        
        let transaction_service = TransactionService::new(
            db_pool.clone(),
            blockchain_client.clone(),
            simulation_service.clone(),
        )?;
        
        let block_building_service = BlockBuildingService::new(
            db_pool.clone(),
            blockchain_client.clone(),
            transaction_service.clone(),
            config.services.block_building.clone(),
        )?;
        
        let liquid_staking_service = LiquidStakingService::new(
            db_pool.clone(),
            blockchain_client.clone(),
            config.services.liquid_staking.clone(),
        )?;
        
        Ok(Self {
            db_pool,
            redis,
            blockchain_client,
            config: config.clone(),
            start_time: Instant::now(),
            transaction_service,
            block_building_service,
            liquid_staking_service,
            simulation_service,
        })
    }
    
    /// Gracefully shutdown all services
    pub async fn shutdown(&self) -> Result<()> {
        // Shutdown services in order
        self.transaction_service.shutdown().await?;
        self.block_building_service.shutdown().await?;
        self.liquid_staking_service.shutdown().await?;
        self.simulation_service.shutdown().await?;
        
        Ok(())
    }
} 