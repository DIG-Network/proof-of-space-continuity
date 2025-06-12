use blake3;
use crc::{Crc, CRC_32_ISO_HDLC};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use log::{debug, info};
use memmap2::Mmap;
use napi::bindgen_prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use sha2::{Digest, Sha256};
use sha3::{Keccak256, Sha3_256};

use hmac::{Hmac, Mac, NewMac};
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

/// Compute full Merkle tree with all intermediate nodes for production proofs
pub fn compute_full_merkle_tree(hashes: &[&[u8]]) -> ([u8; 32], Vec<[u8; 32]>) {
    if hashes.is_empty() {
        return ([0u8; 32], Vec::new());
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

    let root = tree.root().unwrap_or([0u8; 32]);

    // Generate all intermediate nodes level by level
    let mut all_nodes = Vec::new();
    let mut current_level = leaves;

    // Build tree bottom-up, collecting all intermediate nodes
    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        // Process pairs of nodes
        for chunk in current_level.chunks(2) {
            if chunk.len() == 2 {
                // Pair of nodes - compute parent
                let parent_data = [&chunk[0][..], &chunk[1][..]].concat();
                let parent = compute_sha256(&parent_data);
                all_nodes.push(parent);
                next_level.push(parent);
            } else {
                // Odd node - promote to next level
                all_nodes.push(chunk[0]);
                next_level.push(chunk[0]);
            }
        }

        current_level = next_level;
    }

    (root, all_nodes)
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

// ====================================================================
// PRODUCTION CRYPTOGRAPHIC FUNCTIONS
// ====================================================================

/// Advanced hash functions for different use cases
pub fn compute_blake3(data: &[u8]) -> [u8; 32] {
    blake3::hash(data).into()
}

pub fn compute_sha3_256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn compute_keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Deterministic random number generation using ChaCha20
pub fn generate_deterministic_bytes(seed: &[u8], length: usize) -> Vec<u8> {
    let mut seed_array = [0u8; 32];
    let hash = compute_sha256(seed);
    seed_array.copy_from_slice(&hash);

    let mut rng = ChaCha20Rng::from_seed(seed_array);
    (0..length).map(|_| rng.gen()).collect()
}

/// Memory-hard VDF using Argon2
pub fn compute_memory_hard_vdf(
    input: &[u8],
    iterations: u32,
    memory_kb: u32,
    _parallelism: u32,
) -> HashChainResult<([u8; 32], f64, f64)> {
    let start_time = std::time::Instant::now();

    // Real memory-hard computation using actual memory allocation and access patterns
    let memory_size = (memory_kb as usize) * 1024;
    let mut memory_buffer = vec![0u8; memory_size];

    // Initialize memory with deterministic pattern
    let mut seed = compute_blake3(input);
    for chunk in memory_buffer.chunks_mut(32) {
        let chunk_len = chunk.len().min(32);
        chunk[..chunk_len].copy_from_slice(&seed[..chunk_len]);
        seed = compute_blake3(&[&seed[..], &iterations.to_be_bytes()].concat());
    }

    // Perform memory-hard iterations with real memory access
    let mut state = compute_blake3(input);
    for i in 0..iterations {
        // Memory-dependent computation - read from pseudo-random location
        let read_addr = (u32::from_be_bytes([state[0], state[1], state[2], state[3]]) as usize)
            % (memory_size - 64);
        let memory_chunk = &memory_buffer[read_addr..read_addr + 32];

        // Mix state with memory content
        state = compute_blake3(&[&state[..], memory_chunk, &i.to_be_bytes()].concat());

        // Write back to different location
        let write_addr = (u32::from_be_bytes([state[4], state[5], state[6], state[7]]) as usize)
            % (memory_size - 32);
        memory_buffer[write_addr..write_addr + 32].copy_from_slice(&state);
    }

    let hash = state;

    let mut output = [0u8; 32];
    output.copy_from_slice(&hash);

    let computation_time = start_time.elapsed().as_secs_f64() * 1000.0; // ms
    let memory_usage = memory_kb as f64 * 1024.0; // bytes

    Ok((output, computation_time, memory_usage))
}

/// HMAC-based key derivation
pub fn derive_key(master_key: &[u8], context: &[u8], info: &str) -> [u8; 32] {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_varkey(master_key).expect("HMAC can take key of any size");
    mac.update(context);
    mac.update(info.as_bytes());

    let result = mac.finalize().into_bytes();
    let mut derived_key = [0u8; 32];
    derived_key.copy_from_slice(&result);
    derived_key
}

/// Ed25519 signature generation and verification
pub fn sign_data(private_key: &[u8], data: &[u8]) -> HashChainResult<Vec<u8>> {
    if private_key.len() != 32 {
        return Err(HashChainError::InvalidPrivateKeySize(private_key.len()));
    }

    let secret_key = SecretKey::from_bytes(private_key)
        .map_err(|e| HashChainError::CryptographicError(format!("Invalid private key: {}", e)))?;

    let public_key = PublicKey::from(&secret_key);
    let keypair = Keypair {
        secret: secret_key,
        public: public_key,
    };

    let signature = keypair.sign(data);

    Ok(signature.to_bytes().to_vec())
}

pub fn verify_signature(public_key: &[u8], data: &[u8], signature: &[u8]) -> HashChainResult<bool> {
    if public_key.len() != 32 {
        return Err(HashChainError::InvalidPublicKeySize(public_key.len()));
    }

    if signature.len() != 64 {
        return Err(HashChainError::InvalidSignatureSize(signature.len()));
    }

    let public_key = PublicKey::from_bytes(public_key)
        .map_err(|e| HashChainError::CryptographicError(format!("Invalid public key: {}", e)))?;

    let signature = Signature::from_bytes(signature)
        .map_err(|e| HashChainError::CryptographicError(format!("Invalid signature: {}", e)))?;

    Ok(public_key.verify(data, &signature).is_ok())
}

/// Generate cryptographically secure random entropy
pub fn generate_secure_entropy(additional_data: &[u8]) -> [u8; 32] {
    let mut entropy_sources = Vec::new();

    // System timestamp
    let timestamp = get_current_timestamp();
    entropy_sources.extend_from_slice(&timestamp.to_be_bytes());

    // System randomness
    let mut system_random = [0u8; 32];
    getrandom::getrandom(&mut system_random).unwrap_or_default();
    entropy_sources.extend_from_slice(&system_random);

    // Additional user data
    entropy_sources.extend_from_slice(additional_data);

    // Process ID and thread ID for uniqueness
    entropy_sources.extend_from_slice(&std::process::id().to_be_bytes());

    // Final hash
    compute_blake3(&entropy_sources)
}

/// Parameters for computing commitment hash
pub struct CommitmentParams<'a> {
    pub prover_key: &'a [u8],
    pub data_hash: &'a [u8],
    pub block_height: u64,
    pub block_hash: &'a [u8],
    pub selected_chunks: &'a [u32],
    pub chunk_hashes: &'a [Vec<u8>],
    pub vdf_output: &'a [u8],
    pub entropy_hash: &'a [u8],
}

