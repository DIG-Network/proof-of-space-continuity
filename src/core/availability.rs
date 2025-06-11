use napi::bindgen_prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

use crate::core::{
    
    types::*,
    utils::compute_sha256,
};

/// Availability challenge system to ensure data is served, not just stored
pub struct AvailabilityChallenger {
    challenge_probability: f64,
    response_timeout_ms: u32,
    active_challenges: HashMap<String, AvailabilityChallenge>,
}

impl AvailabilityChallenger {
    /// Create new availability challenger
    pub fn new() -> Self {
        AvailabilityChallenger {
            challenge_probability: AVAILABILITY_CHALLENGE_PROBABILITY,
            response_timeout_ms: AVAILABILITY_RESPONSE_TIME_MS,
            active_challenges: HashMap::new(),
        }
    }

    /// Create availability challenge for a chain
    pub fn create_challenge(
        &mut self,
        chain_id: Buffer,
        total_chunks: u32,
        challenger_id: Buffer,
        block_height: u64,
    ) -> Result<Option<AvailabilityChallenge>> {
        // Determine if this chain should be challenged this block
        if !self.should_challenge_chain(&chain_id, block_height)? {
            return Ok(None);
        }

        // Select random chunk to challenge
        let chunk_index = self.select_challenge_chunk(&chain_id, total_chunks, block_height)?;

        // Generate challenge nonce
        let challenge_nonce = self.generate_challenge_nonce(&chain_id, chunk_index, block_height)?;

        // Get current timestamp
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Time error: {}", e)))?
            .as_secs_f64();

        let challenge = AvailabilityChallenge {
            chain_id: chain_id.clone(),
            chunk_index,
            challenge_nonce: Buffer::from(challenge_nonce.to_vec()),
            challenger_id,
            challenge_time: current_time,
            deadline: current_time + (self.response_timeout_ms as f64 / 1000.0),
            reward_amount: AVAILABILITY_REWARD_UNITS as f64,
        };

        // Store active challenge
        let challenge_id = self.compute_challenge_id(&challenge)?;
        self.active_challenges.insert(challenge_id, challenge.clone());

        Ok(Some(challenge))
    }

    /// Process response to availability challenge
    pub fn process_response(
        &mut self,
        challenge_id: String,
        response: AvailabilityResponse,
    ) -> Result<AvailabilityResult> {
        // Find the challenge
        let challenge = self.active_challenges.get(&challenge_id)
            .ok_or_else(|| Error::new(Status::GenericFailure, "Challenge not found".to_string()))?
            .clone();

        // Check if response is within deadline
        let _current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Time error: {}", e)))?
            .as_secs_f64();

        if response.response_time > challenge.deadline {
            // Timeout - prover failed
            self.active_challenges.remove(&challenge_id);
            return Ok(AvailabilityResult::Timeout);
        }

        // Verify chunk data authenticity
        if !self.verify_chunk_authenticity(&challenge, &response)? {
            // Invalid data - prover failed
            self.active_challenges.remove(&challenge_id);
            return Ok(AvailabilityResult::InvalidData);
        }

        // Success - prover responded correctly and on time
        self.active_challenges.remove(&challenge_id);
        Ok(AvailabilityResult::Success {
            response_time_ms: ((response.response_time - challenge.challenge_time) * 1000.0) as u32,
            challenger_reward: challenge.reward_amount,
        })
    }

    /// Determine if chain should be challenged this block
    fn should_challenge_chain(&self, chain_id: &Buffer, block_height: u64) -> Result<bool> {
        // Create deterministic but unpredictable decision
        let mut challenge_seed = Vec::new();
        challenge_seed.extend_from_slice(chain_id);
        challenge_seed.extend_from_slice(&block_height.to_be_bytes());
        challenge_seed.extend_from_slice(b"availability_challenge");

        let challenge_hash = compute_sha256(&challenge_seed);
        let challenge_value = u64::from_be_bytes([
            challenge_hash[0], challenge_hash[1], challenge_hash[2], challenge_hash[3],
            challenge_hash[4], challenge_hash[5], challenge_hash[6], challenge_hash[7],
        ]);

        // Convert probability to threshold
        let threshold = (self.challenge_probability * u64::MAX as f64) as u64;
        
        Ok(challenge_value < threshold)
    }

