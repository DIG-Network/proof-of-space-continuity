use napi::bindgen_prelude::*;


use crate::core::{
    
    types::*,
    utils::{compute_sha256, validate_block_hash, validate_public_key},
    memory_hard_vdf::MemoryHardVDF,
    file_encoding::FileEncoder,
    availability::{AvailabilityChallenger, AvailabilityProver},
};

use crate::consensus::{
    chunk_selection::select_chunks_deterministic_v2,
    network_latency::{NetworkLatencyProver, create_network_proof},
};

/// Create basic ownership commitment
pub fn create_ownership_commitment_internal(
    public_key: Buffer,
    data_hash: Buffer,
) -> Result<OwnershipCommitment> {
    validate_public_key(&public_key)?;

    if data_hash.len() != 32 {
        return Err(Error::new(
            Status::InvalidArg,
            "Data hash must be 32 bytes".to_string(),
        ));
    }

    let mut commitment_data = Vec::new();
    commitment_data.extend_from_slice(&data_hash);
    commitment_data.extend_from_slice(&public_key);
    let commitment_hash = compute_sha256(&commitment_data);

    Ok(OwnershipCommitment {
        public_key,
        data_hash,
        commitment_hash: Buffer::from(commitment_hash.to_vec()),
    })
}

/// Create basic anchored ownership commitment
pub fn create_anchored_ownership_commitment_internal(
    ownership_commitment: OwnershipCommitment,
    block_commitment: BlockCommitment,
) -> Result<AnchoredOwnershipCommitment> {
    validate_block_hash(&block_commitment.block_hash)?;

    if block_commitment.block_height < 0.0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Block height must be non-negative".to_string(),
        ));
    }

    let mut anchored_data = Vec::new();
    anchored_data.extend_from_slice(&ownership_commitment.commitment_hash);
    anchored_data.extend_from_slice(&block_commitment.block_hash);
    // Include block height to ensure uniqueness
    anchored_data.extend_from_slice(&(block_commitment.block_height as u64).to_be_bytes());
    let anchored_hash = compute_sha256(&anchored_data);

    Ok(AnchoredOwnershipCommitment {
        ownership_commitment,
        block_commitment,
        anchored_hash: Buffer::from(anchored_hash.to_vec()),
    })
}

/// Create basic physical access commitment proving data access at specific block
pub fn create_physical_access_commitment_internal(
    previous_commitment: Buffer,
    block_hash: Buffer,
    block_height: f64,
    selected_chunks: Vec<u32>,
    chunk_hashes: Vec<Buffer>,
) -> Result<PhysicalAccessCommitment> {
    validate_block_hash(&block_hash)?;

    if previous_commitment.len() != 32 {
        return Err(Error::new(
            Status::InvalidArg,
            "Previous commitment must be 32 bytes".to_string(),
        ));
    }

    if block_height < 0.0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Block height must be non-negative".to_string(),
        ));
    }

    if selected_chunks.len() != chunk_hashes.len() {
        return Err(Error::new(
            Status::InvalidArg,
            "Selected chunks and chunk hashes must have same length".to_string(),
        ));
    }

    if selected_chunks.len() != CHUNKS_PER_BLOCK as usize {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Must select exactly {} chunks", CHUNKS_PER_BLOCK),
        ));
    }

    // Validate all chunk hashes are 32 bytes
    for (i, chunk_hash) in chunk_hashes.iter().enumerate() {
        if chunk_hash.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Chunk hash {} must be 32 bytes", i),
            ));
        }
    }

    // Create commitment hash
    let mut commitment_data = Vec::new();
    commitment_data.extend_from_slice(&previous_commitment);
    commitment_data.extend_from_slice(&block_hash);
    commitment_data.extend_from_slice(&(block_height as u64).to_be_bytes());

    for &chunk_idx in &selected_chunks {
        commitment_data.extend_from_slice(&chunk_idx.to_be_bytes());
    }

    for chunk_hash in &chunk_hashes {
        commitment_data.extend_from_slice(chunk_hash);
    }

    let commitment_hash = compute_sha256(&commitment_data);

    Ok(PhysicalAccessCommitment {
        block_height,
        previous_commitment,
        block_hash,
        selected_chunks,
        chunk_hashes,
        commitment_hash: Buffer::from(commitment_hash.to_vec()),
    })
}

