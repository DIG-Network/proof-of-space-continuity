use napi::bindgen_prelude::*;
use std::collections::HashSet;
use rayon::prelude::*;

use crate::core::{
    types::*,
    utils::{compute_sha256, validate_chunk_count},
};

/// CONSENSUS CRITICAL: Enhanced chunk selection algorithm V2
/// Implements multi-source entropy, 16 chunks per block, and enhanced security
pub fn select_chunks_deterministic_v2(
    entropy: MultiSourceEntropy,
    total_chunks: f64,
) -> Result<EnhancedChunkSelectionResult> {
    let total_chunks_u64 = total_chunks as u64;

    validate_chunk_count(total_chunks_u64)?;

    // Enhanced minimum chunk requirement for 100MB files
    if total_chunks_u64 < HASHCHAIN_MIN_CHUNKS {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "Total chunks ({}) must be >= {} for enhanced security",
                total_chunks_u64, HASHCHAIN_MIN_CHUNKS
            ),
        ));
    }

    // Select up to 16 chunks or all available chunks if fewer
    let chunks_to_select = std::cmp::min(CHUNKS_PER_BLOCK as u64, total_chunks_u64) as u32;

    let mut selected_indices = Vec::new();
    let mut used_indices = HashSet::new();

    // Use combined entropy for enhanced unpredictability
    let combined_entropy = create_combined_entropy(&entropy)?;

    // Enhanced chunk selection with increased attempts for uniqueness
    for chunk_slot in 0..chunks_to_select {
        let mut attempts = 0;

        while attempts < CHUNK_SELECTION_MAX_ATTEMPTS {
            // Create deterministic but unpredictable seed from combined entropy
            let mut seed_input = Vec::new();
            seed_input.extend_from_slice(&combined_entropy);
            seed_input.extend_from_slice(&chunk_slot.to_be_bytes());
            seed_input.extend_from_slice(&attempts.to_be_bytes());
            seed_input.extend_from_slice(&entropy.timestamp.to_be_bytes());

            let seed_hash = compute_sha256(&seed_input);

            // Extract 16-byte seed for enhanced randomness
            let seed_bytes = &seed_hash[..CHUNK_SELECTION_SEED_SIZE];
            let seed = u128::from_be_bytes([
                seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3],
                seed_bytes[4], seed_bytes[5], seed_bytes[6], seed_bytes[7],
                seed_bytes[8], seed_bytes[9], seed_bytes[10], seed_bytes[11],
                seed_bytes[12], seed_bytes[13], seed_bytes[14], seed_bytes[15],
            ]);

            // Calculate chunk index using secure modulo operation
            let chunk_index = (seed % total_chunks_u64 as u128) as u32;

            // Accept if unique, otherwise retry
            if !used_indices.contains(&chunk_index) {
                selected_indices.push(chunk_index);
                used_indices.insert(chunk_index);
                break;
            }

            attempts += 1;
        }

        // Enhanced fallback for unique chunk selection
        if attempts >= CHUNK_SELECTION_MAX_ATTEMPTS {
            // Use deterministic but different approach for fallback
            let fallback_hash = compute_sha256(&[
                &combined_entropy[..],
                &chunk_slot.to_be_bytes(),
                b"fallback"
            ].concat());

            for offset in 0..total_chunks_u64 as u32 {
                let fallback_seed = u32::from_be_bytes([
                    fallback_hash[offset as usize % 28],
                    fallback_hash[(offset as usize + 1) % 28],
                    fallback_hash[(offset as usize + 2) % 28],
                    fallback_hash[(offset as usize + 3) % 28],
                ]);
                let candidate_idx = fallback_seed % total_chunks_u64 as u32;

                if !used_indices.contains(&candidate_idx) {
                    selected_indices.push(candidate_idx);
                    used_indices.insert(candidate_idx);
                    break;
                }
            }

            // Final safety check
            if selected_indices.len() != (chunk_slot + 1) as usize {
                return Err(Error::new(
                    Status::GenericFailure,
                    format!("Failed to find unique chunk for slot {}", chunk_slot),
                ));
            }
        }
    }

    // Create enhanced verification hash
    let verification_hash = create_verification_hash_v2(&entropy, &selected_indices)?;

    // Create unpredictability proof
    let unpredictability_proof = create_unpredictability_proof(&entropy, &selected_indices)?;

    Ok(EnhancedChunkSelectionResult {
        selected_indices,
        algorithm_version: CHUNK_SELECTION_VERSION,
        total_chunks,
        entropy,
        verification_hash: Buffer::from(verification_hash.to_vec()),
        unpredictability_proof: Buffer::from(unpredictability_proof.to_vec()),
    })
}

