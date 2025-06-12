pub mod chunk_selection;
pub mod commitments;
pub mod network_latency;
pub mod verification;

/// Production consensus validation rules for network compliance
pub use chunk_selection::*;
pub use commitments::*;
pub use network_latency::*;
pub use verification::*;

/// Network consensus compliance validator
pub struct NetworkConsensusValidator {
    /// Minimum VDF iterations required for production
    min_vdf_iterations: u32,
    /// Required chunk count per block
    required_chunks_per_block: u32,
    /// Maximum acceptable latency for network proofs
    max_network_latency_ms: f64,
    /// Required memory size for VDF
    required_vdf_memory_mb: u32,
}

impl Default for NetworkConsensusValidator {
    fn default() -> Self {
        Self::new_production()
    }
}

impl NetworkConsensusValidator {
    /// Create production consensus validator with specification parameters
    pub fn new_production() -> Self {
        Self {
            min_vdf_iterations: 1000, // NETWORK CONSENSUS: Minimum 1000 iterations for continuous VDF
            required_chunks_per_block: crate::core::types::CHUNKS_PER_BLOCK,
            max_network_latency_ms: 200.0, // 200ms max for anti-outsourcing
            required_vdf_memory_mb: 0, // NETWORK CONSENSUS: 256KB for continuous VDF (less than 1MB)
        }
    }

    /// Validate commitment complies with network consensus
    pub fn validate_commitment_consensus(
        &self,
        commitment: &crate::core::types::StorageCommitment,
    ) -> Result<(), String> {
        // 1. Validate chunk count matches network standard
        if commitment.selected_chunks.len() != self.required_chunks_per_block as usize {
            return Err(format!(
                "Invalid chunk count: expected {}, got {}",
                self.required_chunks_per_block,
                commitment.selected_chunks.len()
            ));
        }

        // 2. Validate VDF meets minimum requirements
        if commitment.vdf_proof.iterations < self.min_vdf_iterations {
            return Err(format!(
                "VDF iterations below minimum: expected {}, got {}",
                self.min_vdf_iterations, commitment.vdf_proof.iterations
            ));
        }

        // 3. Validate VDF memory usage
        let memory_mb = (commitment.vdf_proof.memory_usage_bytes / (1024.0 * 1024.0)) as u32;
        if memory_mb < self.required_vdf_memory_mb {
            return Err(format!(
                "VDF memory below requirement: expected {}MB, got {}MB",
                self.required_vdf_memory_mb, memory_mb
            ));
        }

        // 4. Validate entropy sources are present
        if commitment.entropy.blockchain_entropy.len() != 32 {
            return Err("Invalid blockchain entropy length".to_string());
        }
        if commitment.entropy.local_entropy.len() != 32 {
            return Err("Invalid local entropy length".to_string());
        }

        // 5. Validate commitment hash integrity
        if commitment.commitment_hash.len() != 32 {
            return Err("Invalid commitment hash length".to_string());
        }

        Ok(())
    }

    /// Validate VDF proof meets consensus requirements
    pub fn validate_vdf_consensus(
        &self,
        vdf_proof: &crate::core::types::MemoryHardVDFProof,
    ) -> Result<(), String> {
        // NETWORK CONSENSUS REQUIREMENT: Validate continuous VDF format

        // 1. Check minimum iterations for continuous VDF
        if vdf_proof.iterations < self.min_vdf_iterations {
            return Err(format!(
                "VDF iterations insufficient: {} < {}",
                vdf_proof.iterations, self.min_vdf_iterations
            ));
        }

        // 2. CRITICAL: Verify this is a continuous VDF proof (not old memory-hard VDF)
        if !vdf_proof.memory_access_samples.is_empty() {
            return Err("NETWORK CONSENSUS VIOLATION: Old memory-hard VDF format not accepted. Must use continuous VDF.".to_string());
        }

        // 3. Verify continuous VDF memory usage (exactly 256KB)
        if vdf_proof.memory_usage_bytes != 256.0 * 1024.0 {
            return Err(format!("NETWORK CONSENSUS VIOLATION: Continuous VDF must use exactly 256KB memory, got {:.0} bytes", 
                vdf_proof.memory_usage_bytes));
        }

        // 4. Verify VDF signature format (output_state must be 32 bytes)
        if vdf_proof.output_state.len() != 32 {
            return Err(
                "NETWORK CONSENSUS VIOLATION: VDF signature must be exactly 32 bytes".to_string(),
            );
        }

        // 5. Verify VDF state format (input_state must be 32 bytes)
        if vdf_proof.input_state.len() != 32 {
            return Err(
                "NETWORK CONSENSUS VIOLATION: VDF state must be exactly 32 bytes".to_string(),
            );
        }

        // 6. Verify continuous VDF timing characteristics
        if vdf_proof.computation_time_ms != 0.0 {
            return Err("NETWORK CONSENSUS VIOLATION: Continuous VDF should not report discrete computation time".to_string());
        }

        Ok(())
    }

