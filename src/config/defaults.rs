use crate::config::*;

/// Generate default configuration
pub fn default_config() -> Config {
    Config {
        api: default_api_config(),
        database: default_database_config(),
        redis: default_redis_config(),
        blockchain: default_blockchain_config(),
        logging: default_logging_config(),
        services: default_services_config(),
    }
}

fn default_api_config() -> ApiConfig {
    ApiConfig {
        bind_address: "127.0.0.1:8080".to_string(),
        cors_allowed_origins: vec!["*".to_string()],
        request_timeout_seconds: 30,
        max_json_payload_size: 10 * 1024 * 1024, // 10 MB
    }
}

fn default_database_config() -> DatabaseConfig {
    DatabaseConfig {
        url: "postgres://postgres:postgres@localhost:5432/mev_capture".to_string(),
        max_connections: 20,
        idle_timeout_seconds: 300,
        connect_timeout_seconds: 10,
    }
}

fn default_redis_config() -> RedisConfig {
    RedisConfig {
        url: "redis://localhost:6379".to_string(),
        pool_size: 10,
    }
}

fn default_blockchain_config() -> BlockchainConfig {
    BlockchainConfig {
        rpc_url: "http://localhost:8545".to_string(),
        ws_url: "ws://localhost:8546".to_string(),
        chain_id: 1, // Ethereum Mainnet
        max_block_history: 100,
        confirmation_blocks: 12,
        gas_price_refresh_seconds: 10,
    }
}

fn default_logging_config() -> LoggingConfig {
    LoggingConfig {
        level: "info".to_string(),
        json_format: false,
        file_path: None,
    }
}

fn default_services_config() -> ServicesConfig {
    ServicesConfig {
        tx_ordering: default_tx_ordering_config(),
        block_building: default_block_building_config(),
        liquid_staking: default_liquid_staking_config(),
    }
}

fn default_tx_ordering_config() -> TxOrderingConfig {
    TxOrderingConfig {
        worker_threads: num_cpus::get(),
        max_simulation_time_ms: 100,
        simulation_mode: "optimistic".to_string(),
    }
}

fn default_block_building_config() -> BlockBuildingConfig {
    BlockBuildingConfig {
        target_block_fullness: 0.95,
        max_gas_limit: 30_000_000,
        priority_accounts: Vec::new(),
    }
}

fn default_liquid_staking_config() -> LiquidStakingConfig {
    LiquidStakingConfig {
        validator_commission_bps: 500, // 5%
        withdrawal_delay_epochs: 2,
        min_stake_amount: "0.1".to_string(), // 0.1 ETH
    }
} 