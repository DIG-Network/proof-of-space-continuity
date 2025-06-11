use log::{debug, info};
use napi::bindgen_prelude::*;
use std::collections::HashMap;

use crate::core::{
    errors::{HashChainError, HashChainResult},
    types::*,
    utils::{
        compute_merkle_root, compute_sha256, get_current_timestamp, log_performance_metrics,
        PerformanceTimer,
    },
};
use crate::hierarchy::{GroupManager, RegionManager};

/// Result of hierarchical proof computation
#[derive(Clone)]
pub struct HierarchicalProofResult {
    /// Global root proof
    pub global_root_proof: Buffer,
    /// Group proofs by group_id
    pub group_proofs: HashMap<GroupId, Buffer>,
    /// Regional proofs by region_id
    pub regional_proofs: HashMap<RegionId, Buffer>,
    /// Performance statistics
    pub stats: HashMap<String, f64>,
    /// When computed
    pub computed_at: f64,
}

/// Hierarchical proof computation engine
pub struct HierarchicalGlobalProof {
    /// Maximum chains per group
    pub max_chains_per_group: u32,
    /// Maximum groups per region
    pub max_groups_per_region: u32,
}

impl HierarchicalGlobalProof {
    pub fn new(max_chains_per_group: u32, max_groups_per_region: u32) -> Self {
        Self {
            max_chains_per_group,
            max_groups_per_region,
        }
    }

    /// Compute hierarchical global proof for massive chain counts
    /// Parallelizable at each level except the final root
    pub fn compute_hierarchical_proof(
        &self,
        block_hash: &Buffer,
        all_chain_commitments: &HashMap<ChainId, Buffer>,
        previous_global_proof: &Buffer,
    ) -> HashChainResult<HierarchicalProofResult> {
        let start_time = get_current_timestamp();
        let mut stats = HashMap::new();

        stats.insert(
            "total_chains".to_string(),
            all_chain_commitments.len() as f64,
        );

        info!(
            "Starting hierarchical proof computation for {} chains",
            all_chain_commitments.len()
        );

        // Step 1: Organize chains into groups
        let timer = PerformanceTimer::new("organize_groups");
        let chain_groups = self.organize_into_groups(all_chain_commitments)?;
        let organize_time = timer.elapsed_ms();

        stats.insert("total_groups".to_string(), chain_groups.len() as f64);
        stats.insert("organize_time_ms".to_string(), organize_time as f64);

        // Step 2: Compute group proofs in parallel
        let timer = PerformanceTimer::new("group_proofs");
        let group_proofs = self.compute_all_group_proofs_parallel(
            &chain_groups,
            block_hash,
            all_chain_commitments,
        )?;
        let group_compute_time = timer.elapsed_ms();

        stats.insert("groups_processed".to_string(), group_proofs.len() as f64);
        stats.insert(
            "group_compute_time_ms".to_string(),
            group_compute_time as f64,
        );

        // Step 3: Organize groups into regions
        let timer = PerformanceTimer::new("organize_regions");
        let group_regions = self.organize_groups_into_regions(&group_proofs)?;
        let region_organize_time = timer.elapsed_ms();

        stats.insert("total_regions".to_string(), group_regions.len() as f64);
        stats.insert(
            "region_organize_time_ms".to_string(),
            region_organize_time as f64,
        );

        // Step 4: Compute regional proofs in parallel
        let timer = PerformanceTimer::new("regional_proofs");
        let regional_proofs =
            self.compute_all_regional_proofs_parallel(&group_regions, block_hash, &group_proofs)?;
        let region_compute_time = timer.elapsed_ms();

        stats.insert(
            "regions_processed".to_string(),
            regional_proofs.len() as f64,
        );
        stats.insert(
            "region_compute_time_ms".to_string(),
            region_compute_time as f64,
        );

        // Step 5: Compute global root proof (sequential)
        let timer = PerformanceTimer::new("global_root");
        let global_root_proof =
            self.compute_global_root_proof(&regional_proofs, block_hash, previous_global_proof)?;
        let root_compute_time = timer.elapsed_ms();

        stats.insert("root_compute_time_ms".to_string(), root_compute_time as f64);

        // Calculate total time and performance metrics
        let total_time = get_current_timestamp() - start_time;
        let total_time_ms = (total_time * 1000.0) as u32;

        stats.insert("total_time_ms".to_string(), total_time_ms as f64);

        // Estimate sequential time (rough calculation)
        let sequential_estimate = all_chain_commitments.len() as u32 * 1; // 1ms per chain
        stats.insert(
            "sequential_estimate_ms".to_string(),
            sequential_estimate as f64,
        );

        let speedup_factor = if total_time_ms > 0 {
            sequential_estimate as f64 / total_time_ms as f64
        } else {
            1.0
        };
        stats.insert("speedup_factor".to_string(), speedup_factor);

        // Log comprehensive performance report
        self.log_performance_report(&stats);

        Ok(HierarchicalProofResult {
            global_root_proof,
            group_proofs,
            regional_proofs,
            stats,
            computed_at: get_current_timestamp(),
        })
    }