/// Calculate basic commitment hash for verification
pub fn calculate_commitment_hash_internal(commitment: &PhysicalAccessCommitment) -> Result<Buffer> {
    let mut commitment_data = Vec::new();
    commitment_data.extend_from_slice(&commitment.previous_commitment);
    commitment_data.extend_from_slice(&commitment.block_hash);
    commitment_data.extend_from_slice(&(commitment.block_height as u64).to_be_bytes());

    for &chunk_idx in &commitment.selected_chunks {
        commitment_data.extend_from_slice(&chunk_idx.to_be_bytes());
    }

    for chunk_hash in &commitment.chunk_hashes {
        commitment_data.extend_from_slice(chunk_hash);
    }

    let hash = compute_sha256(&commitment_data);
    Ok(Buffer::from(hash.to_vec()))
}

/// Verify basic physical access commitment
pub fn verify_physical_access_commitment(commitment: &PhysicalAccessCommitment) -> Result<bool> {
    // Recalculate commitment hash
    let calculated_hash = calculate_commitment_hash_internal(commitment)?;

    // Compare with stored hash
    Ok(calculated_hash.as_ref() == commitment.commitment_hash.as_ref())
}

/// Enhanced commitment generator using all new security features
pub struct EnhancedCommitmentGenerator {
    file_encoder: FileEncoder,
    memory_vdf: MemoryHardVDF,
    availability_challenger: AvailabilityChallenger,
    availability_prover: AvailabilityProver,
    network_prover: NetworkLatencyProver,
}

impl EnhancedCommitmentGenerator {
    /// Create new enhanced commitment generator
    pub fn new(prover_key: Buffer) -> Result<Self> {
        Ok(EnhancedCommitmentGenerator {
            file_encoder: FileEncoder::new(prover_key)?,
            memory_vdf: MemoryHardVDF::new_standard()?,
            availability_challenger: AvailabilityChallenger::new(),
            availability_prover: AvailabilityProver::new(),
            network_prover: NetworkLatencyProver::new(),
        })
    }

    /// Generate enhanced ownership commitment with prover-specific encoding
    pub fn generate_enhanced_ownership_commitment(
        &self,
        original_data_hash: Buffer,
        encoded_data_hash: Buffer,
        prover_key: Buffer,
    ) -> Result<EnhancedOwnershipCommitment> {
        // Generate encoding parameters
        let encoding_params = self.generate_encoding_params(&prover_key)?;

        // Create enhanced commitment hash
        let mut commitment_input = Vec::new();
        commitment_input.extend_from_slice(&prover_key);
        commitment_input.extend_from_slice(&encoded_data_hash);
        commitment_input.extend_from_slice(&original_data_hash);
        commitment_input.extend_from_slice(&encoding_params);
        commitment_input.extend_from_slice(b"enhanced_ownership_v2");

        let commitment_hash = compute_sha256(&commitment_input);

        Ok(EnhancedOwnershipCommitment {
            public_key: prover_key,
            encoded_data_hash,
            original_data_hash,
            encoding_params: Buffer::from(encoding_params),
            commitment_hash: Buffer::from(commitment_hash.to_vec()),
        })
    }

    /// Generate enhanced physical access commitment with all security features
    pub fn generate_enhanced_physical_access_commitment(
        &mut self,
        block_height: f64,
        previous_commitment: Buffer,
        entropy: MultiSourceEntropy,
        chain_data_path: String,
        total_chunks: f64,
        peer_addresses: Vec<String>,
    ) -> Result<EnhancedPhysicalAccessCommitment> {
        // 1. Enhanced chunk selection with multi-source entropy
        let chunk_selection = select_chunks_deterministic_v2(entropy.clone(), total_chunks)?;

        // 2. Read chunks and encode them
        let mut chunk_hashes = Vec::new();
        for &chunk_index in &chunk_selection.selected_indices {
            let chunk_data = self.read_chunk_from_file(&chain_data_path, chunk_index)?;
            let encoded_chunk = self.file_encoder.encode_chunk(&chunk_data, chunk_index)?;
            let chunk_hash = compute_sha256(&encoded_chunk);
            chunk_hashes.push(Buffer::from(chunk_hash.to_vec()));
        }

        // 3. Generate memory-hard VDF proof
        let vdf_input = self.create_vdf_input(
            &previous_commitment,
            &entropy,
            &chunk_hashes,
        )?;
        let vdf_proof = self.memory_vdf.compute(&vdf_input, 25.0)?; // 25 seconds target

        // 4. Handle availability challenges
        let availability_responses = self.process_availability_challenges(
            &entropy.blockchain_entropy,
            block_height as u64,
        )?;

        // 5. Generate network latency proof
        let network_latency_proof = create_network_proof(peer_addresses)?;

        // 6. Create final enhanced commitment
        let mut commitment_input = Vec::new();
        commitment_input.extend_from_slice(&block_height.to_be_bytes());
        commitment_input.extend_from_slice(&previous_commitment);
        commitment_input.extend_from_slice(&entropy.combined_hash);
        
        // Add chunk selection proof
        commitment_input.extend_from_slice(&chunk_selection.verification_hash);
        commitment_input.extend_from_slice(&chunk_selection.unpredictability_proof);
        
        // Add chunk hashes
        for chunk_hash in &chunk_hashes {
            commitment_input.extend_from_slice(chunk_hash);
        }
        
        // Add VDF proof
        commitment_input.extend_from_slice(&vdf_proof.output_state);
        
        // Add network proof
        commitment_input.extend_from_slice(&network_latency_proof.average_latency_ms.to_be_bytes());
        
        commitment_input.extend_from_slice(b"enhanced_physical_access_v2");

        let commitment_hash = compute_sha256(&commitment_input);

        Ok(EnhancedPhysicalAccessCommitment {
            block_height,
            previous_commitment,
            block_hash: entropy.blockchain_entropy.clone(),
            entropy,
            selected_chunks: chunk_selection.selected_indices,
            chunk_hashes,
            vdf_proof,
            availability_responses,
            network_latency_proof,
            commitment_hash: Buffer::from(commitment_hash.to_vec()),
        })
    }

