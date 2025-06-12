/// Logging and monitoring utilities for proof-of-storage continuity
///
/// This module provides structured logging with different levels for:
/// - Chain state progression
/// - Network operations  
/// - Performance metrics
/// - Error tracking
pub mod chain_state;
pub mod formatter;
pub mod network_logger;
pub mod performance;

// Re-export common types and functions
pub use chain_state::*;
pub use formatter::*;
pub use network_logger::*;
pub use performance::*;

use chrono::{DateTime, Utc};
use colored::*;
use log::{debug, error, info, warn};

/// Log levels for different components
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

/// Logger configuration
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    pub level: LogLevel,
    pub show_timestamps: bool,
    pub show_colors: bool,
    pub show_chain_state: bool,
    pub show_performance: bool,
    pub show_network: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            show_timestamps: true,
            show_colors: true,
            show_chain_state: true,
            show_performance: true,
            show_network: true,
        }
    }
}

/// Initialize the logging system
pub fn init_logger(config: Option<LoggerConfig>) -> Result<(), Box<dyn std::error::Error>> {
    let config = config.unwrap_or_default();

    // Set log level based on config
    let log_level = match config.level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
        LogLevel::Trace => "trace",
    };

    std::env::set_var("RUST_LOG", log_level);

    // Try to init, but ignore error if already initialized
    match env_logger::try_init() {
        Ok(_) => {
            info!("🚀 Proof-of-Storage Continuity Logger initialized");
            info!("📊 Log level: {}", log_level.to_uppercase());
        }
        Err(_) => {
            // Logger already initialized, that's fine
            debug!("Logger already initialized, skipping...");
        }
    }

    Ok(())
}

/// Format a timestamp for logging
pub fn format_timestamp() -> String {
    let now: DateTime<Utc> = Utc::now();
    now.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Log with appropriate color and formatting
pub fn log_with_color(level: LogLevel, emoji: &str, category: &str, message: &str) {
    let timestamp = format_timestamp();
    let formatted_message = format!("{} [{}] {}: {}", emoji, timestamp, category, message);

    match level {
        LogLevel::Error => error!("{}", formatted_message.red()),
        LogLevel::Warn => warn!("{}", formatted_message.yellow()),
        LogLevel::Info => info!("{}", formatted_message.green()),
        LogLevel::Debug => debug!("{}", formatted_message.blue()),
        LogLevel::Trace => debug!("{}", formatted_message.white()),
    }
}
