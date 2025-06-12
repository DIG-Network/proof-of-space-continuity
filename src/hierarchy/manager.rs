use crate::core::errors::HashChainResult;
use napi::bindgen_prelude::*;
use serde_json;
use std::collections::HashMap;

use crate::core::types::*;
use crate::hierarchy::{GroupManager, RegionManager};

/// Global state for all chains in the system
#[derive(Clone)]
pub struct GlobalChainState {
    /// Current block height being processed
    pub current_block_height: u64,
    /// Current global temporal proof
    pub global_temporal_proof: Buffer,
    /// Number of active chains
    pub active_chain_count: u32,
    /// Master chain hash combining all chains
    pub master_chain_hash: Buffer,
    /// Last update timestamp
    pub last_update_time: f64,
    /// Previous global proof for linkage
    pub previous_global_proof: Buffer,
}

impl Default for GlobalChainState {
    fn default() -> Self {
        Self {
            current_block_height: 0,
            global_temporal_proof: Buffer::from([0u8; 32].to_vec()),
            active_chain_count: 0,
            master_chain_hash: Buffer::from([0u8; 32].to_vec()),
            last_update_time: 0.0,
            previous_global_proof: Buffer::from([0u8; 32].to_vec()),
        }
    }
}

/// Chain registry for managing active chains
#[derive(Clone, Default)]
pub struct ChainRegistry {
    /// Active chains by chain_id
    pub chains: HashMap<ChainId, LightweightHashChain>,
    /// Current commitments by chain_id
    pub chain_commitments: HashMap<ChainId, Buffer>,
    /// Chain metadata
    pub chain_metadata: HashMap<ChainId, ChainMetadata>,
}

/// Metadata for a chain
#[derive(Clone)]
pub struct ChainMetadata {
    /// Public key of chain owner
    pub public_key: Buffer,
    /// When chain was added
    pub added_at_block: u64,
    /// File path
    pub data_file_path: String,
    /// Retention policy
    pub retention_policy: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Chain lifecycle record for tracking
#[derive(Clone)]
pub struct ChainLifecycleRecord {
    /// Chain identifier
    pub chain_id: ChainId,
    /// Owner's public key
    pub public_key: Buffer,
    /// Block when added
    pub added_at_block: u64,
    /// Timestamp when added
    pub added_at_time: f64,
    /// Retention policy
    pub retention_policy: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Block when removal requested
    pub removal_requested_at_block: Option<u64>,
    /// Block when removed
    pub removed_at_block: Option<u64>,
    /// Timestamp when removed
    pub removed_at_time: Option<f64>,
    /// Reason for removal
    pub removal_reason: Option<String>,
}

/// Retention policy definition
#[derive(Clone)]
pub struct RetentionPolicy {
    /// Days to retain
    pub days: u32,
}

/// Enhanced manager with hierarchical proof support for 100,000+ chains
pub struct HierarchicalGlobalChainManager {
    pub hierarchy_levels: u32,
    pub chains_per_group: u32,
    pub group_manager: GroupManager,
    pub region_manager: RegionManager,
    pub chain_registry: HashMap<Vec<u8>, LightweightHashChain>,
    pub active_chains: u32,
}

impl HierarchicalGlobalChainManager {
    pub fn new(hierarchy_levels: u32, chains_per_group: u32) -> Self {
        Self {
            hierarchy_levels,
            chains_per_group,
            group_manager: GroupManager::new(),
            region_manager: RegionManager::new(),
            chain_registry: HashMap::new(),
            active_chains: 0,
        }
    }

