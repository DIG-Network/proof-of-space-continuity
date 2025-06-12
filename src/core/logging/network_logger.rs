/// Network Operations Logging
/// 
/// This module provides logging for network operations including:
/// - Peer registration and management
/// - Availability challenges
/// - Network consensus operations
/// - Blockchain data validation

use super::*;
use chrono::{DateTime, Utc};
use colored::*;
use log::{info, debug, warn, error};
use napi::bindgen_prelude::*;
use std::collections::HashMap;

/// Network operation categories for logging
#[derive(Debug, Clone, Copy)]
pub enum NetworkOperation {
    PeerRegistration,
    PeerUpdate,
    PeerRemoval,
    AvailabilityChallenge,
    ChallengeResponse,
    BlockchainValidation,
    ConsensusOperation,
    DataValidation,
}

impl NetworkOperation {
    fn emoji(&self) -> &'static str {
        match self {
            NetworkOperation::PeerRegistration => "📝",
            NetworkOperation::PeerUpdate => "📈",
            NetworkOperation::PeerRemoval => "🗑️",
            NetworkOperation::AvailabilityChallenge => "🎯",
            NetworkOperation::ChallengeResponse => "⚔️",
            NetworkOperation::BlockchainValidation => "✅",
            NetworkOperation::ConsensusOperation => "🤝",
            NetworkOperation::DataValidation => "🔍",
        }
    }

    fn category(&self) -> &'static str {
        match self {
            NetworkOperation::PeerRegistration => "PEER_REG",
            NetworkOperation::PeerUpdate => "PEER_UPD",
            NetworkOperation::PeerRemoval => "PEER_REM",
            NetworkOperation::AvailabilityChallenge => "AVAIL_CHAL",
            NetworkOperation::ChallengeResponse => "CHAL_RESP",
            NetworkOperation::BlockchainValidation => "BLOCKCHAIN",
            NetworkOperation::ConsensusOperation => "CONSENSUS",
            NetworkOperation::DataValidation => "DATA_VAL",
        }
    }
}

/// Network logger for tracking network operations
pub struct NetworkLogger {
    config: LoggerConfig,
    start_time: DateTime<Utc>,
    operation_count: HashMap<String, u64>,
}

impl NetworkLogger {
    /// Create a new network logger
    pub fn new(config: LoggerConfig) -> Self {
        Self {
            config,
            start_time: Utc::now(),
            operation_count: HashMap::new(),
        }
    }

    /// Log peer registration
    pub fn log_peer_registration(&mut self, peer_id: &Buffer, node_type: &str, success: bool) {
        if !self.config.show_network {
            return;
        }

        self.increment_operation_count("peer_registration");
        let peer_hex = hex::encode(&peer_id);
        let peer_id_short = if peer_hex.len() > 16 { &peer_hex[..16] } else { &peer_hex }.to_string();
        
        if success {
            info!("{} Peer registered: {}... as {}", 
                  NetworkOperation::PeerRegistration.emoji().bright_green(),
                  peer_id_short.bright_cyan(),
                  node_type.bright_white());
        } else {
            error!("{} Failed to register peer: {}... as {}", 
                   NetworkOperation::PeerRegistration.emoji().bright_red(),
                   peer_id_short.bright_cyan(),
                   node_type.bright_white());
        }
    }

    /// Log peer information request
    pub fn log_peer_info_request(&mut self, peer_id: &Buffer, success: bool) {
        if !self.config.show_network {
            return;
        }

        let peer_hex = hex::encode(&peer_id);
        let peer_id_short = if peer_hex.len() > 8 { &peer_hex[..8] } else { &peer_hex }.to_string();
        
        if success {
            debug!("{} Getting peer info for {}...", 
                   "📊".bright_blue(),
                   peer_id_short.bright_cyan());
        } else {
            warn!("{} Failed to get peer info for {}...", 
                  "📊".bright_yellow(),
                  peer_id_short.bright_cyan());
        }
    }

    /// Log peer latency update
    pub fn log_peer_latency_update(&mut self, peer_id: &Buffer, latency_ms: f64, success: bool) {
        if !self.config.show_network {
            return;
        }

        self.increment_operation_count("peer_latency_update");
        let peer_hex = hex::encode(&peer_id);
        let peer_id_short = if peer_hex.len() > 8 { &peer_hex[..8] } else { &peer_hex }.to_string();
        
        if success {
            debug!("{} Updated latency for peer {}...: {}ms", 
                   NetworkOperation::PeerUpdate.emoji().bright_blue(),
                   peer_id_short.bright_cyan(),
                   latency_ms.to_string().bright_yellow());
        } else {
            warn!("{} Failed to update latency for peer {}...: {}ms", 
                  NetworkOperation::PeerUpdate.emoji().bright_yellow(),
                  peer_id_short.bright_cyan(),
                  latency_ms.to_string().bright_yellow());
        }
    }

