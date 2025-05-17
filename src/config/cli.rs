use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to configuration file
    #[arg(short, long, env = "CONFIG_FILE")]
    pub config: Option<String>,

    /// Log level (debug, info, warn, error)
    #[arg(short, long, env = "LOG_LEVEL")]
    pub log_level: Option<String>,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run migrations
    Migrate,
    
    /// Generate a default configuration file
    GenerateConfig {
        /// Output path for the generated config
        #[arg(short, long, default_value = "config/default.yaml")]
        output: String,
    },
    
    /// Validate configuration
    ValidateConfig {
        /// Path to configuration file
        #[arg(short, long)]
        config: String,
    },
}

/// Parse command line arguments
pub fn parse_args() -> Args {
    Args::parse()
} 