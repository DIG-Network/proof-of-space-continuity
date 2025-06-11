use log::{debug, info};
use napi::bindgen_prelude::*;
use std::collections::HashMap;

use crate::core::{
    errors::{HashChainError, HashChainResult},
    types::*,
    utils::{compute_merkle_root, compute_sha256, log_performance_metrics, PerformanceTimer},
};

/// Region of groups in hierarchy (Level 2)
#[derive(Clone)]
pub struct Region {
    /// Region identifier
    pub region_id: RegionId,
    /// Maximum groups in this region
    pub max_groups: u32,
    /// Group IDs in this region
    pub group_ids: Vec<GroupId>,
    /// Last computed regional proof
    pub last_regional_proof: Option<Buffer>,
    /// Last update block
    pub last_update_block: u64,
    /// Performance metrics
    pub performance_stats: HashMap<String, f64>,
}

impl Region {
    pub fn new(region_id: RegionId, max_groups: u32) -> Self {
        Self {
            region_id,
            max_groups,
            group_ids: Vec::new(),
            last_regional_proof: None,
            last_update_block: 0,
            performance_stats: HashMap::new(),
        }
    }

    /// Add a group to this region
    pub fn add_group(&mut self, group_id: GroupId) -> HashChainResult<()> {
        if self.is_full() {
            return Err(HashChainError::RegionFull {
                region_id: self.region_id.clone(),
                max_groups: self.max_groups,
            });
        }

        if !self.group_ids.contains(&group_id) {
            self.group_ids.push(group_id);
            debug!(
                "Added group to region {}: {} groups total",
                self.region_id,
                self.group_ids.len()
            );
        }

        Ok(())
    }

    /// Remove a group from this region
    pub fn remove_group(&mut self, group_id: &GroupId) {
        let initial_len = self.group_ids.len();
        self.group_ids.retain(|id| id != group_id);

        if self.group_ids.len() < initial_len {
            debug!(
                "Removed group from region {}: {} groups remaining",
                self.region_id,
                self.group_ids.len()
            );
        }
    }

    /// Check if region is full
    pub fn is_full(&self) -> bool {
        self.group_ids.len() >= self.max_groups as usize
    }

    /// Get current group count
    pub fn group_count(&self) -> u32 {
        self.group_ids.len() as u32
    }

    /// Compute regional proof for this region (Level 2)
    pub fn compute_regional_proof(
        &mut self,
        block_hash: &Buffer,
        group_proofs: &HashMap<GroupId, Buffer>,
        block_height: u64,
    ) -> HashChainResult<Buffer> {
        let timer = PerformanceTimer::new(&format!("regional_proof_{}", self.region_id));

        // Collect proofs for groups in this region
        let mut region_group_proofs = Vec::new();
        for group_id in &self.group_ids {
            if let Some(proof) = group_proofs.get(group_id) {
                region_group_proofs.push(proof.as_ref());
            } else {
                return Err(HashChainError::ChainNotFound {
                    chain_id: format!("group_{}", group_id),
                });
            }
        }

        if region_group_proofs.is_empty() {
            // Empty region gets zero hash
            let zero_proof = Buffer::from([0u8; 32].to_vec());
            self.last_regional_proof = Some(zero_proof.clone());
            self.last_update_block = block_height;
            return Ok(zero_proof);
        }

        // Build merkle tree of group proofs
        let region_merkle = compute_merkle_root(&region_group_proofs);

        // Compute regional proof with REGIONAL_ITERATIONS iterations
        let mut state = {
            let mut data = Vec::new();
            data.extend_from_slice(block_hash);
            data.extend_from_slice(&region_merkle);
            data.extend_from_slice(self.region_id.as_bytes());
            compute_sha256(&data)
        };

        for i in 0..REGIONAL_ITERATIONS {
            let mut iteration_data = Vec::new();
            iteration_data.extend_from_slice(&state);
            iteration_data.extend_from_slice(&i.to_be_bytes());
            state = compute_sha256(&iteration_data);
        }

        let regional_proof = Buffer::from(state.to_vec());

        // Update region state
        self.last_regional_proof = Some(regional_proof.clone());
        self.last_update_block = block_height;

        // Record performance
        let elapsed_ms = timer.elapsed_ms();
        self.performance_stats
            .insert("last_proof_time_ms".to_string(), elapsed_ms as f64);
        self.performance_stats
            .insert("groups_processed".to_string(), self.group_ids.len() as f64);

        log_performance_metrics(
            &format!("Region {} proof", self.region_id),
            self.group_ids.len() as u32,
            elapsed_ms,
            500, // 500ms target for regional proof
        );

        debug!(
            "Computed regional proof for {}: {} groups, {} iterations, {}ms",
            self.region_id,
            self.group_ids.len(),
            REGIONAL_ITERATIONS,
            elapsed_ms
        );

        Ok(regional_proof)
    }