    /// Log peer removal
    pub fn log_peer_removal(&mut self, peer_id: &Buffer, success: bool) {
        if !self.config.show_network {
            return;
        }

        self.increment_operation_count("peer_removal");
        let peer_hex = hex::encode(&peer_id);
        let peer_id_short = if peer_hex.len() > 8 { &peer_hex[..8] } else { &peer_hex }.to_string();
        
        if success {
            info!("{} Removed peer: {}...", 
                  NetworkOperation::PeerRemoval.emoji().bright_green(),
                  peer_id_short.bright_cyan());
        } else {
            error!("{} Failed to remove peer: {}...", 
                   NetworkOperation::PeerRemoval.emoji().bright_red(),
                   peer_id_short.bright_cyan());
        }
    }

    /// Log availability challenge issued
    pub fn log_availability_challenge_issued(&mut self, target_prover: &Buffer, challenge_id: &str) {
        if !self.config.show_network {
            return;
        }

        self.increment_operation_count("availability_challenge");
        let prover_hex = hex::encode(&target_prover);
        let prover_short = if prover_hex.len() > 8 { &prover_hex[..8] } else { &prover_hex }.to_string();
        let challenge_short = if challenge_id.len() > 16 { &challenge_id[..16] } else { challenge_id };
        
        info!("{} Availability challenge issued to prover {}... (Challenge: {}...)", 
              NetworkOperation::AvailabilityChallenge.emoji().bright_green(),
              prover_short.bright_cyan(),
              challenge_short.bright_yellow());
    }

    /// Log availability challenge response
    pub fn log_availability_challenge_response(&mut self, challenge_id: &Buffer, success: bool, response_time_ms: Option<f64>) {
        if !self.config.show_network {
            return;
        }

        self.increment_operation_count("challenge_response");
        let challenge_hex = hex::encode(&challenge_id);
        let challenge_short = if challenge_hex.len() > 16 { &challenge_hex[..16] } else { &challenge_hex }.to_string();
        
        if success {
            let time_info = if let Some(time) = response_time_ms {
                format!(" in {}ms", time.to_string().bright_yellow())
            } else {
                String::new()
            };
            
            info!("{} Challenge response successful: {}...{}", 
                  NetworkOperation::ChallengeResponse.emoji().bright_green(),
                  challenge_short.bright_cyan(),
                  time_info);
        } else {
            error!("{} Challenge response failed: {}...", 
                   NetworkOperation::ChallengeResponse.emoji().bright_red(),
                   challenge_short.bright_cyan());
        }
    }

    /// Log blockchain data validation
    pub fn log_blockchain_validation(&mut self, operation: &str, data_hash: &Buffer, success: bool) {
        if !self.config.show_network {
            return;
        }

        self.increment_operation_count("blockchain_validation");
        let data_hex = hex::encode(&data_hash);
        let data_short = if data_hex.len() > 16 { &data_hex[..16] } else { &data_hex }.to_string();
        
        if success {
            info!("{} Blockchain validation: {} for data {}...", 
                  NetworkOperation::BlockchainValidation.emoji().bright_green(),
                  operation.bright_white(),
                  data_short.bright_cyan());
        } else {
            error!("{} Blockchain validation failed: {} for data {}...", 
                   NetworkOperation::BlockchainValidation.emoji().bright_red(),
                   operation.bright_white(),
                   data_short.bright_cyan());
        }
    }

    /// Log chunk count validation
    pub fn log_chunk_count_validation(&mut self, file_hash: &Buffer, reported_chunks: u32, valid: bool) {
        if !self.config.show_network {
            return;
        }

        let file_hex = hex::encode(&file_hash);
        let file_short = if file_hex.len() > 16 { &file_hex[..16] } else { &file_hex }.to_string();
        
        if valid {
            debug!("{} Chunk count validation: {} chunks for file {}...", 
                   NetworkOperation::DataValidation.emoji().bright_blue(),
                   reported_chunks.to_string().bright_yellow(),
                   file_short.bright_cyan());
        } else {
            warn!("{} Invalid chunk count: {} chunks for file {}...", 
                  NetworkOperation::DataValidation.emoji().bright_yellow(),
                  reported_chunks.to_string().bright_yellow(),
                  file_short.bright_cyan());
        }
    }

