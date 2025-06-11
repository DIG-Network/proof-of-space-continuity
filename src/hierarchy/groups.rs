use log::{debug, info};
use napi::bindgen_prelude::*;
use std::collections::HashMap;

use crate::core::{
    errors::{HashChainError, HashChainResult},
    types::*,
    utils::{compute_merkle_root, compute_sha256, log_performance_metrics, PerformanceTimer},
};

/// Group of chains in hierarchy (Level 1)
#[derive(Clone)]
pub struct ChainGroup {
    /// Group identifier
    pub group_id: GroupId,
    /// Maximum chains in this group
    pub max_chains: u32,
    /// Chain IDs in this group
    pub chain_ids: Vec<ChainId>,
    /// Last computed group proof
    pub last_group_proof: Option<Buffer>,
    /// Last update block
    pub last_update_block: u64,
    /// Performance metrics
    pub performance_stats: HashMap<String, f64>,
}

impl ChainGroup {
    pub fn new(group_id: GroupId, max_chains: u32) -> Self {
        Self {
            group_id,
            max_chains,
            chain_ids: Vec::new(),
            last_group_proof: None,
            last_update_block: 0,
            performance_stats: HashMap::new(),
        }
    }

    /// Add a chain to this group
    pub fn add_chain(&mut self, chain_id: ChainId) -> HashChainResult<()> {
        if self.is_full() {
            return Err(HashChainError::GroupFull {
                group_id: self.group_id.clone(),
                max_chains: self.max_chains,
            });
        }

        if !self.chain_ids.contains(&chain_id) {
            self.chain_ids.push(chain_id);
            debug!(
                "Added chain to group {}: {} chains total",
                self.group_id,
                self.chain_ids.len()
            );
        }

        Ok(())
    }

    /// Remove a chain from this group
    pub fn remove_chain(&mut self, chain_id: &ChainId) {
        let initial_len = self.chain_ids.len();
        self.chain_ids.retain(|id| id != chain_id);

        if self.chain_ids.len() < initial_len {
            debug!(
                "Removed chain from group {}: {} chains remaining",
                self.group_id,
                self.chain_ids.len()
            );
        }
    }

    /// Check if group is full
    pub fn is_full(&self) -> bool {
        self.chain_ids.len() >= self.max_chains as usize
    }

    /// Get current chain count
    pub fn chain_count(&self) -> u32 {
        self.chain_ids.len() as u32
    }

    /// Compute group proof for this group (Level 1)
    pub fn compute_group_proof(
        &mut self,
        block_hash: &Buffer,
        chain_commitments: &HashMap<ChainId, Buffer>,
        block_height: u64,
    ) -> HashChainResult<Buffer> {
        let timer = PerformanceTimer::new(&format!("group_proof_{}", self.group_id));

        // Collect commitments for chains in this group
        let mut group_commitments = Vec::new();
        for chain_id in &self.chain_ids {
            if let Some(commitment) = chain_commitments.get(chain_id) {
                group_commitments.push(commitment.as_ref());
            } else {
                return Err(HashChainError::ChainNotFound {
                    chain_id: hex::encode(chain_id),
                });
            }
        }

        if group_commitments.is_empty() {
            // Empty group gets zero hash
            let zero_proof = Buffer::from([0u8; 32].to_vec());
            self.last_group_proof = Some(zero_proof.clone());
            self.last_update_block = block_height;
            return Ok(zero_proof);
        }

        // Build merkle tree of commitments
        let group_merkle = compute_merkle_root(&group_commitments);

        // Compute group proof with GROUP_ITERATIONS iterations
        let mut state = {
            let mut data = Vec::new();
            data.extend_from_slice(block_hash);
            data.extend_from_slice(&group_merkle);
            data.extend_from_slice(self.group_id.as_bytes());
            compute_sha256(&data)
        };

        for i in 0..GROUP_ITERATIONS {
            let mut iteration_data = Vec::new();
            iteration_data.extend_from_slice(&state);
            iteration_data.extend_from_slice(&i.to_be_bytes());
            state = compute_sha256(&iteration_data);
        }

        let group_proof = Buffer::from(state.to_vec());

        // Update group state
        self.last_group_proof = Some(group_proof.clone());
        self.last_update_block = block_height;

        // Record performance
        let elapsed_ms = timer.elapsed_ms();
        self.performance_stats
            .insert("last_proof_time_ms".to_string(), elapsed_ms as f64);
        self.performance_stats
            .insert("chains_processed".to_string(), self.chain_ids.len() as f64);

        log_performance_metrics(
            &format!("Group {} proof", self.group_id),
            self.chain_ids.len() as u32,
            elapsed_ms,
            100, // 100ms target for group proof
        );

        debug!(
            "Computed group proof for {}: {} chains, {} iterations, {}ms",
            self.group_id,
            self.chain_ids.len(),
            GROUP_ITERATIONS,
            elapsed_ms
        );

        Ok(group_proof)
    }