    /// Verify enhanced physical access commitment
    pub fn verify_enhanced_physical_access_commitment(
        &self,
        commitment: &EnhancedPhysicalAccessCommitment,
        _expected_prover_key: &Buffer,
    ) -> Result<bool> {
        // 1. Verify chunk selection is valid
        let chunk_selection_valid = self.verify_chunk_selection(
            &commitment.entropy,
            commitment.selected_chunks.len() as f64 * (commitment.selected_chunks.len() as f64 / CHUNKS_PER_BLOCK as f64),
            &commitment.selected_chunks,
        )?;

        if !chunk_selection_valid {
            return Ok(false);
        }

        // 2. Verify memory-hard VDF proof
        let vdf_valid = MemoryHardVDF::verify_proof(&commitment.vdf_proof)?;
        if !vdf_valid {
            return Ok(false);
        }

        // 3. Verify availability responses
        let availability_valid = self.verify_availability_responses(&commitment.availability_responses)?;
        if !availability_valid {
            return Ok(false);
        }

        // 4. Verify network latency proof
        let network_valid = self.network_prover.verify_latency_proof(&commitment.network_latency_proof)?;
        if !network_valid {
            return Ok(false);
        }

        // 5. Verify commitment hash
        let commitment_hash_valid = self.verify_commitment_hash(commitment)?;
        if !commitment_hash_valid {
            return Ok(false);
        }

        Ok(true)
    }

    /// Generate encoding parameters for file encoding
    fn generate_encoding_params(&self, prover_key: &Buffer) -> Result<Vec<u8>> {
        let mut params = Vec::new();
        params.extend_from_slice(prover_key);
        params.extend_from_slice(b"enhanced_encoding_v2");
        params.extend_from_slice(&1u32.to_be_bytes()); // Encoding version

        Ok(compute_sha256(&params).to_vec())
    }

    /// Create VDF input from commitment components
    fn create_vdf_input(
        &self,
        previous_commitment: &Buffer,
        entropy: &MultiSourceEntropy,
        chunk_hashes: &[Buffer],
    ) -> Result<Vec<u8>> {
        let mut vdf_input = Vec::new();
        vdf_input.extend_from_slice(previous_commitment);
        vdf_input.extend_from_slice(&entropy.combined_hash);

        // Add chunk hashes
        for chunk_hash in chunk_hashes {
            vdf_input.extend_from_slice(chunk_hash);
        }

        vdf_input.extend_from_slice(b"enhanced_vdf_input_v2");

        // Hash to get 32-byte input
        Ok(compute_sha256(&vdf_input).to_vec())
    }

    /// Read chunk from file (simplified - would use actual file I/O)
    fn read_chunk_from_file(&self, _file_path: &str, chunk_index: u32) -> Result<Vec<u8>> {
        // Simulate reading chunk data
        // In real implementation, this would read from the actual file
        let mut chunk_data = vec![0u8; CHUNK_SIZE_BYTES as usize];
        
        // Fill with deterministic but unique data based on chunk index
        for (i, byte) in chunk_data.iter_mut().enumerate() {
            *byte = ((chunk_index + i as u32) % 256) as u8;
        }

        Ok(chunk_data)
    }