    /// Organize chains into groups
    fn organize_into_groups(
        &self,
        chain_commitments: &HashMap<ChainId, Buffer>,
    ) -> HashChainResult<HashMap<GroupId, Vec<(ChainId, Buffer)>>> {
        let mut groups: HashMap<GroupId, Vec<(ChainId, Buffer)>> = HashMap::new();
        let mut current_group_index = 0;
        let mut current_group_size = 0;

        for (chain_id, commitment) in chain_commitments {
            let group_id = format!("group_{:06}", current_group_index);

            let group_chains = groups.entry(group_id).or_insert_with(Vec::new);
            group_chains.push((chain_id.clone(), commitment.clone()));
            current_group_size += 1;

            if current_group_size >= self.max_chains_per_group {
                current_group_index += 1;
                current_group_size = 0;
            }
        }

        debug!(
            "Organized {} chains into {} groups",
            chain_commitments.len(),
            groups.len()
        );

        Ok(groups)
    }

    /// Compute group proofs sequentially
    fn compute_all_group_proofs_parallel(
        &self,
        chain_groups: &HashMap<GroupId, Vec<(ChainId, Buffer)>>,
        block_hash: &Buffer,
        _all_chain_commitments: &HashMap<ChainId, Buffer>,
    ) -> HashChainResult<HashMap<GroupId, Buffer>> {
        let mut results = HashMap::new();

        // Process groups sequentially for now
        for (group_id, group_chains) in chain_groups {
            match self.compute_group_proof(group_id, group_chains, block_hash) {
                Ok(proof) => {
                    results.insert(group_id.clone(), proof);
                }
                Err(e) => {
                    return Err(HashChainError::HierarchicalProofFailed {
                        reason: format!("Group {} failed: {}", group_id, e),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Compute proof for a single group (Level 1)
    fn compute_group_proof(
        &self,
        group_id: &GroupId,
        group_chains: &[(ChainId, Buffer)],
        block_hash: &Buffer,
    ) -> HashChainResult<Buffer> {
        if group_chains.is_empty() {
            return Ok(Buffer::from([0u8; 32].to_vec()));
        }

        // Collect commitments
        let commitments: Vec<&[u8]> = group_chains
            .iter()
            .map(|(_, commitment)| commitment.as_ref())
            .collect();

        // Build merkle tree
        let group_merkle = compute_merkle_root(&commitments);

        // Compute group proof with GROUP_ITERATIONS
        let mut state = {
            let mut data = Vec::new();
            data.extend_from_slice(block_hash);
            data.extend_from_slice(&group_merkle);
            data.extend_from_slice(group_id.as_bytes());
            compute_sha256(&data)
        };

        for i in 0..GROUP_ITERATIONS {
            let mut iteration_data = Vec::new();
            iteration_data.extend_from_slice(&state);
            iteration_data.extend_from_slice(&i.to_be_bytes());
            state = compute_sha256(&iteration_data);
        }

        Ok(Buffer::from(state.to_vec()))
    }

    /// Organize groups into regions
    fn organize_groups_into_regions(
        &self,
        group_proofs: &HashMap<GroupId, Buffer>,
    ) -> HashChainResult<HashMap<RegionId, Vec<(GroupId, Buffer)>>> {
        let mut regions: HashMap<RegionId, Vec<(GroupId, Buffer)>> = HashMap::new();
        let mut current_region_index = 0;
        let mut current_region_size = 0;

        for (group_id, proof) in group_proofs {
            let region_id = format!("region_{:03}", current_region_index);

            let region_groups = regions.entry(region_id).or_insert_with(Vec::new);
            region_groups.push((group_id.clone(), proof.clone()));
            current_region_size += 1;

            if current_region_size >= self.max_groups_per_region {
                current_region_index += 1;
                current_region_size = 0;
            }
        }

        debug!(
            "Organized {} groups into {} regions",
            group_proofs.len(),
            regions.len()
        );

        Ok(regions)
    }

    /// Compute regional proofs sequentially
    fn compute_all_regional_proofs_parallel(
        &self,
        group_regions: &HashMap<RegionId, Vec<(GroupId, Buffer)>>,
        block_hash: &Buffer,
        _group_proofs: &HashMap<GroupId, Buffer>,
    ) -> HashChainResult<HashMap<RegionId, Buffer>> {
        let mut results = HashMap::new();

        // Process regions sequentially for now
        for (region_id, region_groups) in group_regions {
            match self.compute_regional_proof(region_id, region_groups, block_hash) {
                Ok(proof) => {
                    results.insert(region_id.clone(), proof);
                }
                Err(e) => {
                    return Err(HashChainError::HierarchicalProofFailed {
                        reason: format!("Region {} failed: {}", region_id, e),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Compute proof for a region (Level 2)
    fn compute_regional_proof(
        &self,
        region_id: &RegionId,
        region_groups: &[(GroupId, Buffer)],
        block_hash: &Buffer,
    ) -> HashChainResult<Buffer> {
        if region_groups.is_empty() {
            return Ok(Buffer::from([0u8; 32].to_vec()));
        }

        // Collect group proofs
        let group_proofs: Vec<&[u8]> = region_groups
            .iter()
            .map(|(_, proof)| proof.as_ref())
            .collect();

        // Build merkle tree
        let region_merkle = compute_merkle_root(&group_proofs);

        // Compute regional proof with REGIONAL_ITERATIONS
        let mut state = {
            let mut data = Vec::new();
            data.extend_from_slice(block_hash);
            data.extend_from_slice(&region_merkle);
            data.extend_from_slice(region_id.as_bytes());
            compute_sha256(&data)
        };

        for i in 0..REGIONAL_ITERATIONS {
            let mut iteration_data = Vec::new();
            iteration_data.extend_from_slice(&state);
            iteration_data.extend_from_slice(&i.to_be_bytes());
            state = compute_sha256(&iteration_data);
        }

        Ok(Buffer::from(state.to_vec()))
    }

    /// Compute final global root proof (Level 3)
    fn compute_global_root_proof(
        &self,
        regional_proofs: &HashMap<RegionId, Buffer>,
        block_hash: &Buffer,
        previous_global_proof: &Buffer,
    ) -> HashChainResult<Buffer> {
        if regional_proofs.is_empty() {
            return Ok(Buffer::from([0u8; 32].to_vec()));
        }

        // Collect all regional proofs
        let all_regional_proofs: Vec<&[u8]> = regional_proofs
            .values()
            .map(|proof| proof.as_ref())
            .collect();

        // Build merkle tree of regional proofs
        let global_merkle = compute_merkle_root(&all_regional_proofs);

        // Compute global root proof with GLOBAL_ROOT_ITERATIONS
        let mut state = {
            let mut data = Vec::new();
            data.extend_from_slice(block_hash);
            data.extend_from_slice(&global_merkle);
            data.extend_from_slice(previous_global_proof);
            data.extend_from_slice(&(regional_proofs.len() as u32).to_be_bytes());
            compute_sha256(&data)
        };

        for i in 0..GLOBAL_ROOT_ITERATIONS {
            let mut iteration_data = Vec::new();
            iteration_data.extend_from_slice(&state);
            iteration_data.extend_from_slice(&i.to_be_bytes());
            state = compute_sha256(&iteration_data);
        }

        Ok(Buffer::from(state.to_vec()))
    }

    /// Log comprehensive performance report
    fn log_performance_report(&self, stats: &HashMap<String, f64>) {
        let total_chains = stats.get("total_chains").unwrap_or(&0.0);
        let total_groups = stats.get("total_groups").unwrap_or(&0.0);
        let total_regions = stats.get("total_regions").unwrap_or(&0.0);
        let total_time = stats.get("total_time_ms").unwrap_or(&0.0);
        let speedup = stats.get("speedup_factor").unwrap_or(&1.0);

        info!("🎯 Hierarchical Proof Performance Report:");
        info!(
            "   📊 Scale: {} chains → {} groups → {} regions",
            total_chains, total_groups, total_regions
        );

        if let (Some(group_time), Some(region_time), Some(root_time)) = (
            stats.get("group_compute_time_ms"),
            stats.get("region_compute_time_ms"),
            stats.get("root_compute_time_ms"),
        ) {
            info!("   ⏱️  Timing breakdown:");
            info!("      Level 1 (Groups): {:.1}ms", group_time);
            info!("      Level 2 (Regions): {:.1}ms", region_time);
            info!("      Level 3 (Root): {:.1}ms", root_time);
        }

        info!(
            "   🚀 Total: {:.1}ms (speedup: {:.1}x)",
            total_time, speedup
        );

        if *total_time > BLOCK_PROCESSING_TARGET_MS as f64 {
            info!("   ⚠️  Performance target missed!");
        } else {
            info!("   ✅ Performance target achieved!");
        }
    }
}

impl Default for HierarchicalGlobalProof {
    fn default() -> Self {
        Self::new(CHAINS_PER_GROUP, GROUPS_PER_REGION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_proof_creation() {
        let proof_engine = HierarchicalGlobalProof::new(1000, 10);
        assert_eq!(proof_engine.max_chains_per_group, 1000);
        assert_eq!(proof_engine.max_groups_per_region, 10);
    }

    #[test]
    fn test_organize_into_groups() {
        let proof_engine = HierarchicalGlobalProof::new(2, 10);
        let mut chain_commitments = HashMap::new();

        // Add 5 chains
        for i in 0..5 {
            let chain_id = vec![i as u8; 32];
            let commitment = Buffer::from(vec![i as u8; 32]);
            chain_commitments.insert(chain_id, commitment);
        }

        let groups = proof_engine
            .organize_into_groups(&chain_commitments)
            .unwrap();

        // Should create 3 groups (2+2+1)
        assert_eq!(groups.len(), 3);

        // First two groups should have 2 chains each
        assert_eq!(groups.get("group_000000").unwrap().len(), 2);
        assert_eq!(groups.get("group_000001").unwrap().len(), 2);
        assert_eq!(groups.get("group_000002").unwrap().len(), 1);
    }

    #[test]
    fn test_compute_group_proof() {
        let proof_engine = HierarchicalGlobalProof::new(1000, 10);
        let block_hash = Buffer::from([1u8; 32].to_vec());

        // Create test group
        let mut group_chains = Vec::new();
        for i in 0..3 {
            let chain_id = vec![i as u8; 32];
            let commitment = Buffer::from(vec![i as u8; 32]);
            group_chains.push((chain_id, commitment));
        }

        let proof = proof_engine
            .compute_group_proof(&"test_group".to_string(), &group_chains, &block_hash)
            .unwrap();

        assert_eq!(proof.len(), 32);
    }
}