    /// Verify regional proof
    pub fn verify_regional_proof(
        &self,
        block_hash: &Buffer,
        group_proofs: &HashMap<GroupId, Buffer>,
        expected_proof: &Buffer,
    ) -> HashChainResult<bool> {
        // Collect group proofs
        let mut region_group_proofs = Vec::new();
        for group_id in &self.group_ids {
            if let Some(proof) = group_proofs.get(group_id) {
                region_group_proofs.push(proof.as_ref());
            } else {
                return Ok(false);
            }
        }

        if region_group_proofs.is_empty() {
            // Empty region should have zero hash
            return Ok(expected_proof.as_ref() == &[0u8; 32]);
        }

        // Rebuild proof
        let region_merkle = compute_merkle_root(&region_group_proofs);

        let mut state = {
            let mut data = Vec::new();
            data.extend_from_slice(block_hash);
            data.extend_from_slice(&region_merkle);
            data.extend_from_slice(self.region_id.as_bytes());
            compute_sha256(&data)
        };

        for i in 0..REGIONAL_ITERATIONS {
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

    /// Check if region needs rebalancing
    pub fn needs_rebalancing(&self) -> bool {
        // Region is considered for rebalancing if it's less than 20% full
        // or more than 90% full (to allow for growth)
        let usage_ratio = self.group_ids.len() as f32 / self.max_groups as f32;
        usage_ratio < 0.2 || usage_ratio > 0.9
    }

    /// Get region metadata for monitoring
    pub fn get_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("region_id".to_string(), self.region_id.clone());
        metadata.insert("group_count".to_string(), self.group_ids.len().to_string());
        metadata.insert("max_groups".to_string(), self.max_groups.to_string());
        metadata.insert(
            "usage_percent".to_string(),
            format!(
                "{:.1}",
                (self.group_ids.len() as f32 / self.max_groups as f32) * 100.0
            ),
        );
        metadata.insert(
            "last_update_block".to_string(),
            self.last_update_block.to_string(),
        );
        metadata.insert(
            "has_proof".to_string(),
            self.last_regional_proof.is_some().to_string(),
        );
        metadata
    }
}

/// Region manager for handling multiple regions
pub struct RegionManager {
    /// Regions by region_id
    pub regions: HashMap<RegionId, Region>,
    /// Group to region mapping
    pub group_to_region: HashMap<GroupId, RegionId>,
    /// Next region counter for assignment
    pub next_region_counter: u32,
}

impl RegionManager {
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            group_to_region: HashMap::new(),
            next_region_counter: 0,
        }
    }

    /// Find or create a region for a new group
    pub fn assign_group_to_region(&mut self, group_id: GroupId) -> HashChainResult<RegionId> {
        // Check if group is already assigned
        if let Some(existing_region_id) = self.group_to_region.get(&group_id) {
            return Ok(existing_region_id.clone());
        }

        // Find a region with available space
        let available_region = self
            .regions
            .iter()
            .find(|(_, region)| !region.is_full())
            .map(|(region_id, _)| region_id.clone());

        let region_id = if let Some(region_id) = available_region {
            region_id
        } else {
            // Create new region
            let new_region_id = format!("region_{:03}", self.next_region_counter);
            let new_region = Region::new(new_region_id.clone(), GROUPS_PER_REGION);
            self.regions.insert(new_region_id.clone(), new_region);
            self.next_region_counter += 1;

            info!(
                "Created new region: {} (total regions: {})",
                new_region_id,
                self.regions.len()
            );
            new_region_id
        };

        // Add group to region
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.add_group(group_id.clone())?;
            self.group_to_region.insert(group_id, region_id.clone());
            Ok(region_id)
        } else {
            Err(HashChainError::GroupAssignment {
                reason: "Failed to access region after creation".to_string(),
            })
        }
    }

    /// Remove group from its region
    pub fn remove_group_from_region(&mut self, group_id: &GroupId) -> HashChainResult<()> {
        if let Some(region_id) = self.group_to_region.remove(group_id) {
            if let Some(region) = self.regions.get_mut(&region_id) {
                region.remove_group(group_id);

                // If region becomes empty, consider removing it
                if region.group_count() == 0 {
                    info!(
                        "Region {} is now empty, keeping for potential reuse",
                        region_id
                    );
                }
            }
        }
        Ok(())
    }

    /// Compute proofs for all regions in parallel
    pub fn compute_all_regional_proofs(
        &mut self,
        block_hash: &Buffer,
        group_proofs: &HashMap<GroupId, Buffer>,
        block_height: u64,
    ) -> HashChainResult<HashMap<RegionId, Buffer>> {
        use std::sync::{Arc, Mutex};

        let timer = PerformanceTimer::new("all_regional_proofs");
        let results = Arc::new(Mutex::new(HashMap::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Collect region data for parallel processing
        let region_data: Vec<(RegionId, Region)> = self
            .regions
            .iter()
            .map(|(id, region)| (id.clone(), region.clone()))
            .collect();

        // Process regions sequentially for now
        for (region_id, mut region) in region_data {
            match region.compute_regional_proof(block_hash, group_proofs, block_height) {
                Ok(proof) => {
                    let mut results_lock = results.lock().unwrap();
                    results_lock.insert(region_id.clone(), proof);
                }
                Err(e) => {
                    let mut errors_lock = errors.lock().unwrap();
                    errors_lock.push((region_id.clone(), e));
                }
            }
        }

        // Check for errors
        let errors_vec = errors.lock().unwrap();
        if !errors_vec.is_empty() {
            let first_error = &errors_vec[0];
            return Err(HashChainError::HierarchicalProofFailed {
                reason: format!("Region {} failed: {}", first_error.0, first_error.1),
            });
        }

        // Update regions with computed proofs
        let results_map = results.lock().unwrap();
        for (region_id, proof) in results_map.iter() {
            if let Some(region) = self.regions.get_mut(region_id) {
                region.last_regional_proof = Some(proof.clone());
                region.last_update_block = block_height;
            }
        }

        let elapsed_ms = timer.elapsed_ms();
        log_performance_metrics(
            "All regional proofs",
            self.regions.len() as u32,
            elapsed_ms,
            800, // 800ms target for all regional proofs
        );

        Ok(results_map.clone())
    }

    /// Get region statistics
    pub fn get_statistics(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();

        stats.insert("total_regions".to_string(), self.regions.len() as f64);
        stats.insert(
            "total_groups".to_string(),
            self.group_to_region.len() as f64,
        );

        let non_empty_regions = self
            .regions
            .values()
            .filter(|r| r.group_count() > 0)
            .count();
        stats.insert("non_empty_regions".to_string(), non_empty_regions as f64);

        let avg_groups_per_region = if self.regions.len() > 0 {
            self.group_to_region.len() as f64 / self.regions.len() as f64
        } else {
            0.0
        };
        stats.insert("avg_groups_per_region".to_string(), avg_groups_per_region);

        let full_regions = self.regions.values().filter(|r| r.is_full()).count();
        stats.insert("full_regions".to_string(), full_regions as f64);

        stats
    }
}

impl Default for RegionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let region = Region::new("test_region".to_string(), 10);
        assert_eq!(region.region_id, "test_region");
        assert_eq!(region.max_groups, 10);
        assert_eq!(region.group_count(), 0);
        assert!(!region.is_full());
    }

    #[test]
    fn test_region_add_remove() {
        let mut region = Region::new("test_region".to_string(), 2);
        let group1 = "group_001".to_string();
        let group2 = "group_002".to_string();
        let group3 = "group_003".to_string();

        // Add groups
        assert!(region.add_group(group1.clone()).is_ok());
        assert!(region.add_group(group2.clone()).is_ok());
        assert_eq!(region.group_count(), 2);
        assert!(region.is_full());

        // Try to add when full
        assert!(region.add_group(group3.clone()).is_err());

        // Remove group
        region.remove_group(&group1);
        assert_eq!(region.group_count(), 1);
        assert!(!region.is_full());
    }

    #[test]
    fn test_region_manager() {
        let mut manager = RegionManager::new();
        let group1 = "group_001".to_string();
        let group2 = "group_002".to_string();

        // Assign groups
        let region1 = manager.assign_group_to_region(group1.clone()).unwrap();
        let region2 = manager.assign_group_to_region(group2.clone()).unwrap();

        // Should be in same region initially
        assert_eq!(region1, region2);
        assert_eq!(manager.regions.len(), 1);
        assert_eq!(manager.group_to_region.len(), 2);
    }
}
