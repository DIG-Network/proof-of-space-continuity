/// Chain State Logging and Tracking
/// 
/// This module provides detailed logging and visualization of chain state changes
/// as the proof-of-storage chain evolves over time

use super::*;
use crate::core::types::*;
use chrono::{DateTime, Utc};
use colored::*;
use log::{info, debug};
use napi::bindgen_prelude::*;
use serde_json::json;
use std::collections::HashMap;

/// Chain state tracker that maintains the current state of all chains
#[derive(Clone)]
pub struct ChainStateTracker {
    pub block_height: u64,
    pub chains: HashMap<String, Vec<ChainCommitment>>,
    pub config: LoggerConfig,
    pub start_time: DateTime<Utc>,
}

/// Individual chain commitment with enhanced tracking
#[derive(Clone)]
pub struct ChainCommitment {
    pub prover_key: Buffer,
    pub chain_id: Buffer,
    pub data_hash: Buffer,
    pub block_height: u64,
    pub block_hash: Buffer,
    pub file_hashes: Vec<FileHashInfo>,
    pub chunk_hashes: Vec<Buffer>,
    pub vdf_proof: MemoryHardVDFProof,
    pub entropy: MultiSourceEntropy,
    pub commitment_hash: Buffer,
    pub prev_commitment_hash: Option<Buffer>,
    pub timestamp: DateTime<Utc>,
}

/// File hash information for tracking individual files
#[derive(Clone)]
pub struct FileHashInfo {
    pub name: String,
    pub hash: Buffer,
    pub size: u64,
}

impl ChainStateTracker {
    /// Create a new chain state tracker
    pub fn new(config: LoggerConfig) -> Self {
        Self {
            block_height: 0,
            chains: HashMap::new(),
            config,
            start_time: Utc::now(),
        }
    }

    /// Add a new commitment to a chain
    pub fn add_chain_commitment(&mut self, chain_id: Buffer, commitment: ChainCommitment) {
        let chain_id_hex = hex::encode(&chain_id);
        
        if !self.chains.contains_key(&chain_id_hex) {
            self.chains.insert(chain_id_hex.clone(), Vec::new());
        }
        
        let show_chain_state = self.config.show_chain_state;
        if let Some(chain) = self.chains.get_mut(&chain_id_hex) {
            chain.push(commitment.clone());
            let chain_length = chain.len();
            
            if show_chain_state {
                self.log_commitment_added(&chain_id_hex, &commitment, chain_length);
            }
        }
    }

    /// Get chain by ID
    pub fn get_chain(&self, chain_id: &Buffer) -> Option<&Vec<ChainCommitment>> {
        let chain_id_hex = hex::encode(chain_id);
        self.chains.get(&chain_id_hex)
    }

    /// Get chain length
    pub fn get_chain_length(&self, chain_id: &Buffer) -> usize {
        let chain_id_hex = hex::encode(chain_id);
        self.chains.get(&chain_id_hex).map(|c| c.len()).unwrap_or(0)
    }

    /// Get all chains
    pub fn get_all_chains(&self) -> Vec<(Buffer, &Vec<ChainCommitment>)> {
        self.chains
            .iter()
            .map(|(chain_id_hex, commitments)| {
                let chain_id = Buffer::from(hex::decode(chain_id_hex).unwrap_or_default());
                (chain_id, commitments)
            })
            .collect()
    }

    /// Increment block height and log progress
    pub fn increment_block_height(&mut self) -> u64 {
        self.block_height += 1;
        
        if self.config.show_chain_state {
            self.log_block_progress();
        }
        
        self.block_height
    }

    /// Display complete chain state
    pub fn display_chain_state(&self, chain_id: &str, chain: &[ChainCommitment]) {
        if !self.config.show_chain_state {
            return;
        }

        let duration = Utc::now().signed_duration_since(self.start_time);
        
        info!("");
        info!("{}", "📊 === CURRENT CHAIN STATE ===".bright_blue().bold());
        let chain_id_display = if chain_id.len() > 16 {
            format!("{}...", &chain_id[..16])
        } else {
            chain_id.to_string()
        };
        info!("{} {}", "Chain ID:".bright_white(), chain_id_display.bright_cyan());
        info!("{} {} blocks", "Current Length:".bright_white(), chain.len().to_string().bright_green());
        info!("{} {}", "Current Height:".bright_white(), self.block_height.to_string().bright_green());
        info!("{} {} seconds", "Runtime:".bright_white(), duration.num_seconds().to_string().bright_yellow());
        info!("");
        info!("{}", "Latest Block:".bright_white().bold());
        
        if let Some(latest_commitment) = chain.last() {
            let latest_index = chain.len() - 1;
            self.display_commitment_details(latest_commitment, latest_index, chain.len());
        } else {
            info!("  No blocks in chain yet");
        }
        info!("{}", "=".repeat(80).bright_blue());
        info!("");
    }