/// Create combined entropy from multiple sources for enhanced security
fn create_combined_entropy(entropy: &MultiSourceEntropy) -> Result<Vec<u8>> {
    let mut combined = Vec::new();

    // Add blockchain entropy
    combined.extend_from_slice(&entropy.blockchain_entropy);

    // Add beacon entropy if available
    if let Some(ref beacon_entropy) = entropy.beacon_entropy {
        combined.extend_from_slice(beacon_entropy);
    } else {
        // If no beacon entropy, add deterministic fallback
        combined.extend_from_slice(&[0u8; 32]);
    }

    // Add local entropy
    combined.extend_from_slice(&entropy.local_entropy);

    // Add timestamp for temporal uniqueness
    combined.extend_from_slice(&entropy.timestamp.to_be_bytes());

    // Hash everything together
    let final_hash = compute_sha256(&combined);
    Ok(final_hash.to_vec())
}

/// Create enhanced verification hash for consensus validation
fn create_verification_hash_v2(
    entropy: &MultiSourceEntropy,
    selected_indices: &[u32]
) -> Result<[u8; 32]> {
    let mut verification_input = Vec::new();

    // Add algorithm version
    verification_input.extend_from_slice(&CHUNK_SELECTION_VERSION.to_be_bytes());

    // Add combined entropy hash
    verification_input.extend_from_slice(&entropy.combined_hash);

    // Add sorted indices for order-independence
    let mut sorted_indices = selected_indices.to_vec();
    sorted_indices.sort();
    for idx in sorted_indices {
        verification_input.extend_from_slice(&idx.to_be_bytes());
    }

    // Add timestamp for uniqueness
    verification_input.extend_from_slice(&entropy.timestamp.to_be_bytes());

    Ok(compute_sha256(&verification_input))
}

/// Create unpredictability proof to resist pre-computation attacks
fn create_unpredictability_proof(
    entropy: &MultiSourceEntropy,
    selected_indices: &[u32]
) -> Result<[u8; 32]> {
    let mut proof_input = Vec::new();

    // Use entropy sources in specific order
    proof_input.extend_from_slice(&entropy.blockchain_entropy);
    
    if let Some(ref beacon_entropy) = entropy.beacon_entropy {
        proof_input.extend_from_slice(beacon_entropy);
    }
    
    proof_input.extend_from_slice(&entropy.local_entropy);

    // Add indices in original selection order (preserves timing info)
    for idx in selected_indices {
        proof_input.extend_from_slice(&idx.to_be_bytes());
    }

    // Add multiple entropy source indicator
    proof_input.push(if entropy.beacon_entropy.is_some() { 0xFF } else { 0x00 });

    Ok(compute_sha256(&proof_input))
}

/// Enhanced verification function for multi-source entropy
pub fn verify_enhanced_chunk_selection(
    entropy: MultiSourceEntropy,
    total_chunks: f64,
    claimed_indices: Vec<u32>,
    expected_algorithm_version: Option<u32>,
) -> Result<bool> {
    let expected_version = expected_algorithm_version.unwrap_or(CHUNK_SELECTION_VERSION);

    if expected_version != CHUNK_SELECTION_VERSION {
        return Ok(false);
    }

    // Re-run the enhanced algorithm
    let result = select_chunks_deterministic_v2(entropy, total_chunks)?;

    // Verify indices match exactly (order matters for consensus)
    if claimed_indices.len() != result.selected_indices.len() {
        return Ok(false);
    }

    Ok(claimed_indices == result.selected_indices)
}

/// Legacy compatibility function - wraps v2 with single entropy source
pub fn select_chunks_deterministic(
    block_hash: Buffer,
    total_chunks: f64,
) -> Result<ChunkSelectionResult> {
    // Convert to enhanced entropy format for backwards compatibility
    let entropy = MultiSourceEntropy {
        blockchain_entropy: block_hash.clone(),
        beacon_entropy: None,
        local_entropy: Buffer::from([0u8; 32].to_vec()), // Deterministic for compatibility
        timestamp: 0.0, // Deterministic for compatibility
        combined_hash: compute_sha256(&block_hash).to_vec().into(),
    };

    // Run enhanced algorithm
    let enhanced_result = select_chunks_deterministic_v2(entropy, total_chunks)?;

    // Convert back to legacy format
    Ok(ChunkSelectionResult {
        selected_indices: enhanced_result.selected_indices,
        algorithm_version: 1, // Report as v1 for compatibility
        total_chunks,
        block_hash,
        verification_hash: enhanced_result.verification_hash,
    })
}

/// Verify chunk selection matches network consensus algorithm (legacy)
pub fn verify_chunk_selection_internal(
    block_hash: Buffer,
    total_chunks: f64,
    claimed_indices: Vec<u32>,
    expected_algorithm_version: Option<u32>,
) -> Result<bool> {
    let expected_version = expected_algorithm_version.unwrap_or(1); // Default to v1 for compatibility

    if expected_version == 1 {
        // Use legacy verification
        let result = select_chunks_deterministic(block_hash, total_chunks)?;
        return Ok(claimed_indices == result.selected_indices);
    } else if expected_version == 2 {
        // Would need entropy for v2 verification
        return Err(Error::new(
            Status::InvalidArg,
            "V2 verification requires MultiSourceEntropy".to_string(),
        ));
    }

    Ok(false)
}