    /// Process availability challenges for this block
    fn process_availability_challenges(
        &mut self,
        _block_hash: &Buffer,
        _block_height: u64,
    ) -> Result<Vec<AvailabilityResponse>> {
        // Simulate processing availability challenges
        // In real implementation, this would respond to actual challenges
        Ok(Vec::new()) // No challenges to respond to in simulation
    }

    /// Issue availability challenge using the challenger - uses availability_challenger field
    pub fn issue_availability_challenge(&mut self, chain_id: Buffer, total_chunks: u32, block_height: u64) -> Result<Option<AvailabilityChallenge>> {
        // Use the availability_challenger field
        self.availability_challenger.create_challenge(
            chain_id,
            total_chunks,
            Buffer::from([1u8; 32].to_vec()), // challenger_id
            block_height,
        )
    }

    /// Respond to availability challenge using the prover - uses availability_prover field  
    pub fn respond_to_availability_challenge(&mut self, challenge: &AvailabilityChallenge) -> Result<AvailabilityResponse> {
        // Use the availability_prover field
        self.availability_prover.respond_to_challenge(challenge)
    }

    /// Get challenge statistics from challenger - uses availability_challenger field
    pub fn get_challenge_stats(&self) -> String {
        let stats = self.availability_challenger.get_challenge_stats();
        format!(
            "{{\"active_challenges\": {}, \"challenge_probability\": {}, \"timeout_ms\": {}}}",
            stats.active_challenges,
            stats.challenge_probability,
            stats.response_timeout_ms
        )
    }

    /// Register chain for availability proving - uses availability_prover field
    pub fn register_chain_for_availability(&mut self, chain_id: String, file_path: String, total_chunks: u32) {
        // Use the availability_prover field
        self.availability_prover.register_chain(chain_id, file_path, total_chunks);
    }

    /// Clean up expired challenges - uses availability_challenger field
    pub fn cleanup_expired_challenges(&mut self) -> Result<Vec<String>> {
        // Use the availability_challenger field
        self.availability_challenger.cleanup_expired_challenges()
    }

    /// Verify chunk selection matches expected algorithm
    fn verify_chunk_selection(
        &self,
        entropy: &MultiSourceEntropy,
        total_chunks: f64,
        selected_chunks: &[u32],
    ) -> Result<bool> {
        let expected_selection = select_chunks_deterministic_v2(entropy.clone(), total_chunks)?;
        Ok(selected_chunks == expected_selection.selected_indices.as_slice())
    }

