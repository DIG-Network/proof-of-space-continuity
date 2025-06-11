use napi::bindgen_prelude::*;
use std::collections::HashSet;

use crate::core::{
    errors::{HashChainError, HashChainResult},
    types::*,
    utils::{compute_sha256, validate_block_hash, validate_chunk_count},
};

/// CONSENSUS CRITICAL: Standardized chunk selection algorithm V1
pub fn select_chunks_deterministic(
    block_hash: Buffer,
    total_chunks: f64,
) -> Result<ChunkSelectionResult> {
    let total_chunks_u64 = total_chunks as u64;

    validate_block_hash(&block_hash)?;
    validate_chunk_count(total_chunks_u64)?;

    if total_chunks_u64 < CHUNKS_PER_BLOCK as u64 {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "Total chunks ({}) must be >= CHUNKS_PER_BLOCK ({})",
                total_chunks_u64, CHUNKS_PER_BLOCK
            ),
        ));
    }

    let mut selected_indices = Vec::new();
    let mut used_indices = HashSet::new();

    // Deterministic chunk selection using standardized algorithm
    for chunk_slot in 0..CHUNKS_PER_BLOCK {
        let mut attempts = 0;

        while attempts < CHUNK_SELECTION_MAX_ATTEMPTS {
            // Create deterministic seed from block hash and attempt number
            let mut seed_input = Vec::new();
            seed_input.extend_from_slice(&block_hash);
            seed_input.extend_from_slice(&chunk_slot.to_be_bytes());
            seed_input.extend_from_slice(&attempts.to_be_bytes());

            let seed_hash = compute_sha256(&seed_input);

            // Extract 8-byte seed from hash
            let seed_bytes = &seed_hash[..CHUNK_SELECTION_SEED_SIZE];
            let seed = u64::from_be_bytes([
                seed_bytes[0],
                seed_bytes[1],
                seed_bytes[2],
                seed_bytes[3],
                seed_bytes[4],
                seed_bytes[5],
                seed_bytes[6],
                seed_bytes[7],
            ]);

            // Calculate chunk index using modulo (consensus standard)
            let chunk_index = (seed % total_chunks_u64) as u32;

            // Accept if unique, otherwise retry with next attempt
            if !used_indices.contains(&chunk_index) {
                selected_indices.push(chunk_index);
                used_indices.insert(chunk_index);
                break;
            }

            attempts += 1;
        }

        // Consensus requirement: must find unique chunk within max attempts
        if attempts >= CHUNK_SELECTION_MAX_ATTEMPTS {
            // For edge case where we have exactly CHUNKS_PER_BLOCK chunks,
            // find the first unused chunk as a fallback
            for candidate_idx in 0..total_chunks_u64 as u32 {
                if !used_indices.contains(&candidate_idx) {
                    selected_indices.push(candidate_idx);
                    used_indices.insert(candidate_idx);
                    break;
                }
            }

            // If we still couldn't find a unique chunk, this is a critical error
            if selected_indices.len() != (chunk_slot + 1) as usize {
                return Err(Error::new(
                    Status::GenericFailure,
                    format!("Failed to find unique chunk for slot {}", chunk_slot),
                ));
            }
        }
    }

    // Create verification hash for consensus validation
    let mut verification_input = Vec::new();
    verification_input.extend_from_slice(&CHUNK_SELECTION_VERSION.to_be_bytes());
    verification_input.extend_from_slice(&block_hash);
    verification_input.extend_from_slice(&total_chunks_u64.to_be_bytes());

    let mut sorted_indices = selected_indices.clone();
    sorted_indices.sort();
    for idx in sorted_indices {
        verification_input.extend_from_slice(&idx.to_be_bytes());
    }

    let verification_hash = compute_sha256(&verification_input);

    Ok(ChunkSelectionResult {
        selected_indices,
        algorithm_version: CHUNK_SELECTION_VERSION,
        total_chunks,
        block_hash,
        verification_hash: Buffer::from(verification_hash.to_vec()),
    })
}

/// Verify chunk selection matches network consensus algorithm
pub fn verify_chunk_selection_internal(
    block_hash: Buffer,
    total_chunks: f64,
    claimed_indices: Vec<u32>,
    expected_algorithm_version: Option<u32>,
) -> Result<bool> {
    let expected_version = expected_algorithm_version.unwrap_or(CHUNK_SELECTION_VERSION);

    if expected_version != CHUNK_SELECTION_VERSION {
        return Ok(false);
    }

    // Re-run the standardized algorithm
    let result = select_chunks_deterministic(block_hash, total_chunks)?;

    // Verify indices match exactly
    if claimed_indices.len() != result.selected_indices.len() {
        return Ok(false);
    }

    // Verify order preservation (consensus requirement)
    Ok(claimed_indices == result.selected_indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_selection_deterministic() {
        let block_hash = Buffer::from([1u8; 32].to_vec());
        let total_chunks = 1000.0;

        let result1 = select_chunks_deterministic(block_hash.clone(), total_chunks).unwrap();
        let result2 = select_chunks_deterministic(block_hash, total_chunks).unwrap();

        // Should be deterministic
        assert_eq!(result1.selected_indices, result2.selected_indices);
        assert_eq!(
            result1.verification_hash.as_ref(),
            result2.verification_hash.as_ref()
        );
    }

    #[test]
    fn test_chunk_selection_validation() {
        let block_hash = Buffer::from([1u8; 32].to_vec());

        // Too few chunks
        let result = select_chunks_deterministic(block_hash.clone(), 2.0);
        assert!(result.is_err());

        // Invalid block hash
        let invalid_hash = Buffer::from([1u8; 31].to_vec());
        let result = select_chunks_deterministic(invalid_hash, 1000.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_selection_verification() {
        let block_hash = Buffer::from([1u8; 32].to_vec());
        let total_chunks = 1000.0;

        let result = select_chunks_deterministic(block_hash.clone(), total_chunks).unwrap();

        // Should verify correctly
        let is_valid = verify_chunk_selection_internal(
            block_hash,
            total_chunks,
            result.selected_indices,
            Some(CHUNK_SELECTION_VERSION),
        )
        .unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_chunk_selection_unique() {
        let block_hash = Buffer::from([1u8; 32].to_vec());
        let total_chunks = 1000.0;

        let result = select_chunks_deterministic(block_hash, total_chunks).unwrap();

        // All indices should be unique
        let mut unique_indices = HashSet::new();
        for idx in &result.selected_indices {
            assert!(unique_indices.insert(*idx));
        }

        assert_eq!(unique_indices.len(), CHUNKS_PER_BLOCK as usize);
    }
}