    /// Verify group proof
    pub fn verify_group_proof(
        &self,
        block_hash: &Buffer,
        chain_commitments: &HashMap<ChainId, Buffer>,
        expected_proof: &Buffer,
    ) -> HashChainResult<bool> {
        // Collect commitments
        let mut group_commitments = Vec::new();
        for chain_id in &self.chain_ids {
            if let Some(commitment) = chain_commitments.get(chain_id) {
                group_commitments.push(commitment.as_ref());
            } else {
                return Ok(false);
            }
        }

        if group_commitments.is_empty() {
            // Empty group should have zero hash
            return Ok(expected_proof.as_ref() == &[0u8; 32]);
        }

        // Rebuild proof
        let group_merkle = compute_merkle_root(&group_commitments);

        let mut state = {
            let mut data = Vec::new();
            data.extend_from_slice(block_hash);
            data.extend_from_slice(&group_merkle);
            data.extend_from_slice(self.group_id.as_bytes());
            compute_sha256(&data)
        };

        for i in 0..GROUP_ITERATIONS {
            let mut iteration_data = Vec::new();
            iteration_data.extend_from_slice(&state);
            iteration_data.extend_from_slice(&i.to_be_bytes());
            state = compute_sha256(&iteration_data);
        }

        Ok(state.as_ref() == expected_proof.as_ref())
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> HashMap<String, f64> {
        self.performance_stats.clone()
    }

    /// Reset performance statistics
    pub fn reset_performance_stats(&mut self) {
        self.performance_stats.clear();
    }

    /// Check if group needs rebalancing
    pub fn needs_rebalancing(&self) -> bool {
        // Group is considered for rebalancing if it's less than 10% full
        // or more than 95% full (to allow for growth)
        let usage_ratio = self.chain_ids.len() as f32 / self.max_chains as f32;
        usage_ratio < 0.1 || usage_ratio > 0.95
    }

    /// Get group metadata for monitoring
    pub fn get_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("group_id".to_string(), self.group_id.clone());
        metadata.insert("chain_count".to_string(), self.chain_ids.len().to_string());
        metadata.insert("max_chains".to_string(), self.max_chains.to_string());
        metadata.insert(
            "usage_percent".to_string(),
            format!(
                "{:.1}",
                (self.chain_ids.len() as f32 / self.max_chains as f32) * 100.0
            ),
        );
        metadata.insert(
            "last_update_block".to_string(),
            self.last_update_block.to_string(),
        );
        metadata.insert(
            "has_proof".to_string(),
            self.last_group_proof.is_some().to_string(),
        );
        metadata
    }
}

/// Group manager for handling multiple groups
pub struct GroupManager {
    /// Groups by group_id
    pub groups: HashMap<GroupId, ChainGroup>,
    /// Chain to group mapping
    pub chain_to_group: HashMap<ChainId, GroupId>,
    /// Next group counter for assignment
    pub next_group_counter: u32,
}