    /// Select chunk to challenge
    fn select_challenge_chunk(
        &self,
        chain_id: &Buffer,
        total_chunks: u32,
        block_height: u64,
    ) -> Result<u32> {
        let mut chunk_seed = Vec::new();
        chunk_seed.extend_from_slice(chain_id);
        chunk_seed.extend_from_slice(&block_height.to_be_bytes());
        chunk_seed.extend_from_slice(b"challenge_chunk_selection");

        let chunk_hash = compute_sha256(&chunk_seed);
        let chunk_value = u32::from_be_bytes([
            chunk_hash[0], chunk_hash[1], chunk_hash[2], chunk_hash[3],
        ]);

        Ok(chunk_value % total_chunks)
    }

    /// Generate challenge nonce
    fn generate_challenge_nonce(
        &self,
        chain_id: &Buffer,
        chunk_index: u32,
        block_height: u64,
    ) -> Result<[u8; 32]> {
        let mut nonce_seed = Vec::new();
        nonce_seed.extend_from_slice(chain_id);
        nonce_seed.extend_from_slice(&chunk_index.to_be_bytes());
        nonce_seed.extend_from_slice(&block_height.to_be_bytes());
        nonce_seed.extend_from_slice(b"challenge_nonce");

        Ok(compute_sha256(&nonce_seed))
    }

    /// Compute unique challenge ID
    fn compute_challenge_id(&self, challenge: &AvailabilityChallenge) -> Result<String> {
        let mut id_input = Vec::new();
        id_input.extend_from_slice(&challenge.chain_id);
        id_input.extend_from_slice(&challenge.chunk_index.to_be_bytes());
        id_input.extend_from_slice(&challenge.challenge_nonce);
        id_input.extend_from_slice(&challenge.challenger_id);

        let id_hash = compute_sha256(&id_input);
        Ok(hex::encode(id_hash))
    }

    /// Verify chunk data authenticity
    fn verify_chunk_authenticity(
        &self,
        challenge: &AvailabilityChallenge,
        response: &AvailabilityResponse,
    ) -> Result<bool> {
        // Verify chunk data length
        if response.chunk_data.len() != CHUNK_SIZE_BYTES as usize {
            return Ok(false);
        }

        // Verify authenticity proof
        // This would typically involve checking the chunk hash against stored Merkle proofs
        // For this implementation, we'll check that the proof contains the expected challenge nonce
        if response.authenticity_proof.len() < 32 {
            return Ok(false);
        }

        // Verify proof contains challenge nonce (simplified verification)
        let expected_proof = self.compute_expected_proof(challenge, &response.chunk_data)?;
        
        Ok(response.authenticity_proof.as_ref() == expected_proof.as_slice())
    }

    /// Compute expected authenticity proof
    fn compute_expected_proof(&self, challenge: &AvailabilityChallenge, chunk_data: &[u8]) -> Result<Vec<u8>> {
        let mut proof_input = Vec::new();
        proof_input.extend_from_slice(&challenge.challenge_nonce);
        proof_input.extend_from_slice(chunk_data);
        proof_input.extend_from_slice(&challenge.chain_id);
        proof_input.extend_from_slice(b"authenticity_proof");

        Ok(compute_sha256(&proof_input).to_vec())
    }

    /// Clean up expired challenges
    pub fn cleanup_expired_challenges(&mut self) -> Result<Vec<String>> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Time error: {}", e)))?
            .as_secs_f64();

        let mut expired_challenges = Vec::new();

