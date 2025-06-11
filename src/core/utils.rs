use crc::{Crc, CRC_32_ISO_HDLC};
use log::{debug, info};
use memmap2::Mmap;
use napi::bindgen_prelude::*;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::errors::{HashChainError, HashChainResult};
use crate::core::types::*;

/// Compute SHA256 hash of data
pub fn compute_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Alias for compute_sha256 for new interface compatibility
pub fn sha256(data: &[u8]) -> [u8; 32] {
    compute_sha256(data)
}

/// Compute SHA256 from two byte slices (for Merkle tree operations)
pub fn compute_sha256_from_slices(left: &[u8], right: &[u8]) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(left);
    data.extend_from_slice(right);
    compute_sha256(&data)
}

/// Fast CRC32 checksum for file integrity (much faster than SHA256)
pub fn compute_crc32(data: &[u8]) -> u32 {
    const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
    CRC.checksum(data)
}

/// Memory-mapped file checksum (solves Windows file access issues)
pub fn compute_file_checksum_mmap(file_path: &str) -> HashChainResult<u32> {
    let file = File::open(file_path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    Ok(compute_crc32(&mmap))
}

/// Get current timestamp in seconds since Unix epoch
pub fn get_current_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Generate a random nonce for proofs
pub fn generate_proof_nonce(additional_entropy: &[u8]) -> [u8; 12] {
    let timestamp = get_current_timestamp();
    let mut data = Vec::new();
    data.extend_from_slice(&timestamp.to_be_bytes());
    data.extend_from_slice(additional_entropy);

    let hash = compute_sha256(&data);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&hash[..12]);
    nonce
}

/// Validate basic input parameters
pub fn validate_public_key(public_key: &Buffer) -> HashChainResult<()> {
    if public_key.len() != 32 {
        return Err(HashChainError::InvalidPublicKeySize(public_key.len()));
    }
    Ok(())
}

pub fn validate_block_hash(block_hash: &Buffer) -> HashChainResult<()> {
    if block_hash.len() != 32 {
        return Err(HashChainError::InvalidBlockHashSize(block_hash.len()));
    }
    Ok(())
}

pub fn validate_block_height(block_height: f64) -> HashChainResult<u64> {
    if block_height < 0.0 {
        return Err(HashChainError::InvalidBlockHeight(block_height));
    }
    if block_height > (u64::MAX as f64) {
        return Err(HashChainError::InvalidBlockHeight(block_height));
    }
    Ok(block_height as u64)
}

/// Performance timing utilities
pub struct PerformanceTimer {
    start_time: std::time::Instant,
    operation_name: String,
}

impl PerformanceTimer {
    pub fn new(operation_name: &str) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            operation_name: operation_name.to_string(),
        }
    }

    pub fn elapsed_ms(&self) -> u32 {
        self.start_time.elapsed().as_millis() as u32
    }

    pub fn check_target(self, target_ms: u32) -> HashChainResult<u32> {
        let elapsed = self.elapsed_ms();
        if elapsed > target_ms {
            debug!(
                "Performance target missed: {} took {}ms (target: {}ms)",
                self.operation_name, elapsed, target_ms
            );
        } else {
            debug!(
                "Performance target met: {} took {}ms (target: {}ms)",
                self.operation_name, elapsed, target_ms
            );
        }
        Ok(elapsed)
    }
}

/// Hierarchical system utilities
pub fn generate_chain_id(public_key: &Buffer, data_file_hash: &[u8]) -> ChainId {
    let mut data = Vec::new();
    data.extend_from_slice(public_key);
    data.extend_from_slice(data_file_hash);
    compute_sha256(&data).to_vec()
}

pub fn generate_group_id(chain_count: u32) -> GroupId {
    format!("group_{:06}", chain_count / CHAINS_PER_GROUP)
}

pub fn generate_region_id(group_count: u32) -> RegionId {
    format!("region_{:03}", group_count / GROUPS_PER_REGION)
}

/// Compute hierarchical position hash for compact proofs
pub fn compute_hierarchical_position(
    chain_id: &ChainId,
    group_id: &GroupId,
    region_id: &RegionId,
) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(chain_id);
    data.extend_from_slice(group_id.as_bytes());
    data.extend_from_slice(region_id.as_bytes());
    compute_sha256(&data)
}

/// Chunk validation utilities
pub fn validate_chunk_index(chunk_idx: u32, total_chunks: u64) -> HashChainResult<()> {
    if chunk_idx >= total_chunks as u32 {
        return Err(HashChainError::ChunkIndexOutOfRange {
            index: chunk_idx,
            max: total_chunks,
        });
    }
    Ok(())
}