    /// Display detailed commitment information
    fn display_commitment_details(&self, commitment: &ChainCommitment, index: usize, _total: usize) {
        let connector = if index == 0 { "" } else { "↓" };
        
        info!("");
        if !connector.is_empty() {
            info!("  {}", connector.bright_blue());
        }
        info!("  {} Block {}:", "".bright_white(), (index + 1).to_string().bright_yellow().bold());
        
        // Block Info
        info!("  ├─ {}:", "Block Info".bright_white().bold());
        info!("  │  ├─ Height: {}", commitment.block_height.to_string().bright_cyan());
        info!("  │  ├─ Block Hash: {}...", 
              hex::encode(&commitment.block_hash)[..16].bright_cyan());
        info!("  │  ├─ Commitment Hash: {}...", 
              hex::encode(&commitment.commitment_hash)[..16].bright_cyan());
        if let Some(prev_hash) = &commitment.prev_commitment_hash {
            info!("  │  └─ Previous Hash: {}...", 
                  hex::encode(prev_hash)[..16].bright_cyan());
        }

        // Data Info
        info!("  ├─ {}:", "Data".bright_white().bold());
        info!("  │  ├─ Combined Data Hash: {}...", 
              hex::encode(&commitment.data_hash)[..16].bright_cyan());
        info!("  │  └─ Files:");
        for file in &commitment.file_hashes {
            info!("  │     ├─ {}", file.name.bright_green());
            info!("  │     │  ├─ Size: {} bytes", file.size.to_string().bright_yellow());
            info!("  │     │  └─ Hash: {}...", hex::encode(&file.hash)[..16].bright_cyan());
        }

        // VDF Proof
        info!("  ├─ {}:", "VDF Proof".bright_white().bold());
        info!("  │  ├─ Input State: {}...", 
              hex::encode(&commitment.vdf_proof.input_state)[..16].bright_cyan());
        info!("  │  ├─ Output State: {}...", 
              hex::encode(&commitment.vdf_proof.output_state)[..16].bright_cyan());
        info!("  │  ├─ Iterations: {}", commitment.vdf_proof.iterations.to_string().bright_yellow());
        info!("  │  ├─ Computation Time: {}ms", 
              commitment.vdf_proof.computation_time_ms.to_string().bright_yellow());
        info!("  │  └─ Memory Usage: {:.2}MB", 
              (commitment.vdf_proof.memory_usage_bytes / (1024.0 * 1024.0)).to_string().bright_yellow());

        // Entropy Sources
        info!("  ├─ {}:", "Entropy Sources".bright_white().bold());
        info!("  │  ├─ Blockchain: {}...", 
              hex::encode(&commitment.entropy.blockchain_entropy)[..16].bright_cyan());
        info!("  │  ├─ Local: {}...", 
              hex::encode(&commitment.entropy.local_entropy)[..16].bright_cyan());
        info!("  │  ├─ Timestamp: {}", 
              commitment.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string().bright_yellow());
        info!("  │  └─ Combined: {}...", 
              hex::encode(&commitment.entropy.combined_hash)[..16].bright_cyan());

        // Chunk Proofs
        info!("  ├─ {}:", "Chunk Proofs".bright_white().bold());
        info!("  │  ├─ Total Chunks: {}", commitment.chunk_hashes.len().to_string().bright_yellow());
        info!("  │  └─ Chunk Hashes:");
        let display_count = std::cmp::min(3, commitment.chunk_hashes.len());
        for i in 0..display_count {
            info!("  │     ├─ Chunk {}: {}...", 
                  i, hex::encode(&commitment.chunk_hashes[i])[..16].bright_cyan());
        }
        if commitment.chunk_hashes.len() > 3 {
            info!("  │     └─ ... and {} more chunks", 
                  (commitment.chunk_hashes.len() - 3).to_string().bright_yellow());
        }

        // Availability Challenge (simulated)
        let challenge_id = format!("challenge_{}", commitment.block_height);
        info!("  └─ {}:", "Availability Challenge".bright_white().bold());
        let challenge_display = if challenge_id.len() > 16 {
            format!("{}...", &challenge_id[..16])
        } else {
            challenge_id.clone()
        };
        info!("     ├─ Challenge ID: {}", challenge_display.bright_cyan());
        info!("     ├─ Challenged Chunks: {}", "0, 5, 10".bright_yellow());
        info!("     ├─ Timestamp: {}", 
              commitment.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string().bright_yellow());
        info!("     └─ Deadline: {}", 
              (commitment.timestamp + chrono::Duration::minutes(1))
                  .format("%Y-%m-%d %H:%M:%S UTC").to_string().bright_yellow());
    }