        self.active_challenges.retain(|challenge_id, challenge| {
            if current_time > challenge.deadline {
                expired_challenges.push(challenge_id.clone());
                false
            } else {
                true
            }
        });

        Ok(expired_challenges)
    }

    /// Get statistics about challenges
    pub fn get_challenge_stats(&self) -> ChallengeStats {
        ChallengeStats {
            active_challenges: self.active_challenges.len() as u32,
            challenge_probability: self.challenge_probability,
            response_timeout_ms: self.response_timeout_ms,
        }
    }
}

/// Result of processing availability challenge
#[derive(Debug, Clone)]
pub enum AvailabilityResult {
    Success {
        response_time_ms: u32,
        challenger_reward: f64,
    },
    Timeout,
    InvalidData,
}

/// Challenge statistics
#[derive(Debug, Clone)]
pub struct ChallengeStats {
    pub active_challenges: u32,
    pub challenge_probability: f64,
    pub response_timeout_ms: u32,
}

/// Chain storage statistics
#[derive(Debug, Clone)]
pub struct ChainStorageStats {
    pub total_chunks: u32,
    pub cached_chunks: u32,
    pub file_path: String,
    pub cache_hit_ratio: f64,
}

/// Availability prover for responding to challenges
pub struct AvailabilityProver {
    chain_data: HashMap<String, ChainAvailabilityData>,
}

#[derive(Clone)]
struct ChainAvailabilityData {
    file_path: String,
    total_chunks: u32,
    chunk_cache: HashMap<u32, Vec<u8>>, // Cache recently accessed chunks
}

impl AvailabilityProver {
    /// Create new availability prover
    pub fn new() -> Self {
        AvailabilityProver {
            chain_data: HashMap::new(),
        }
    }

    /// Register chain for availability proving
    pub fn register_chain(&mut self, chain_id: String, file_path: String, total_chunks: u32) {
        let chain_data = ChainAvailabilityData {
            file_path,
            total_chunks,
            chunk_cache: HashMap::new(),
        };
        self.chain_data.insert(chain_id, chain_data);
    }

    /// Respond to availability challenge
    pub fn respond_to_challenge(&mut self, challenge: &AvailabilityChallenge) -> Result<AvailabilityResponse> {
        let chain_id = hex::encode(&challenge.chain_id);
        
        // Read chunk data - separate the operations to avoid borrowing conflicts
        let chunk_data = self.read_chunk_for_chain(&chain_id, challenge.chunk_index)?;

        // Generate authenticity proof
        let authenticity_proof = self.generate_authenticity_proof(challenge, &chunk_data)?;

        // Get current timestamp
        let response_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Time error: {}", e)))?
            .as_secs_f64();

        // Generate challenge ID
        let challenge_id = self.compute_challenge_id(challenge)?;

        Ok(AvailabilityResponse {
            challenge_id: Buffer::from(challenge_id),
            chunk_data: Buffer::from(chunk_data),
            response_time,
            authenticity_proof: Buffer::from(authenticity_proof),
        })
    }

    /// Validate chunk index against total chunks - uses total_chunks field
    pub fn validate_chunk_index(&self, chain_id: &str, chunk_index: u32) -> bool {
        if let Some(chain_data) = self.chain_data.get(chain_id) {
            // Use the total_chunks field for validation
            chunk_index < chain_data.total_chunks
        } else {
            false
        }
    }

    /// Get total chunks for a chain - uses total_chunks field
    pub fn get_total_chunks(&self, chain_id: &str) -> Option<u32> {
        self.chain_data.get(chain_id).map(|chain_data| chain_data.total_chunks)
    }

    /// Validate challenge bounds - uses total_chunks field
    pub fn validate_challenge_bounds(&self, chain_id: &str, challenge: &AvailabilityChallenge) -> bool {
        if let Some(chain_data) = self.chain_data.get(chain_id) {
            // Use total_chunks field to validate challenge is within bounds
            challenge.chunk_index < chain_data.total_chunks
        } else {
            false
        }
    }