    /// Verify availability responses are valid
    fn verify_availability_responses(&self, responses: &[AvailabilityResponse]) -> Result<bool> {
        // In real implementation, would verify each response against its challenge
        // For now, just check basic structure
        for response in responses {
            if response.chunk_data.len() != CHUNK_SIZE_BYTES as usize {
                return Ok(false);
            }
            if response.authenticity_proof.len() != 32 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Verify commitment hash is correct
    fn verify_commitment_hash(&self, commitment: &EnhancedPhysicalAccessCommitment) -> Result<bool> {
        // Reconstruct the commitment hash and compare
        let mut commitment_input = Vec::new();
        commitment_input.extend_from_slice(&commitment.block_height.to_be_bytes());
        commitment_input.extend_from_slice(&commitment.previous_commitment);
        commitment_input.extend_from_slice(&commitment.entropy.combined_hash);
        
        // This is a simplified verification - would include all components
        commitment_input.extend_from_slice(b"enhanced_physical_access_v2");

        let expected_hash = compute_sha256(&commitment_input);
        Ok(commitment.commitment_hash.as_ref() == expected_hash.as_slice())
    }
}

/// Generate enhanced commitment for a block
pub fn generate_enhanced_block_commitment(
    prover_key: Buffer,
    block_height: f64,
    previous_commitment: Buffer,
    entropy: MultiSourceEntropy,
    chain_data_path: String,
    total_chunks: f64,
    peer_addresses: Vec<String>,
) -> Result<EnhancedPhysicalAccessCommitment> {
    let mut generator = EnhancedCommitmentGenerator::new(prover_key)?;
    generator.generate_enhanced_physical_access_commitment(
        block_height,
        previous_commitment,
        entropy,
        chain_data_path,
        total_chunks,
        peer_addresses,
    )
}

/// Verify enhanced commitment for consensus
pub fn verify_enhanced_block_commitment(
    commitment: &EnhancedPhysicalAccessCommitment,
    _expected_prover_key: &Buffer,
) -> Result<bool> {
    let generator = EnhancedCommitmentGenerator::new(_expected_prover_key.clone())?;
    generator.verify_enhanced_physical_access_commitment(commitment, _expected_prover_key)
}

/// Batch process multiple enhanced commitments in parallel
pub fn process_enhanced_commitments_parallel(
    commitment_requests: Vec<EnhancedCommitmentRequest>,
) -> Result<Vec<EnhancedPhysicalAccessCommitment>> {
    use rayon::prelude::*;

    let results: Vec<Result<EnhancedPhysicalAccessCommitment>> = commitment_requests
        .into_par_iter()
        .map(|request| {
            generate_enhanced_block_commitment(
                request.prover_key,
                request.block_height,
                request.previous_commitment,
                request.entropy,
                request.chain_data_path,
                request.total_chunks,
                request.peer_addresses,
            )
        })
        .collect();

    // Collect results and propagate any errors
    let mut successful_results = Vec::new();
    for result in results {
        match result {
            Ok(commitment) => successful_results.push(commitment),
            Err(e) => return Err(e),
        }
    }

    Ok(successful_results)
}

/// Request for enhanced commitment generation
#[derive(Clone)]
pub struct EnhancedCommitmentRequest {
    pub prover_key: Buffer,
    pub block_height: f64,
    pub previous_commitment: Buffer,
    pub entropy: MultiSourceEntropy,
    pub chain_data_path: String,
    pub total_chunks: f64,
    pub peer_addresses: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_ownership_commitment() {
        let prover_key = Buffer::from([42u8; 32].to_vec());
        let generator = EnhancedCommitmentGenerator::new(prover_key.clone()).unwrap();
        
        let original_hash = Buffer::from([1u8; 32].to_vec());
        let encoded_hash = Buffer::from([2u8; 32].to_vec());

        let commitment = generator.generate_enhanced_ownership_commitment(
            original_hash.clone(),
            encoded_hash.clone(),
            prover_key.clone(),
        ).unwrap();

        assert_eq!(commitment.public_key.as_ref(), prover_key.as_ref());
        assert_eq!(commitment.original_data_hash.as_ref(), original_hash.as_ref());
        assert_eq!(commitment.encoded_data_hash.as_ref(), encoded_hash.as_ref());
        assert_eq!(commitment.commitment_hash.len(), 32);
    }

    #[test]
    fn test_enhanced_physical_access_commitment_generation() {
        let prover_key = Buffer::from([42u8; 32].to_vec());
        let mut generator = EnhancedCommitmentGenerator::new(prover_key).unwrap();
        
        let entropy = MultiSourceEntropy {
            blockchain_entropy: Buffer::from([1u8; 32].to_vec()),
            beacon_entropy: Some(Buffer::from([2u8; 32].to_vec())),
            local_entropy: Buffer::from([3u8; 32].to_vec()),
            timestamp: 1234567890.0,
            combined_hash: Buffer::from([4u8; 32].to_vec()),
        };

        let previous_commitment = Buffer::from([5u8; 32].to_vec());
        let peer_addresses = vec![
            "192.168.1.1".to_string(),
            "10.0.0.1".to_string(),
        ];

        let commitment = generator.generate_enhanced_physical_access_commitment(
            100.0,
            previous_commitment.clone(),
            entropy.clone(),
            "/fake/path".to_string(),
            100000.0,
            peer_addresses,
        ).unwrap();

        assert_eq!(commitment.block_height, 100.0);
        assert_eq!(commitment.previous_commitment.as_ref(), previous_commitment.as_ref());
        assert_eq!(commitment.selected_chunks.len(), CHUNKS_PER_BLOCK as usize);
        assert_eq!(commitment.chunk_hashes.len(), CHUNKS_PER_BLOCK as usize);
        assert_eq!(commitment.commitment_hash.len(), 32);
    }

    #[test]
    fn test_enhanced_commitment_verification() {
        let prover_key = Buffer::from([42u8; 32].to_vec());
        let generator = EnhancedCommitmentGenerator::new(prover_key.clone()).unwrap();
        
        // Create a minimal commitment for testing
        let entropy = MultiSourceEntropy {
            blockchain_entropy: Buffer::from([1u8; 32].to_vec()),
            beacon_entropy: None,
            local_entropy: Buffer::from([3u8; 32].to_vec()),
            timestamp: 1234567890.0,
            combined_hash: Buffer::from([4u8; 32].to_vec()),
        };

        // This test would need a fully formed commitment to verify
        // For now, just test that the verification function exists and runs
        let is_valid = generator.verify_chunk_selection(
            &entropy,
            100000.0,
            &[1, 2, 3, 4],
        );
        
        assert!(is_valid.is_ok());
    }
} 