    /// Log when a commitment is added
    fn log_commitment_added(&self, chain_id: &str, commitment: &ChainCommitment, chain_length: usize) {
        let short_chain_id = if chain_id.len() > 16 {
            &chain_id[..16]
        } else {
            chain_id
        };
        let short_commitment = hex::encode(&commitment.commitment_hash)[..16].to_string();
        
        info!("{} New commitment added to chain {}... - Block {} (Length: {})", 
              "✅".bright_green(),
              short_chain_id.bright_cyan(),
              commitment.block_height.to_string().bright_yellow(),
              chain_length.to_string().bright_green());
        
        debug!("{} Commitment hash: {}...", 
               "🔗".bright_blue(),
               short_commitment.bright_cyan());
        
        debug!("{} Files: [{}]", 
               "📁".bright_blue(),
               commitment.file_hashes.iter()
                   .map(|f| f.name.clone())
                   .collect::<Vec<_>>()
                   .join(", ")
                   .bright_white());
    }

    /// Log block progress
    fn log_block_progress(&self) {
        let total_chains = self.chains.len();
        let total_commitments: usize = self.chains.values().map(|c| c.len()).sum();
        let duration = Utc::now().signed_duration_since(self.start_time);
        
        info!("{} Block {} completed - {} chains, {} total commitments (Runtime: {}s)", 
              "📦".bright_blue(),
              self.block_height.to_string().bright_yellow(),
              total_chains.to_string().bright_green(),
              total_commitments.to_string().bright_green(),
              duration.num_seconds().to_string().bright_yellow());
    }

    /// Get summary statistics
    pub fn get_statistics(&self) -> serde_json::Value {
        let total_chains = self.chains.len();
        let total_commitments: usize = self.chains.values().map(|c| c.len()).sum();
        let duration = Utc::now().signed_duration_since(self.start_time);
        
        let avg_chain_length = if total_chains > 0 {
            total_commitments as f64 / total_chains as f64
        } else {
            0.0
        };

        json!({
            "block_height": self.block_height,
            "total_chains": total_chains,
            "total_commitments": total_commitments,
            "average_chain_length": avg_chain_length,
            "runtime_seconds": duration.num_seconds(),
            "commitments_per_second": if duration.num_seconds() > 0 {
                total_commitments as f64 / duration.num_seconds() as f64
            } else {
                0.0
            }
        })
    }

    /// Log statistics summary
    pub fn log_statistics(&self) {
        let stats = self.get_statistics();
        
        info!("");
        info!("{}", "📊 === CHAIN STATISTICS ===".bright_blue().bold());
        info!("{} {}", "Current Block Height:".bright_white(), 
              stats["block_height"].to_string().bright_yellow());
        info!("{} {}", "Total Chains:".bright_white(), 
              stats["total_chains"].to_string().bright_green());
        info!("{} {}", "Total Commitments:".bright_white(), 
              stats["total_commitments"].to_string().bright_green());
        info!("{} {:.2}", "Average Chain Length:".bright_white(), 
              stats["average_chain_length"].as_f64().unwrap_or(0.0).to_string().bright_cyan());
        info!("{} {}s", "Runtime:".bright_white(), 
              stats["runtime_seconds"].to_string().bright_yellow());
        info!("{} {:.2}/s", "Commitments Per Second:".bright_white(), 
              stats["commitments_per_second"].as_f64().unwrap_or(0.0).to_string().bright_cyan());
        info!("{}", "=".repeat(50).bright_blue());
        info!("");
    }
} 