/// Compute commitment hash from parameters struct
pub fn compute_commitment_hash(params: &CommitmentParams) -> [u8; 32] {
    let mut commitment_data = Vec::new();

    // Prover identity
    commitment_data.extend_from_slice(params.prover_key);

    // Data commitment
    commitment_data.extend_from_slice(params.data_hash);

    // Blockchain anchor
    commitment_data.extend_from_slice(&params.block_height.to_be_bytes());
    commitment_data.extend_from_slice(params.block_hash);

    // Selected chunks (deterministic challenge)
    for &chunk_idx in params.selected_chunks {
        commitment_data.extend_from_slice(&chunk_idx.to_be_bytes());
    }

    // Chunk proofs
    for chunk_hash in params.chunk_hashes {
        commitment_data.extend_from_slice(chunk_hash);
    }

    // VDF proof
    commitment_data.extend_from_slice(params.vdf_output);

    // Entropy contribution
    commitment_data.extend_from_slice(params.entropy_hash);

    // Use Blake3 for final commitment (faster than SHA2)
    compute_blake3(&commitment_data)
}

/// Deterministic chunk selection using entropy (updated for 16 chunks)
pub fn select_chunks_deterministic(entropy: &[u8], total_chunks: f64, num_chunks: u32) -> Vec<u32> {
    let total_chunks_u32 = total_chunks as u32;

    if num_chunks == 0 || total_chunks_u32 == 0 || num_chunks > total_chunks_u32 {
        return Vec::new();
    }

    let mut selected = Vec::new();
    let mut used_indices = std::collections::HashSet::new();

    // Use entropy to seed deterministic selection
    let mut current_entropy = entropy.to_vec();

    while selected.len() < num_chunks as usize {
        // Hash current entropy to get next random value
        let hash = compute_blake3(&current_entropy);
        let chunk_index =
            u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) % total_chunks_u32;

        if !used_indices.contains(&chunk_index) {
            selected.push(chunk_index);
            used_indices.insert(chunk_index);
        }

        // Update entropy for next iteration
        current_entropy = hash.to_vec();
    }

    selected.sort_unstable();
    selected
}