    /// Get storage statistics for chain - uses total_chunks field
    pub fn get_chain_storage_stats(&self, chain_id: &str) -> Option<ChainStorageStats> {
        self.chain_data.get(chain_id).map(|chain_data| {
            ChainStorageStats {
                total_chunks: chain_data.total_chunks,
                cached_chunks: chain_data.chunk_cache.len() as u32,
                file_path: chain_data.file_path.clone(),
                cache_hit_ratio: if chain_data.total_chunks > 0 {
                    chain_data.chunk_cache.len() as f64 / chain_data.total_chunks as f64
                } else {
                    0.0
                },
            }
        })
    }

    /// Validate chunk range request - uses total_chunks field
    pub fn validate_chunk_range(&self, chain_id: &str, start_chunk: u32, end_chunk: u32) -> bool {
        if let Some(chain_data) = self.chain_data.get(chain_id) {
            // Use total_chunks field to validate range is within bounds
            start_chunk <= end_chunk && 
            end_chunk < chain_data.total_chunks &&
            start_chunk < chain_data.total_chunks
        } else {
            false
        }
    }

    /// Read chunk for a specific chain
    fn read_chunk_for_chain(&mut self, chain_id: &str, chunk_index: u32) -> Result<Vec<u8>> {
        // Check if chain exists first
        if !self.chain_data.contains_key(chain_id) {
            return Err(Error::new(Status::GenericFailure, "Chain not found".to_string()));
        }

        // Check cache first (avoid mutable borrow)
        if let Some(cached_chunk) = self.chain_data.get(chain_id)
            .and_then(|chain_data| chain_data.chunk_cache.get(&chunk_index)) {
            return Ok(cached_chunk.clone());
        }

        // Get file path for reading
        let file_path = self.chain_data.get(chain_id)
            .ok_or_else(|| Error::new(Status::GenericFailure, "Chain not found".to_string()))?
            .file_path.clone();

        // Read chunk data from file
        let chunk_data = self.read_chunk_from_file(&file_path, chunk_index)?;

        // Now update cache with mutable borrow
        if let Some(chain_data) = self.chain_data.get_mut(chain_id) {
            chain_data.chunk_cache.insert(chunk_index, chunk_data.clone());

            // Limit cache size
            if chain_data.chunk_cache.len() > 100 {
                // Remove oldest entries (simplified LRU)
                let keys_to_remove: Vec<_> = chain_data.chunk_cache.keys()
                    .take(20)
                    .cloned()
                    .collect();
                for key in keys_to_remove {
                    chain_data.chunk_cache.remove(&key);
                }
            }
        }

        Ok(chunk_data)
    }

    /// Read chunk directly from file
    fn read_chunk_from_file(&self, file_path: &str, chunk_index: u32) -> Result<Vec<u8>> {
        use std::fs::File;
        use std::io::{Read, Seek, SeekFrom};

        let mut file = File::open(file_path)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to open file: {}", e)))?;

        let chunk_offset = chunk_index as u64 * CHUNK_SIZE_BYTES as u64;
        file.seek(SeekFrom::Start(chunk_offset))
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to seek: {}", e)))?;

        let mut chunk_data = vec![0u8; CHUNK_SIZE_BYTES as usize];
        let bytes_read = file.read(&mut chunk_data)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to read: {}", e)))?;

        // Handle partial last chunk
        if bytes_read < CHUNK_SIZE_BYTES as usize {
            chunk_data.resize(bytes_read, 0);
        }

        Ok(chunk_data)
    }