pub fn validate_chunk_count(count: u64) -> HashChainResult<()> {
    if count > HASHCHAIN_MAX_CHUNKS {
        return Err(HashChainError::TooManyChunks {
            count,
            max: HASHCHAIN_MAX_CHUNKS,
        });
    }
    if count < HASHCHAIN_MIN_CHUNKS {
        return Err(HashChainError::TooFewChunks {
            count,
            min: HASHCHAIN_MIN_CHUNKS,
        });
    }
    Ok(())
}

/// File path utilities
pub fn derive_data_file_path(hashchain_file_path: &str) -> String {
    hashchain_file_path.replace(".hashchain", ".data")
}

pub fn create_output_file_paths(output_dir: &str, data_hash_hex: &str) -> (String, String) {
    let data_file_path = format!("{}/{}.data", output_dir, data_hash_hex);
    let hashchain_file_path = format!("{}/{}.hashchain", output_dir, data_hash_hex);
    (data_file_path, hashchain_file_path)
}

/// Merkle tree utilities for hierarchical system
pub fn compute_merkle_root(hashes: &[&[u8]]) -> [u8; 32] {
    if hashes.is_empty() {
        return [0u8; 32];
    }

    // Convert to the format expected by rs_merkle
    let leaves: Vec<[u8; 32]> = hashes
        .iter()
        .map(|&hash| {
            let mut leaf = [0u8; 32];
            leaf.copy_from_slice(&hash[..32]);
            leaf
        })
        .collect();

    use rs_merkle::{algorithms::Sha256 as MerkleSha256, MerkleTree as RsMerkleTree};
    let tree = RsMerkleTree::<MerkleSha256>::from_leaves(&leaves);
    tree.root().unwrap_or([0u8; 32])
}

/// Scale monitoring utilities
pub fn check_hierarchical_limits(
    chain_count: u32,
    group_count: u32,
    region_count: u32,
) -> HashChainResult<()> {
    // Check maximum chains
    if chain_count > MAX_CHAINS_PER_INSTANCE {
        return Err(HashChainError::ScaleLimit {
            count: chain_count,
            limit: MAX_CHAINS_PER_INSTANCE,
        });
    }

    // Check group distribution
    let expected_groups = (chain_count + CHAINS_PER_GROUP - 1) / CHAINS_PER_GROUP;
    if group_count > expected_groups + 10 {
        // Allow some overhead
        return Err(HashChainError::GroupAssignment {
            reason: format!(
                "Too many groups: {} (expected ~{})",
                group_count, expected_groups
            ),
        });
    }

    // Check region distribution
    let expected_regions = (group_count + GROUPS_PER_REGION - 1) / GROUPS_PER_REGION;
    if region_count > expected_regions + 5 {
        // Allow some overhead
        return Err(HashChainError::GroupAssignment {
            reason: format!(
                "Too many regions: {} (expected ~{})",
                region_count, expected_regions
            ),
        });
    }

    Ok(())
}

/// Logging and monitoring utilities
pub fn log_performance_metrics(operation: &str, chain_count: u32, elapsed_ms: u32, target_ms: u32) {
    let chains_per_sec = if elapsed_ms > 0 {
        (chain_count * 1000) / elapsed_ms
    } else {
        0
    };

    if elapsed_ms <= target_ms {
        info!(
            "✅ {}: {} chains in {}ms (target: {}ms) - {} chains/sec",
            operation, chain_count, elapsed_ms, target_ms, chains_per_sec
        );
    } else {
        info!(
            "⚠️  {}: {} chains in {}ms (target: {}ms) - {} chains/sec - SLOW",
            operation, chain_count, elapsed_ms, target_ms, chains_per_sec
        );
    }
}

/// Convert hex strings to bytes and vice versa
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    // Error and Status are already imported from napi::bindgen_prelude::*
    hex::decode(hex).map_err(|e| Error::new(Status::InvalidArg, format!("Invalid hex: {}", e)))
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Safe division for performance calculations
pub fn safe_division(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

/// Memory usage estimation
pub fn estimate_memory_usage(chain_count: u32) -> f64 {
    // Rough estimate: 1KB per chain
    (chain_count as f64 * 1024.0) / (1024.0 * 1024.0) // MB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_computation() {
        let data = b"test data";
        let hash = compute_sha256(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_chain_id_generation() {
        let public_key = Buffer::from([1u8; 32].to_vec());
        let data_hash = [2u8; 32];
        let chain_id = generate_chain_id(&public_key, &data_hash);
        assert_eq!(chain_id.len(), 32);
    }

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::new("test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }
}