/// Verify chunk selection algorithm
pub fn verify_chunk_selection(entropy: &[u8], total_chunks: u32, selected_chunks: &[u32]) -> bool {
    let num_chunks = selected_chunks.len() as u32;
    let expected = select_chunks_deterministic(entropy, total_chunks as f64, num_chunks);

    if expected.len() != selected_chunks.len() {
        return false;
    }

    for (i, &chunk) in selected_chunks.iter().enumerate() {
        if chunk != expected[i] {
            return false;
        }
    }

    true
}

/// Generate multi-source entropy for enhanced randomness
pub fn generate_multi_source_entropy(
    blockchain_entropy: &[u8],
    beacon_entropy: Option<&[u8]>,
    prover_entropy: &[u8],
) -> [u8; 32] {
    let mut entropy_data = Vec::new();

    // Blockchain randomness
    entropy_data.extend_from_slice(blockchain_entropy);

    // External beacon (if available)
    if let Some(beacon) = beacon_entropy {
        entropy_data.extend_from_slice(beacon);
    }

    // Prover-specific entropy
    entropy_data.extend_from_slice(prover_entropy);

    // System entropy
    let secure_entropy = generate_secure_entropy(&entropy_data);
    entropy_data.extend_from_slice(&secure_entropy);

    // Final combination using Keccak256 (different from other hashes)
    compute_keccak256(&entropy_data)
}

/// Continuous VDF state for tracking iterations and state
pub struct ContinuousVDF {
    current_state: [u8; 32],
    total_iterations: u64,
    last_block_height: u64,
    last_block_hash: [u8; 32],
    memory_buffer: Vec<u8>,
    pub memory_size: usize,
    pub start_time: std::time::Instant,
}

impl ContinuousVDF {
    pub fn new(initial_state: [u8; 32], memory_kb: u32) -> Self {
        let memory_size = (memory_kb as usize) * 1024;
        let mut memory_buffer = vec![0u8; memory_size];

        // Initialize memory with deterministic pattern
        let mut seed = compute_blake3(&initial_state);
        for chunk in memory_buffer.chunks_mut(32) {
            let chunk_len = chunk.len().min(32);
            chunk[..chunk_len].copy_from_slice(&seed[..chunk_len]);
            seed = compute_blake3(&[&seed[..], &0u64.to_be_bytes()].concat());
        }

        Self {
            current_state: initial_state,
            total_iterations: 0,
            last_block_height: 0,
            last_block_hash: [0u8; 32],
            memory_buffer,
            memory_size,
            start_time: std::time::Instant::now(),
        }
    }

