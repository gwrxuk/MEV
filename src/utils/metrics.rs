use anyhow::Result;
use metrics::{counter, gauge, histogram};
use prometheus::{Encoder, Registry, TextEncoder};
use std::time::{Duration, Instant};

/// Register all application metrics
pub fn register_metrics() {
    // Transaction metrics
    register_transaction_metrics();
    
    // Block building metrics
    register_block_metrics();
    
    // API metrics
    register_api_metrics();
    
    // Database metrics
    register_database_metrics();
    
    // Blockchain client metrics
    register_blockchain_metrics();
}

fn register_transaction_metrics() {
    // Transaction counts
    counter!("transactions_received_total", "Total number of transactions received");
    counter!("transactions_processed_total", "Total number of transactions processed");
    counter!("transactions_dropped_total", "Total number of transactions dropped");
    
    // Transaction timing
    histogram!("transaction_processing_time_seconds", "Time to process a transaction");
    histogram!("transaction_simulation_time_seconds", "Time to simulate a transaction");
}

fn register_block_metrics() {
    // Block metrics
    counter!("blocks_built_total", "Total number of blocks built");
    counter!("blocks_submitted_total", "Total number of blocks submitted");
    counter!("blocks_accepted_total", "Total number of blocks accepted by the network");
    
    // Block timing and size
    histogram!("block_building_time_seconds", "Time to build a block");
    gauge!("block_fullness_ratio", "Ratio of block gas used to gas limit");
    histogram!("block_profit_eth", "Profit extracted per block in ETH");
}

fn register_api_metrics() {
    // API request metrics
    counter!("api_requests_total", "Total number of API requests");
    counter!("api_errors_total", "Total number of API errors");
    
    // API timing
    histogram!("api_request_duration_seconds", "API request duration in seconds");
}

fn register_database_metrics() {
    // Database metrics
    gauge!("db_connections_active", "Number of active database connections");
    counter!("db_queries_total", "Total number of database queries");
    histogram!("db_query_duration_seconds", "Database query duration in seconds");
}

fn register_blockchain_metrics() {
    // Blockchain client metrics
    counter!("blockchain_requests_total", "Total number of blockchain client requests");
    counter!("blockchain_errors_total", "Total number of blockchain client errors");
    gauge!("blockchain_current_block", "Current blockchain block height");
    histogram!("blockchain_request_duration_seconds", "Blockchain request duration in seconds");
}

/// Returns current metrics in Prometheus format
pub fn get_prometheus_metrics() -> Result<String> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    
    Ok(String::from_utf8(buffer)?)
}

/// Timer utility for measuring and recording performance metrics
pub struct MetricsTimer {
    name: &'static str,
    start: Instant,
}

impl MetricsTimer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }
    
    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();
        histogram!(self.name, duration.as_secs_f64());
        duration
    }
} 