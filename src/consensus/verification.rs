use napi::bindgen_prelude::*;

use crate::consensus::{
    chunk_selection::verify_chunk_selection_internal,
    commitments::calculate_commitment_hash_internal,
};
use crate::core::{types::*, utils::validate_chunk_index};

/// Verify proof window for storage continuity
pub fn verify_proof_of_storage_continuity_internal(
    proof_window: ProofWindow,
    anchored_commitment: Buffer,
    merkle_root: Buffer,
    total_chunks: f64,
) -> Result<bool> {
    // Validate input parameters first
    if anchored_commitment.len() != HASH_SIZE {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Anchored commitment must be {} bytes", HASH_SIZE),
        ));
    }

    if merkle_root.len() != HASH_SIZE {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Merkle root must be {} bytes", HASH_SIZE),
        ));
    }

    if total_chunks <= 0.0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Total chunks must be positive".to_string(),
        ));
    }

    // CONSENSUS CRITICAL: Verify proof window has exactly 8 commitments
    if proof_window.commitments.len() != PROOF_WINDOW_BLOCKS as usize {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "Proof window must have exactly {} commitments, got {}",
                PROOF_WINDOW_BLOCKS,
                proof_window.commitments.len()
            ),
        ));
    }

    // CONSENSUS CRITICAL: Verify start commitment connects to anchored commitment
    if proof_window.start_commitment.as_ref() != anchored_commitment.as_ref() {
        return Err(Error::new(
            Status::InvalidArg,
            "Start commitment does not match anchored commitment".to_string(),
        ));
    }

    // Verify commitment chain integrity
    let mut expected_previous = proof_window.start_commitment.clone();

    for commitment in &proof_window.commitments {
        if commitment.previous_commitment.as_ref() != expected_previous.as_ref() {
            return Ok(false);
        }

        // Verify commitment hash
        let expected_hash = calculate_commitment_hash_internal(commitment)?;
        if commitment.commitment_hash.as_ref() != expected_hash.as_ref() {
            return Ok(false);
        }

        // Verify chunk selection follows consensus
        if !verify_chunk_selection_internal(
            commitment.block_hash.clone(),
            total_chunks,
            commitment.selected_chunks.clone(),
            Some(CHUNK_SELECTION_VERSION),
        )? {
            return Ok(false);
        }

        // Verify all selected chunks are within valid range
        for &chunk_idx in &commitment.selected_chunks {
            if validate_chunk_index(chunk_idx, total_chunks as u64).is_err() {
                return Ok(false);
            }
        }

        // Verify chunk hashes are properly formatted
        for chunk_hash in &commitment.chunk_hashes {
            if chunk_hash.len() != HASH_SIZE {
                return Ok(false);
            }
        }

        expected_previous = commitment.commitment_hash.clone();
    }

    // Verify end commitment matches
    if expected_previous.as_ref() != proof_window.end_commitment.as_ref() {
        return Ok(false);
    }

    Ok(true)
}

/// Verify a single commitment's integrity
pub fn verify_commitment_integrity(
    commitment: &PhysicalAccessCommitment,
    total_chunks: f64,
) -> Result<bool> {
    // Verify commitment hash
    let calculated_hash = calculate_commitment_hash_internal(commitment)?;
    if commitment.commitment_hash.as_ref() != calculated_hash.as_ref() {
        return Ok(false);
    }

    // Verify chunk selection
    if !verify_chunk_selection_internal(
        commitment.block_hash.clone(),
        total_chunks,
        commitment.selected_chunks.clone(),
        Some(CHUNK_SELECTION_VERSION),
    )? {
        return Ok(false);
    }

    // Verify chunk count and hash count match
    if commitment.selected_chunks.len() != commitment.chunk_hashes.len() {
        return Ok(false);
    }

    if commitment.selected_chunks.len() != CHUNKS_PER_BLOCK as usize {
        return Ok(false);
    }

    // Verify all chunk indices are valid
    for &chunk_idx in &commitment.selected_chunks {
        if validate_chunk_index(chunk_idx, total_chunks as u64).is_err() {
            return Ok(false);
        }
    }

    // Verify all chunk hashes are correct size
    for chunk_hash in &commitment.chunk_hashes {
        if chunk_hash.len() != HASH_SIZE {
            return Ok(false);
        }
    }

    Ok(true)
}