    /// Log consensus operation
    pub fn log_consensus_operation(&mut self, operation: &str, success: bool, details: Option<&str>) {
        if !self.config.show_network {
            return;
        }

        self.increment_operation_count("consensus_operation");
        
        let detail_str = if let Some(details) = details {
            format!(" - {}", details.bright_white())
        } else {
            String::new()
        };

        if success {
            info!("{} Consensus operation: {}{}", 
                  NetworkOperation::ConsensusOperation.emoji().bright_green(),
                  operation.bright_white(),
                  detail_str);
        } else {
            error!("{} Consensus operation failed: {}{}", 
                   NetworkOperation::ConsensusOperation.emoji().bright_red(),
                   operation.bright_white(),
                   detail_str);
        }
    }

    /// Log active peers request
    pub fn log_active_peers_request(&mut self, peer_count: usize) {
        if !self.config.show_network {
            return;
        }

        debug!("{} Getting active peers... (Found: {})", 
               "👥".bright_blue(),
               peer_count.to_string().bright_green());
    }

    /// Log network announcement
    pub fn log_network_announcement(&mut self, announcement_type: &str, success: bool) {
        if !self.config.show_network {
            return;
        }

        if success {
            info!("{} Network announcement: {}", 
                  "📢".bright_green(),
                  announcement_type.bright_white());
        } else {
            error!("{} Network announcement failed: {}", 
                   "📢".bright_red(),
                   announcement_type.bright_white());
        }
    }

    /// Log proof broadcast
    pub fn log_proof_broadcast(&mut self, proof_type: &str, proof_size: usize, success: bool) {
        if !self.config.show_network {
            return;
        }

        if success {
            info!("{} Proof broadcast: {} ({} bytes)", 
                  "📡".bright_green(),
                  proof_type.bright_white(),
                  proof_size.to_string().bright_yellow());
        } else {
            error!("{} Proof broadcast failed: {} ({} bytes)", 
                   "📡".bright_red(),
                   proof_type.bright_white(),
                   proof_size.to_string().bright_yellow());
        }
    }

    /// Log prover reputation check
    pub fn log_prover_reputation(&mut self, prover_key: &Buffer, reputation: f64) {
        if !self.config.show_network {
            return;
        }

        let prover_hex = hex::encode(&prover_key);
        let prover_short = if prover_hex.len() > 8 { &prover_hex[..8] } else { &prover_hex }.to_string();
        let reputation_color = if reputation >= 0.9 {
            reputation.to_string().bright_green()
        } else if reputation >= 0.7 {
            reputation.to_string().bright_yellow()
        } else {
            reputation.to_string().bright_red()
        };

        debug!("{} Prover reputation: {}... - {}", 
               "⭐".bright_blue(),
               prover_short.bright_cyan(),
               reputation_color);
    }

    /// Log storage statistics
    pub fn log_storage_stats(&mut self, total_chunks: u32, total_size: u64, available_space: u64) {
        if !self.config.show_network {
            return;
        }

        debug!("{} Storage statistics: {} chunks, {} bytes stored, {} bytes available", 
               "💾".bright_blue(),
               total_chunks.to_string().bright_yellow(),
               total_size.to_string().bright_cyan(),
               available_space.to_string().bright_green());
    }

    /// Log network statistics
    pub fn log_network_stats(&self) {
        if !self.config.show_network {
            return;
        }

        let duration = Utc::now().signed_duration_since(self.start_time);
        
        info!("");
        info!("{}", "🌐 === NETWORK STATISTICS ===".bright_blue().bold());
        
        for (operation, count) in &self.operation_count {
            let ops_per_second = if duration.num_seconds() > 0 {
                *count as f64 / duration.num_seconds() as f64
            } else {
                0.0
            };
            
            info!("{} {}: {} total ({:.2}/s)", 
                  "📊".bright_blue(),
                  operation.replace("_", " ").to_uppercase().bright_white(),
                  count.to_string().bright_green(),
                  ops_per_second.to_string().bright_cyan());
        }
        
        info!("{} Runtime: {}s", 
              "⏱️".bright_blue(),
              duration.num_seconds().to_string().bright_yellow());
        info!("{}", "=".repeat(50).bright_blue());
        info!("");
    }

    /// Increment operation counter
    fn increment_operation_count(&mut self, operation: &str) {
        *self.operation_count.entry(operation.to_string()).or_insert(0) += 1;
    }

    /// Get operation statistics
    pub fn get_operation_stats(&self) -> HashMap<String, u64> {
        self.operation_count.clone()
    }
} 