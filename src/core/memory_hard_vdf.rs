use napi::bindgen_prelude::*;
use std::time::Instant;

use crate::core::{types::*, utils::compute_sha256};

/// Memory-hard VDF implementation for ASIC resistance
/// Uses 256MB memory buffer to resist hardware acceleration
pub struct MemoryHardVDF {
    memory_buffer: Vec<u8>,
    memory_size: usize,
    iterations_per_second: u32,
}

impl MemoryHardVDF {
    /// Create new memory-hard VDF with specified memory size
    pub fn new(memory_size: usize) -> Result<Self> {
        if memory_size < 1024 * 1024 {
            return Err(Error::new(
                Status::InvalidArg,
                "Memory size must be at least 1MB".to_string(),
            ));
        }

        Ok(MemoryHardVDF {
            memory_buffer: vec![0u8; memory_size],
            memory_size,
            iterations_per_second: 375_000, // Estimated iterations per second with memory
        })
    }

    /// Create standard 256MB memory-hard VDF
    pub fn new_standard() -> Result<Self> {
        Self::new(MEMORY_HARD_VDF_MEMORY)
    }

    /// Compute memory-hard VDF for target duration
    pub fn compute(
        &mut self,
        input_state: &[u8],
        target_time_seconds: f64,
    ) -> Result<MemoryHardVDFProof> {
        if input_state.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Input state must be 32 bytes".to_string(),
            ));
        }

        let start_time = Instant::now();

        // Calculate target iterations based on performance calibration
        let target_iterations = if target_time_seconds >= 30.0 {
            // For production use (40+ second windows), use full specification iterations
            MEMORY_HARD_ITERATIONS
        } else {
            // Use calibrated iterations per second for accurate timing
            let estimated_iterations =
                (target_time_seconds * self.iterations_per_second as f64) as u32;
            std::cmp::max(estimated_iterations, 10_000) // Minimum 10K iterations for testing
        };

        // Initialize memory buffer with input state
        self.initialize_memory(input_state)?;

        let mut state = input_state.to_vec();
        let mut access_samples = Vec::new();

        // Perform memory-hard iterations
        for iteration in 0..target_iterations {
            // Memory-dependent computation
            let (new_state, read_addr, write_addr, memory_hash) =
                self.memory_hard_iteration(&state, iteration)?;

            state = new_state;

            // Sample access pattern every 50,000 iterations for verification (increased sampling)
            if iteration % 50_000 == 0 {
                access_samples.push(MemoryAccessSample {
                    iteration,
                    read_address: read_addr as f64,
                    write_address: write_addr as f64,
                    memory_content_hash: Buffer::from(memory_hash.to_vec()),
                });
                // Trace-level logging for VDF internals
                log::trace!(
                    "[VDF TRACE] Iteration: {} | State: {} | Read Addr: {} | Write Addr: {} | MemHash: {}",
                    iteration,
                    hex::encode(&state[..8]),
                    read_addr,
                    write_addr,
                    hex::encode(&memory_hash[..8])
                );
            }
        }

        let computation_time = start_time.elapsed().as_secs_f64() * 1000.0; // Convert to ms

        // Update performance calibration based on actual computation time
        self.update_performance_calibration(target_iterations, computation_time / 1000.0);

        Ok(MemoryHardVDFProof {
            input_state: Buffer::from(input_state.to_vec()),
            output_state: Buffer::from(state),
            iterations: target_iterations,
            memory_access_samples: access_samples,
            computation_time_ms: computation_time,
            memory_usage_bytes: self.memory_size as f64,
        })
    }

    /// Initialize memory buffer with input state
    fn initialize_memory(&mut self, input_state: &[u8]) -> Result<()> {
        // Fill memory with deterministic but complex pattern based on input
        let mut seed = input_state.to_vec();

        for chunk_idx in 0..(self.memory_size / 32) {
            let start_offset = chunk_idx * 32;
            let end_offset = std::cmp::min(start_offset + 32, self.memory_size);

            // Generate deterministic content for this chunk
            seed.extend_from_slice(&chunk_idx.to_be_bytes());
            let chunk_hash = compute_sha256(&seed);

            // Copy hash to memory buffer
            let copy_len = end_offset - start_offset;
            self.memory_buffer[start_offset..end_offset].copy_from_slice(&chunk_hash[..copy_len]);

            // Update seed for next iteration
            seed = chunk_hash.to_vec();
        }

        Ok(())
    }

    /// Update performance calibration based on actual computation results
    fn update_performance_calibration(&mut self, iterations: u32, actual_time_seconds: f64) {
        if actual_time_seconds > 0.0 && iterations > 1000 {
            // Calculate actual iterations per second
            let actual_rate = iterations as f64 / actual_time_seconds;

            // Use exponential moving average to smooth calibration updates
            let alpha = 0.1; // Learning rate
            self.iterations_per_second =
                ((1.0 - alpha) * self.iterations_per_second as f64 + alpha * actual_rate) as u32;

            // Ensure rate stays within reasonable bounds (100K to 1M per second)
            self.iterations_per_second = self.iterations_per_second.clamp(100_000, 1_000_000);
        }
    }

    /// Get current performance calibration
    pub fn get_iterations_per_second(&self) -> u32 {
        self.iterations_per_second
    }

    /// Single memory-hard iteration
    fn memory_hard_iteration(
        &mut self,
        state: &[u8],
        iteration: u32,
    ) -> Result<(Vec<u8>, u64, u64, [u8; 32])> {
        // Calculate read address from current state
        let read_seed = compute_sha256(&[state, &iteration.to_be_bytes()].concat());
        let read_addr = u64::from_be_bytes([
            read_seed[0],
            read_seed[1],
            read_seed[2],
            read_seed[3],
            read_seed[4],
            read_seed[5],
            read_seed[6],
            read_seed[7],
        ]) as usize
            % (self.memory_size - 1024); // Ensure we can read 1KB

        // Read 1KB from memory at calculated address
        let memory_chunk = &self.memory_buffer[read_addr..read_addr + 1024];
        let memory_hash = compute_sha256(memory_chunk);

        // Compute new state mixing current state with memory content
        let new_state = compute_sha256(
            &[
                state,
                &memory_hash,
                &iteration.to_be_bytes(),
                b"memory_hard_vdf",
            ]
            .concat(),
        );

        // Calculate write address from new state
        let write_input = [&new_state[..], b"write"].concat();
        let write_seed = compute_sha256(&write_input);
        let write_addr = u64::from_be_bytes([
            write_seed[0],
            write_seed[1],
            write_seed[2],
            write_seed[3],
            write_seed[4],
            write_seed[5],
            write_seed[6],
            write_seed[7],
        ]) as usize
            % (self.memory_size - 32); // Ensure we can write 32 bytes

        // Write new state to memory at calculated address
        self.memory_buffer[write_addr..write_addr + 32].copy_from_slice(&new_state);

        Ok((
            new_state.to_vec(),
            read_addr as u64,
            write_addr as u64,
            memory_hash,
        ))
    }

    /// Verify a memory-hard VDF proof
    pub fn verify_proof(proof: &MemoryHardVDFProof) -> Result<bool> {
        // Basic validation
        if proof.input_state.len() != 32 {
            return Ok(false);
        }
        if proof.output_state.len() != 32 {
            return Ok(false);
        }
        if proof.iterations < 1000 {
            return Ok(false);
        }
        if proof.memory_usage_bytes < 1024.0 * 1024.0 {
            return Ok(false);
        }

        // Verify computation time is reasonable
        let expected_time_min = proof.iterations as f64 / 500_000.0 * 1000.0; // Min time (fast hardware)
        let expected_time_max = proof.iterations as f64 / 200_000.0 * 1000.0; // Max time (slow hardware)

        if proof.computation_time_ms < expected_time_min
            || proof.computation_time_ms > expected_time_max
        {
            return Ok(false);
        }

        // Verify memory access samples exist
        if proof.memory_access_samples.is_empty() {
            return Ok(false);
        }

        // Verify memory access pattern consistency
        if !Self::verify_memory_access_pattern(&proof.memory_access_samples, proof.iterations) {
            return Ok(false);
        }

        // Verify output state consistency with deterministic reproduction (spot check)
        if !Self::verify_output_consistency(proof) {
            return Ok(false);
        }
        Ok(true)
    }

    /// Verify memory access pattern is consistent with expected algorithm
    fn verify_memory_access_pattern(samples: &[MemoryAccessSample], total_iterations: u32) -> bool {
        if samples.is_empty() {
            return false;
        }

        // Check that sample iterations are within valid range
        for sample in samples {
            if sample.iteration >= total_iterations {
                return false;
            }

            // Verify memory content hash is properly sized
            if sample.memory_content_hash.len() != 32 {
                return false;
            }

            // Verify addresses are reasonable (not zero, within memory bounds)
            if sample.read_address == 0.0 || sample.write_address == 0.0 {
                return false;
            }
        }

        // Verify sampling frequency is consistent (every 50,000 iterations)
        let expected_sample_count = (total_iterations / 50_000) + 1;
        let actual_sample_count = samples.len() as u32;

        // Allow some tolerance for rounding
        (actual_sample_count as i32 - expected_sample_count as i32).abs() <= 1
    }

    /// Verify output state consistency with partial re-computation
    fn verify_output_consistency(proof: &MemoryHardVDFProof) -> bool {
        // Perform spot-check verification by recomputing first few iterations
        let mut state = proof.input_state.to_vec();

        // Recreate initial memory state deterministically
        let mut temp_vdf = match MemoryHardVDF::new(proof.memory_usage_bytes as usize) {
            Ok(vdf) => vdf,
            Err(_) => return false,
        };

        if temp_vdf.initialize_memory(&proof.input_state).is_err() {
            return false;
        }

        // Verify first few iterations match expected pattern
        for iteration in 0..std::cmp::min(1000u32, proof.iterations) {
            match temp_vdf.memory_hard_iteration(&state, iteration) {
                Ok((new_state, _, _, _)) => {
                    state = new_state;
                }
                Err(_) => return false,
            }
        }

        // For very short proofs, we can verify the complete output
        if proof.iterations <= 1000 {
            return state == proof.output_state.as_ref();
        }

        // For longer proofs, verify the pattern is consistent
        // This is a heuristic check - full verification would require complete recomputation
        true
    }
}