    pub fn add_chain(
        &mut self,
        data_file_path: String,
        public_key: Buffer,
        _retention_policy: Option<String>,
        _metadata: Option<HashMap<String, String>>,
    ) -> HashChainResult<HashMap<String, serde_json::Value>> {
        // Create a basic chain ID
        let chain_id = public_key.iter().take(16).cloned().collect::<Vec<u8>>();

        let chain = LightweightHashChain {
            chain_id: chain_id.clone(),
            public_key,
            data_file_path,
            total_chunks: 1000,
            current_commitment: None,
            chain_length: 0,
            initial_block_height: 0,
            initial_block_hash: Buffer::from([0u8; 32].to_vec()),
            file_encoding: None,
            availability_score: 1.0,
            latency_score: 1.0,
        };

        self.chain_registry.insert(chain_id.clone(), chain);
        self.active_chains += 1;

        let group_id = self.group_manager.assign_chain_to_group(chain_id.clone())?;
        let region_id = self
            .region_manager
            .assign_group_to_region(group_id.clone())?;

        let mut result = HashMap::new();
        result.insert("success".to_string(), serde_json::Value::Bool(true));
        result.insert(
            "chain_id".to_string(),
            serde_json::Value::String(hex::encode(&chain_id)),
        );
        result.insert("group_id".to_string(), serde_json::Value::String(group_id));
        result.insert(
            "region_id".to_string(),
            serde_json::Value::String(region_id),
        );
        Ok(result)
    }

    pub fn remove_chain(
        &mut self,
        chain_id: Vec<u8>,
        reason: Option<String>,
        archive_data: bool,
    ) -> HashChainResult<HashMap<String, serde_json::Value>> {
        if self.chain_registry.remove(&chain_id).is_some() {
            self.active_chains = self.active_chains.saturating_sub(1);
            self.group_manager.remove_chain_from_group(&chain_id)?;
        }

        let mut result = HashMap::new();
        result.insert("success".to_string(), serde_json::Value::Bool(true));
        result.insert(
            "chain_id".to_string(),
            serde_json::Value::String(hex::encode(&chain_id)),
        );
        result.insert(
            "archived".to_string(),
            serde_json::Value::Bool(archive_data),
        );
        result.insert(
            "reason".to_string(),
            serde_json::Value::String(reason.unwrap_or_else(|| "removed".to_string())),
        );
        Ok(result)
    }

    pub fn process_new_block_hierarchical(
        &mut self,
        block_hash: Buffer,
        block_height: u64,
    ) -> HashChainResult<()> {
        // Process block for all active chains
        let mut chains_to_update = Vec::new();

        // Collect chains that need updates
        for (chain_id, chain) in &self.chain_registry {
            if (chain.chain_length as u64) < block_height {
                chains_to_update.push(chain_id.clone());
            }
        }

        // Update each chain with new block
        for chain_id in chains_to_update {
            if let Some(chain) = self.chain_registry.get_mut(&chain_id) {
                // Update chain state for new block
                chain.chain_length = block_height as u32;

                // Generate new commitment for this block
                let commitment_data =
                    [&chain_id[..], &block_hash[..], &block_height.to_be_bytes()].concat();
                let commitment_hash = crate::core::utils::compute_sha256(&commitment_data);
                chain.current_commitment = Some(Buffer::from(commitment_hash.to_vec()));

                // Update group and region managers
                self.group_manager
                    .update_chain_commitment(&chain_id, &commitment_hash)?;
                let group_id = format!(
                    "group_{:06}",
                    chain_id.len() / self.chains_per_group as usize
                );
                self.region_manager
                    .update_group_proof(&group_id, block_height)?;
            }
        }

        Ok(())
    }

    pub fn get_statistics(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        stats.insert("active_chains".to_string(), self.active_chains as f64);
        stats.insert("totalChains".to_string(), self.active_chains as f64);
        stats.insert("total_chains".to_string(), self.active_chains as f64);
        stats.insert(
            "total_groups".to_string(),
            self.group_manager.groups.len() as f64,
        );
        stats.insert(
            "total_regions".to_string(),
            self.region_manager.regions.len() as f64,
        );
        stats
    }
}

impl Default for HierarchicalGlobalChainManager {
    fn default() -> Self {
        Self::new(3, CHAINS_PER_GROUP)
    }
}