    /// Generate authenticity proof for chunk
    fn generate_authenticity_proof(&self, challenge: &AvailabilityChallenge, chunk_data: &[u8]) -> Result<Vec<u8>> {
        let mut proof_input = Vec::new();
        proof_input.extend_from_slice(&challenge.challenge_nonce);
        proof_input.extend_from_slice(chunk_data);
        proof_input.extend_from_slice(&challenge.chain_id);
        proof_input.extend_from_slice(b"authenticity_proof");

        Ok(compute_sha256(&proof_input).to_vec())
    }

    /// Compute challenge ID
    fn compute_challenge_id(&self, challenge: &AvailabilityChallenge) -> Result<Vec<u8>> {
        let mut id_input = Vec::new();
        id_input.extend_from_slice(&challenge.chain_id);
        id_input.extend_from_slice(&challenge.chunk_index.to_be_bytes());
        id_input.extend_from_slice(&challenge.challenge_nonce);
        id_input.extend_from_slice(&challenge.challenger_id);

        Ok(compute_sha256(&id_input).to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_availability_challenger_creation() {
        let mut challenger = AvailabilityChallenger::new();
        let chain_id = Buffer::from([1u8; 32].to_vec());
        let challenger_id = Buffer::from([2u8; 32].to_vec());

        let challenge = challenger.create_challenge(chain_id, 1000, challenger_id, 100).unwrap();

        // Challenge creation is probabilistic, so it might be None
        if let Some(challenge) = challenge {
            assert_eq!(challenge.chain_id.len(), 32);
            assert!(challenge.chunk_index < 1000);
            assert_eq!(challenge.challenge_nonce.len(), 32);
            assert_eq!(challenge.reward_amount, AVAILABILITY_REWARD_UNITS as f64);
        }
    }

    #[test]
    fn test_challenge_deterministic() {
        let mut challenger1 = AvailabilityChallenger::new();
        let mut challenger2 = AvailabilityChallenger::new();
        
        let chain_id = Buffer::from([3u8; 32].to_vec());
        let challenger_id = Buffer::from([4u8; 32].to_vec());
        let block_height = 200;

        let challenge1 = challenger1.create_challenge(chain_id.clone(), 1000, challenger_id.clone(), block_height).unwrap();
        let challenge2 = challenger2.create_challenge(chain_id, 1000, challenger_id, block_height).unwrap();

        // Should be deterministic for same inputs
        match (challenge1, challenge2) {
            (Some(c1), Some(c2)) => {
                assert_eq!(c1.chunk_index, c2.chunk_index);
                assert_eq!(c1.challenge_nonce.as_ref(), c2.challenge_nonce.as_ref());
            }
            (None, None) => {
                // Both decided not to challenge - also deterministic
            }
            _ => {
                panic!("Challenges should be deterministic");
            }
        }
    }

    #[test]
    fn test_availability_prover() {
        let mut prover = AvailabilityProver::new();
        prover.register_chain("test_chain".to_string(), "/fake/path".to_string(), 1000);

        let challenge = AvailabilityChallenge {
            chain_id: Buffer::from(hex::decode("test_chain").unwrap_or_default()),
            chunk_index: 42,
            challenge_nonce: Buffer::from([5u8; 32].to_vec()),
            challenger_id: Buffer::from([6u8; 32].to_vec()),
            challenge_time: 1234567890.0,
            deadline: 1234567890.5,
            reward_amount: AVAILABILITY_REWARD_UNITS as f64,
        };

        // This will fail because /fake/path doesn't exist, but it tests the flow
        let result = prover.respond_to_challenge(&challenge);
        assert!(result.is_err()); // Expected to fail with fake path
    }

    #[test]
    fn test_challenge_stats() {
        let challenger = AvailabilityChallenger::new();
        let stats = challenger.get_challenge_stats();

        assert_eq!(stats.active_challenges, 0);
        assert_eq!(stats.challenge_probability, AVAILABILITY_CHALLENGE_PROBABILITY);
        assert_eq!(stats.response_timeout_ms, AVAILABILITY_RESPONSE_TIME_MS);
    }
} 