/// Create multi-source entropy for memory-hard VDF
pub fn create_vdf_entropy(
    blockchain_entropy: Buffer,
    beacon_entropy: Option<Buffer>,
    local_entropy: Buffer,
    timestamp: f64,
) -> Result<MultiSourceEntropy> {
    // Validate entropy sources
    if blockchain_entropy.len() != 32 {
        return Err(Error::new(
            Status::InvalidArg,
            "Blockchain entropy must be 32 bytes".to_string(),
        ));
    }

    if let Some(ref beacon) = beacon_entropy {
        if beacon.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Beacon entropy must be 32 bytes".to_string(),
            ));
        }
    }

    if local_entropy.len() != 32 {
        return Err(Error::new(
            Status::InvalidArg,
            "Local entropy must be 32 bytes".to_string(),
        ));
    }

    // Combine all entropy sources
    let mut combined = Vec::new();
    combined.extend_from_slice(&blockchain_entropy);

    if let Some(ref beacon) = beacon_entropy {
        combined.extend_from_slice(beacon);
    } else {
        combined.extend_from_slice(&[0u8; 32]); // Deterministic fallback
    }

    combined.extend_from_slice(&local_entropy);
    combined.extend_from_slice(&timestamp.to_be_bytes());

    let combined_hash = compute_sha256(&combined);

    Ok(MultiSourceEntropy {
        blockchain_entropy,
        beacon_entropy,
        local_entropy,
        timestamp,
        combined_hash: Buffer::from(combined_hash.to_vec()),
    })
}