    /// Validate network latency proof for anti-outsourcing
    pub fn validate_network_latency_consensus(
        &self,
        proof: &crate::core::types::NetworkLatencyProof,
    ) -> Result<(), String> {
        // Check minimum peer count
        if proof.peer_latencies.len() < crate::core::types::NETWORK_LATENCY_SAMPLES as usize {
            return Err(format!(
                "Insufficient network samples: {} < {}",
                proof.peer_latencies.len(),
                crate::core::types::NETWORK_LATENCY_SAMPLES
            ));
        }

        // Check average latency is within bounds
        if proof.average_latency_ms > self.max_network_latency_ms {
            return Err(format!(
                "Average latency too high: {:.1}ms > {:.1}ms",
                proof.average_latency_ms, self.max_network_latency_ms
            ));
        }

        // Check for suspiciously consistent latencies (anti-outsourcing)
        if proof.latency_variance < 1.0 {
            return Err(
                "Network latency variance too low - possible outsourcing detected".to_string(),
            );
        }

        // Check individual latencies are reasonable
        for measurement in &proof.peer_latencies {
            if measurement.latency_ms < 0.5 || measurement.latency_ms > 2000.0 {
                return Err(format!(
                    "Suspicious latency measurement: {:.1}ms",
                    measurement.latency_ms
                ));
            }
        }

        Ok(())
    }

    /// Validate chunk selection follows consensus algorithm
    pub fn validate_chunk_selection_consensus(
        &self,
        entropy: &crate::core::types::MultiSourceEntropy,
        total_chunks: u32,
        selected_chunks: &[u32],
    ) -> Result<(), String> {
        // Verify chunk count
        if selected_chunks.len() != self.required_chunks_per_block as usize {
            return Err(format!(
                "Wrong chunk count: {} != {}",
                selected_chunks.len(),
                self.required_chunks_per_block
            ));
        }

        // Verify all chunks are within bounds
        for &chunk_idx in selected_chunks {
            if chunk_idx >= total_chunks {
                return Err(format!(
                    "Chunk index out of bounds: {} >= {}",
                    chunk_idx, total_chunks
                ));
            }
        }

        // Verify selection is deterministic
        let expected_chunks = crate::core::utils::select_chunks_deterministic(
            &entropy.combined_hash,
            total_chunks as f64,
            self.required_chunks_per_block,
        );

        if selected_chunks != expected_chunks.as_slice() {
            return Err("Chunk selection does not match consensus algorithm".to_string());
        }

        Ok(())
    }

    /// Comprehensive consensus validation for full commitment
    pub fn validate_full_consensus(
        &self,
        commitment: &crate::core::types::StorageCommitment,
        total_chunks: u32,
    ) -> Result<(), String> {
        // Run all consensus validations
        self.validate_commitment_consensus(commitment)?;
        self.validate_vdf_consensus(&commitment.vdf_proof)?;
        self.validate_chunk_selection_consensus(
            &commitment.entropy,
            total_chunks,
            &commitment.selected_chunks,
        )?;

        Ok(())
    }
}