/// Parallel chunk selection for high performance with many chains
pub fn select_chunks_parallel(
    entropy_list: Vec<(MultiSourceEntropy, f64)>, // (entropy, total_chunks) pairs
) -> Result<Vec<EnhancedChunkSelectionResult>> {
    // Use rayon for parallel processing
    let results: Vec<Result<EnhancedChunkSelectionResult>> = entropy_list
        .into_par_iter()
        .map(|(entropy, total_chunks)| {
            select_chunks_deterministic_v2(entropy, total_chunks)
        })
        .collect();

    // Collect results and propagate any errors
    let mut successful_results = Vec::new();
    for result in results {
        match result {
            Ok(chunk_result) => successful_results.push(chunk_result),
            Err(e) => return Err(e),
        }
    }

    Ok(successful_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_chunk_selection_deterministic() {
        let entropy = MultiSourceEntropy {
            blockchain_entropy: Buffer::from([1u8; 32].to_vec()),
            beacon_entropy: Some(Buffer::from([2u8; 32].to_vec())),
            local_entropy: Buffer::from([3u8; 32].to_vec()),
            timestamp: 1234567890.0,
            combined_hash: Buffer::from([4u8; 32].to_vec()),
        };
        let total_chunks = 100000.0;

        let result1 = select_chunks_deterministic_v2(entropy.clone(), total_chunks).unwrap();
        let result2 = select_chunks_deterministic_v2(entropy, total_chunks).unwrap();

        // Should be deterministic
        assert_eq!(result1.selected_indices, result2.selected_indices);
        assert_eq!(
            result1.verification_hash.as_ref(),
            result2.verification_hash.as_ref()
        );
    }

    #[test]
    fn test_enhanced_chunk_selection_validation() {
        let entropy = MultiSourceEntropy {
            blockchain_entropy: Buffer::from([1u8; 32].to_vec()),
            beacon_entropy: None,
            local_entropy: Buffer::from([3u8; 32].to_vec()),
            timestamp: 1234567890.0,
            combined_hash: Buffer::from([4u8; 32].to_vec()),
        };

        // Should work with large files (100MB+)
        let result = select_chunks_deterministic_v2(entropy.clone(), 100000.0);
        assert!(result.is_ok());

        // Should reject files too small for enhanced security
        let result = select_chunks_deterministic_v2(entropy, 1000.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_enhanced_chunk_selection_verification() {
        let entropy = MultiSourceEntropy {
            blockchain_entropy: Buffer::from([1u8; 32].to_vec()),
            beacon_entropy: Some(Buffer::from([2u8; 32].to_vec())),
            local_entropy: Buffer::from([3u8; 32].to_vec()),
            timestamp: 1234567890.0,
            combined_hash: Buffer::from([4u8; 32].to_vec()),
        };
        let total_chunks = 100000.0;

        let result = select_chunks_deterministic_v2(entropy.clone(), total_chunks).unwrap();

        // Should verify correctly
        let is_valid = verify_enhanced_chunk_selection(
            entropy,
            total_chunks,
            result.selected_indices,
            Some(CHUNK_SELECTION_VERSION),
        )
        .unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_enhanced_chunk_selection_unique() {
        let entropy = MultiSourceEntropy {
            blockchain_entropy: Buffer::from([1u8; 32].to_vec()),
            beacon_entropy: Some(Buffer::from([2u8; 32].to_vec())),
            local_entropy: Buffer::from([3u8; 32].to_vec()),
            timestamp: 1234567890.0,
            combined_hash: Buffer::from([4u8; 32].to_vec()),
        };
        let total_chunks = 100000.0;

        let result = select_chunks_deterministic_v2(entropy, total_chunks).unwrap();

        // All indices should be unique
        let mut unique_indices = HashSet::new();
        for idx in &result.selected_indices {
            assert!(unique_indices.insert(*idx));
        }

        // Should select exactly 16 chunks for enhanced security
        assert_eq!(unique_indices.len(), CHUNKS_PER_BLOCK as usize);
        assert_eq!(result.selected_indices.len(), CHUNKS_PER_BLOCK as usize);
    }

    #[test]
    fn test_parallel_chunk_selection() {
        let entropy_list: Vec<_> = (0..10)
            .map(|i| {
                let entropy = MultiSourceEntropy {
                    blockchain_entropy: Buffer::from([i as u8; 32].to_vec()),
                    beacon_entropy: Some(Buffer::from([(i + 1) as u8; 32].to_vec())),
                    local_entropy: Buffer::from([(i + 2) as u8; 32].to_vec()),
                    timestamp: (1234567890 + i) as f64,
                    combined_hash: Buffer::from([(i + 3) as u8; 32].to_vec()),
                };
                (entropy, 100000.0)
            })
            .collect();

        let results = select_chunks_parallel(entropy_list).unwrap();
        assert_eq!(results.len(), 10);

        // Each result should have 16 unique chunks
        for result in results {
            assert_eq!(result.selected_indices.len(), CHUNKS_PER_BLOCK as usize);
            let unique_count = result.selected_indices.iter().collect::<HashSet<_>>().len();
            assert_eq!(unique_count, CHUNKS_PER_BLOCK as usize);
        }
    }
}