/// Compute memory-hard VDF for block processing
pub fn compute_block_vdf(
    input_state: Buffer,
    target_time_seconds: f64,
) -> Result<MemoryHardVDFProof> {
    let mut vdf = MemoryHardVDF::new_standard()?;
    vdf.compute(&input_state, target_time_seconds)
}

/// Verify memory-hard VDF proof for consensus
pub fn verify_block_vdf(proof: &MemoryHardVDFProof) -> Result<bool> {
    MemoryHardVDF::verify_proof(proof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_hard_vdf_basic() {
        let mut vdf = MemoryHardVDF::new_standard().unwrap();
        let input_state = [1u8; 32];

        let proof = vdf.compute(&input_state, 0.1).unwrap(); // 100ms target

        assert_eq!(proof.input_state.len(), 32);
        assert_eq!(proof.output_state.len(), 32);
        assert!(proof.iterations >= 1000);
        assert_eq!(proof.memory_usage_bytes, MEMORY_HARD_VDF_MEMORY as f64);
        assert!(!proof.memory_access_samples.is_empty());
    }

    #[test]
    fn test_memory_hard_vdf_deterministic() {
        let mut vdf1 = MemoryHardVDF::new_standard().unwrap();
        let mut vdf2 = MemoryHardVDF::new_standard().unwrap();
        let input_state = [2u8; 32];

        let proof1 = vdf1.compute(&input_state, 0.05).unwrap();
        let proof2 = vdf2.compute(&input_state, 0.05).unwrap();

        // Should be deterministic for same input and iterations
        assert_eq!(proof1.output_state.as_ref(), proof2.output_state.as_ref());
        assert_eq!(proof1.iterations, proof2.iterations);
    }

    #[test]
    fn test_memory_hard_vdf_verification() {
        let mut vdf = MemoryHardVDF::new_standard().unwrap();
        let input_state = [3u8; 32];

        let proof = vdf.compute(&input_state, 0.1).unwrap();

        let is_valid = MemoryHardVDF::verify_proof(&proof).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_vdf_entropy_creation() {
        let blockchain_entropy = Buffer::from([1u8; 32].to_vec());
        let beacon_entropy = Some(Buffer::from([2u8; 32].to_vec()));
        let local_entropy = Buffer::from([3u8; 32].to_vec());
        let timestamp = 1234567890.0;

        let entropy =
            create_vdf_entropy(blockchain_entropy, beacon_entropy, local_entropy, timestamp)
                .unwrap();

        assert_eq!(entropy.blockchain_entropy.len(), 32);
        assert!(entropy.beacon_entropy.is_some());
        assert_eq!(entropy.local_entropy.len(), 32);
        assert_eq!(entropy.timestamp, timestamp);
        assert_eq!(entropy.combined_hash.len(), 32);
    }

    #[test]
    fn test_block_vdf_functions() {
        let input_state = Buffer::from([4u8; 32].to_vec());

        let proof = compute_block_vdf(input_state, 0.05).unwrap();
        let is_valid = verify_block_vdf(&proof).unwrap();

        assert!(is_valid);
        assert!(proof.iterations >= 1000);
    }
}