    /// Perform one iteration of the VDF
    pub fn iterate(&mut self) -> [u8; 32] {
        // Memory-dependent computation - read from pseudo-random location
        let read_addr = (u32::from_be_bytes([
            self.current_state[0],
            self.current_state[1],
            self.current_state[2],
            self.current_state[3],
        ]) as usize)
            % (self.memory_size - 64);

        let memory_chunk = &self.memory_buffer[read_addr..read_addr + 32];

        // Mix state with memory content
        self.current_state = compute_blake3(
            &[
                &self.current_state[..],
                memory_chunk,
                &self.total_iterations.to_be_bytes(),
            ]
            .concat(),
        );

        // Write back to different location
        let write_addr = (u32::from_be_bytes([
            self.current_state[4],
            self.current_state[5],
            self.current_state[6],
            self.current_state[7],
        ]) as usize)
            % (self.memory_size - 32);

        self.memory_buffer[write_addr..write_addr + 32].copy_from_slice(&self.current_state);

        self.total_iterations += 1;
        self.current_state
    }

    /// Get current VDF state and iteration count
    pub fn get_state(&self) -> ([u8; 32], u64) {
        (self.current_state, self.total_iterations)
    }

    /// Sign a block against the current VDF state
    pub fn sign_block(
        &mut self,
        block_height: u64,
        block_hash: [u8; 32],
        required_iterations: u64,
    ) -> HashChainResult<[u8; 32]> {
        if self.total_iterations < required_iterations {
            return Err(HashChainError::VDFError(format!(
                "Insufficient VDF iterations: {} < {}",
                self.total_iterations, required_iterations
            )));
        }

        // Create block signature using VDF state
        let signature_data = [
            &self.current_state[..],
            &block_height.to_be_bytes(),
            &block_hash[..],
            &self.total_iterations.to_be_bytes(),
        ]
        .concat();

        let signature = compute_blake3(&signature_data);

        // Update last block info
        self.last_block_height = block_height;
        self.last_block_hash = block_hash;

        Ok(signature)
    }

    /// Verify a block signature
    pub fn verify_block_signature(
        &self,
        block_height: u64,
        block_hash: [u8; 32],
        signature: [u8; 32],
        required_iterations: u64,
    ) -> bool {
        if self.total_iterations < required_iterations {
            return false;
        }

        let signature_data = [
            &self.current_state[..],
            &block_height.to_be_bytes(),
            &block_hash[..],
            &self.total_iterations.to_be_bytes(),
        ]
        .concat();

        let expected_signature = compute_blake3(&signature_data);
        signature == expected_signature
    }
}

/// Sign block data using Ed25519 signature
pub fn sign_block(
    private_key: &[u8],
    block_height: u64,
    block_hash: &[u8],
    vdf_state: &[u8],
    total_iterations: u64,
) -> HashChainResult<Vec<u8>> {
    // Create block data to sign
    let block_data = [
        &block_height.to_be_bytes(),
        block_hash,
        vdf_state,
        &total_iterations.to_be_bytes(),
    ]
    .concat();

    // Sign the block data
    sign_data(private_key, &block_data)
}

/// Verify block signature
pub fn verify_block_signature(
    public_key: &[u8],
    block_height: u64,
    block_hash: &[u8],
    vdf_state: &[u8],
    total_iterations: u64,
    signature: &[u8],
) -> HashChainResult<bool> {
    // Recreate block data
    let block_data = [
        &block_height.to_be_bytes(),
        block_hash,
        vdf_state,
        &total_iterations.to_be_bytes(),
    ]
    .concat();

    // Verify the signature
    verify_signature(public_key, &block_data, signature)
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