impl GroupManager {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            chain_to_group: HashMap::new(),
            next_group_counter: 0,
        }
    }

    /// Find or create a group for a new chain
    pub fn assign_chain_to_group(&mut self, chain_id: ChainId) -> HashChainResult<GroupId> {
        // Check if chain is already assigned
        if let Some(existing_group_id) = self.chain_to_group.get(&chain_id) {
            return Ok(existing_group_id.clone());
        }

        // Find a group with available space
        let available_group = self
            .groups
            .iter()
            .find(|(_, group)| !group.is_full())
            .map(|(group_id, _)| group_id.clone());

        let group_id = if let Some(group_id) = available_group {
            group_id
        } else {
            // Create new group
            let new_group_id = format!("group_{:06}", self.next_group_counter);
            let new_group = ChainGroup::new(new_group_id.clone(), CHAINS_PER_GROUP);
            self.groups.insert(new_group_id.clone(), new_group);
            self.next_group_counter += 1;

            info!(
                "Created new group: {} (total groups: {})",
                new_group_id,
                self.groups.len()
            );
            new_group_id
        };

        // Add chain to group
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.add_chain(chain_id.clone())?;
            self.chain_to_group.insert(chain_id, group_id.clone());
            Ok(group_id)
        } else {
            Err(HashChainError::GroupAssignment {
                reason: "Failed to access group after creation".to_string(),
            })
        }
    }

    /// Remove chain from its group
    pub fn remove_chain_from_group(&mut self, chain_id: &ChainId) -> HashChainResult<()> {
        if let Some(group_id) = self.chain_to_group.remove(chain_id) {
            if let Some(group) = self.groups.get_mut(&group_id) {
                group.remove_chain(chain_id);

                // If group becomes empty, consider removing it
                if group.chain_count() == 0 {
                    info!(
                        "Group {} is now empty, keeping for potential reuse",
                        group_id
                    );
                }
            }
        }
        Ok(())
    }

    /// Compute proofs for all groups in parallel
    pub fn compute_all_group_proofs(
        &mut self,
        block_hash: &Buffer,
        chain_commitments: &HashMap<ChainId, Buffer>,
        block_height: u64,
    ) -> HashChainResult<HashMap<GroupId, Buffer>> {
        use std::sync::{Arc, Mutex};

        let timer = PerformanceTimer::new("all_group_proofs");
        let results = Arc::new(Mutex::new(HashMap::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Collect group data for parallel processing
        let group_data: Vec<(GroupId, ChainGroup)> = self
            .groups
            .iter()
            .map(|(id, group)| (id.clone(), group.clone()))
            .collect();

        // Process groups sequentially for now
        for (group_id, mut group) in group_data {
            match group.compute_group_proof(block_hash, chain_commitments, block_height) {
                Ok(proof) => {
                    let mut results_lock = results.lock().unwrap();
                    results_lock.insert(group_id.clone(), proof);
                }
                Err(e) => {
                    let mut errors_lock = errors.lock().unwrap();
                    errors_lock.push((group_id.clone(), e));
                }
            }
        }

        // Check for errors
        let errors_vec = errors.lock().unwrap();
        if !errors_vec.is_empty() {
            let first_error = &errors_vec[0];
            return Err(HashChainError::HierarchicalProofFailed {
                reason: format!("Group {} failed: {}", first_error.0, first_error.1),
            });
        }

        // Update groups with computed proofs
        let results_map = results.lock().unwrap();
        for (group_id, proof) in results_map.iter() {
            if let Some(group) = self.groups.get_mut(group_id) {
                group.last_group_proof = Some(proof.clone());
                group.last_update_block = block_height;
            }
        }

        let elapsed_ms = timer.elapsed_ms();
        log_performance_metrics(
            "All group proofs",
            self.groups.len() as u32,
            elapsed_ms,
            200, // 200ms target for all group proofs
        );

        Ok(results_map.clone())
    }

    /// Get group statistics
    pub fn get_statistics(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();

        stats.insert("total_groups".to_string(), self.groups.len() as f64);
        stats.insert("total_chains".to_string(), self.chain_to_group.len() as f64);

        let non_empty_groups = self.groups.values().filter(|g| g.chain_count() > 0).count();
        stats.insert("non_empty_groups".to_string(), non_empty_groups as f64);

        let avg_chains_per_group = if self.groups.len() > 0 {
            self.chain_to_group.len() as f64 / self.groups.len() as f64
        } else {
            0.0
        };
        stats.insert("avg_chains_per_group".to_string(), avg_chains_per_group);

        let full_groups = self.groups.values().filter(|g| g.is_full()).count();
        stats.insert("full_groups".to_string(), full_groups as f64);

        stats
    }
}

impl Default for GroupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_group_creation() {
        let group = ChainGroup::new("test_group".to_string(), 10);
        assert_eq!(group.group_id, "test_group");
        assert_eq!(group.max_chains, 10);
        assert_eq!(group.chain_count(), 0);
        assert!(!group.is_full());
    }

    #[test]
    fn test_chain_group_add_remove() {
        let mut group = ChainGroup::new("test_group".to_string(), 2);
        let chain1 = vec![1u8; 32];
        let chain2 = vec![2u8; 32];
        let chain3 = vec![3u8; 32];

        // Add chains
        assert!(group.add_chain(chain1.clone()).is_ok());
        assert!(group.add_chain(chain2.clone()).is_ok());
        assert_eq!(group.chain_count(), 2);
        assert!(group.is_full());

        // Try to add when full
        assert!(group.add_chain(chain3.clone()).is_err());

        // Remove chain
        group.remove_chain(&chain1);
        assert_eq!(group.chain_count(), 1);
        assert!(!group.is_full());
    }

    #[test]
    fn test_group_manager() {
        let mut manager = GroupManager::new();
        let chain1 = vec![1u8; 32];
        let chain2 = vec![2u8; 32];

        // Assign chains
        let group1 = manager.assign_chain_to_group(chain1.clone()).unwrap();
        let group2 = manager.assign_chain_to_group(chain2.clone()).unwrap();

        // Should be in same group initially
        assert_eq!(group1, group2);
        assert_eq!(manager.groups.len(), 1);
        assert_eq!(manager.chain_to_group.len(), 2);
    }
}
