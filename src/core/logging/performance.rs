/// Performance Logging and Metrics
///
/// This module provides logging for performance metrics including:
/// - Execution timing
/// - Memory usage tracking
/// - Throughput measurements
/// - VDF computation monitoring
use super::*;
use chrono::{DateTime, Utc};
use colored::*;
use log::info;
use std::time::Instant;

/// Performance operation categories
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum PerformanceCategory {
    VdfComputation,
    ChunkSelection,
    MerkleTreeGeneration,
    HashChainCreation,
    BlockProcessing,
    ProofGeneration,
    ProofVerification,
    NetworkOperation,
    DataStorage,
    DataRetrieval,
}

impl PerformanceCategory {
    fn emoji(&self) -> &'static str {
        match self {
            PerformanceCategory::VdfComputation => "🧮",
            PerformanceCategory::ChunkSelection => "🎯",
            PerformanceCategory::MerkleTreeGeneration => "🌳",
            PerformanceCategory::HashChainCreation => "🔗",
            PerformanceCategory::BlockProcessing => "📦",
            PerformanceCategory::ProofGeneration => "🛡️",
            PerformanceCategory::ProofVerification => "✅",
            PerformanceCategory::NetworkOperation => "🌐",
            PerformanceCategory::DataStorage => "💾",
            PerformanceCategory::DataRetrieval => "📁",
        }
    }

    fn category_name(&self) -> &'static str {
        match self {
            PerformanceCategory::VdfComputation => "VDF_COMPUTE",
            PerformanceCategory::ChunkSelection => "CHUNK_SELECT",
            PerformanceCategory::MerkleTreeGeneration => "MERKLE_TREE",
            PerformanceCategory::HashChainCreation => "HASHCHAIN",
            PerformanceCategory::BlockProcessing => "BLOCK_PROC",
            PerformanceCategory::ProofGeneration => "PROOF_GEN",
            PerformanceCategory::ProofVerification => "PROOF_VER",
            PerformanceCategory::NetworkOperation => "NETWORK",
            PerformanceCategory::DataStorage => "STORAGE",
            PerformanceCategory::DataRetrieval => "RETRIEVAL",
        }
    }
}

/// Performance timer for measuring operation duration
#[derive(Debug)]
pub struct ProofTimer {
    start_time: Instant,
    operation_name: String,
}

impl ProofTimer {
    pub fn new(operation_name: &str) -> Self {
        Self {
            start_time: Instant::now(),
            operation_name: operation_name.to_string(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    pub fn finish(self) -> u64 {
        let elapsed_ms = self.elapsed_ms();
        info!(
            "⚡ {}: {}ms",
            self.operation_name.bright_white(),
            elapsed_ms.to_string().bright_yellow()
        );
        elapsed_ms
    }
}

/// Performance logger for tracking metrics
pub struct ProofPerformanceLogger {
    config: LoggerConfig,
    start_time: DateTime<Utc>,
}

impl ProofPerformanceLogger {
    pub fn new(config: LoggerConfig) -> Self {
        Self {
            config,
            start_time: Utc::now(),
        }
    }

    pub fn log_vdf_performance(&self, iterations: u32, computation_time_ms: f64) {
        if !self.config.show_performance {
            return;
        }

        info!(
            "🧮 VDF Performance: {} iterations in {}ms",
            iterations.to_string().bright_yellow(),
            computation_time_ms.to_string().bright_yellow()
        );
    }

    pub fn log_storage_performance(
        &self,
        operation: &str,
        data_size_bytes: u64,
        operation_time_ms: u64,
    ) {
        if !self.config.show_performance {
            return;
        }

        let data_size_mb = data_size_bytes as f64 / (1024.0 * 1024.0);
        let throughput_mbps = if operation_time_ms > 0 {
            let time_seconds = operation_time_ms as f64 / 1000.0;
            data_size_mb / time_seconds
        } else {
            0.0
        };

        info!(
            "💾 Storage {}: {:.2}MB in {}ms ({:.2} MB/s)",
            operation.bright_white(),
            data_size_mb.to_string().bright_cyan(),
            operation_time_ms.to_string().bright_yellow(),
            throughput_mbps.to_string().bright_cyan()
        );
    }

    /// Get total session uptime since logger creation
    pub fn get_session_uptime_ms(&self) -> i64 {
        (Utc::now() - self.start_time).num_milliseconds()
    }

    /// Log session summary with uptime
    pub fn log_session_summary(&self, operations_count: u32) {
        if !self.config.show_performance {
            return;
        }

        let uptime_ms = self.get_session_uptime_ms();
        let uptime_seconds = uptime_ms as f64 / 1000.0;

        info!(
            "📊 Session Summary: {} operations in {:.2}s ({:.2} ops/s)",
            operations_count.to_string().bright_yellow(),
            uptime_seconds.to_string().bright_cyan(),
            if uptime_seconds > 0.0 {
                operations_count as f64 / uptime_seconds
            } else {
                0.0
            }
            .to_string()
            .bright_cyan()
        );
    }

    /// Log categorized performance metric
    pub fn log_categorized_operation(
        &self,
        category: PerformanceCategory,
        operation_name: &str,
        duration_ms: u64,
    ) {
        if !self.config.show_performance {
            return;
        }

        info!(
            "{} {}: {} in {}ms",
            category.emoji(),
            category.category_name().bright_white(),
            operation_name.bright_white(),
            duration_ms.to_string().bright_yellow()
        );
    }
}
