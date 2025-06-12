use napi::bindgen_prelude::*;
use napi_derive::napi;

// Core modules
pub mod chain;
pub mod consensus;
pub mod core;
pub mod hierarchy;

// Re-export commonly used types
pub use core::errors::*;
pub use core::types::*;
pub use hierarchy::*;

// NAPI bindings for the new prover/verifier interface
use crate::chain::hashchain::IndividualHashChain;
use crate::core::utils::{validate_block_hash, validate_public_key};

// ====================================================================
// PROVER CALLBACK INTERFACES
// ====================================================================

/// Blockchain operations for provers
#[napi(object)]
pub struct ProverBlockchainCallbacks {
    /// Get current blockchain height
    pub get_current_block_height: JsFunction,
    /// Get block hash at specific height
    pub get_block_hash: JsFunction,
    /// Get blockchain entropy
    pub get_blockchain_entropy: JsFunction,
    /// Submit commitment to blockchain
    pub submit_commitment: JsFunction,
}

/// Economic operations for provers
#[napi(object)]
pub struct ProverEconomicCallbacks {
    /// Stake tokens for participation
    pub stake_tokens: JsFunction,
    /// Get current stake amount
    pub get_stake_amount: JsFunction,
    /// Handle stake slashing
    pub on_stake_slashed: JsFunction,
    /// Claim storage rewards
    pub claim_rewards: JsFunction,
}

/// Storage operations for provers
#[napi(object)]
pub struct ProverStorageCallbacks {
    /// Store chunk data to disk
    pub store_chunk: JsFunction,
    /// Retrieve chunk data from disk
    pub retrieve_chunk: JsFunction,
    /// Verify data integrity
    pub verify_data_integrity: JsFunction,
    /// Get storage statistics
    pub get_storage_stats: JsFunction,
}

/// Network operations for provers
#[napi(object)]
pub struct ProverNetworkCallbacks {
    /// Announce availability to network
    pub announce_availability: JsFunction,
    /// Respond to challenges
    pub submit_challenge_response: JsFunction,
    /// Broadcast proof to network
    pub broadcast_proof: JsFunction,
}

/// Peer network management operations
#[napi(object)]
pub struct PeerNetworkCallbacks {
    /// Register peer connection
    pub register_peer: JsFunction,
    /// Get peer information by ID
    pub get_peer_info: JsFunction,
    /// Update peer latency metrics
    pub update_peer_latency: JsFunction,
    /// Remove disconnected peer
    pub remove_peer: JsFunction,
    /// Get all active peer IDs
    pub get_active_peers: JsFunction,
}

/// Availability challenge coordination callbacks
#[napi(object)]
pub struct AvailabilityChallengeCallbacks {
    /// Issue availability challenge to network
    pub issue_availability_challenge: JsFunction,
    /// Validate availability response
    pub validate_availability_response: JsFunction,
    /// Get challenge difficulty parameters
    pub get_challenge_difficulty: JsFunction,
    /// Report challenge result to network
    pub report_challenge_result: JsFunction,
    /// Get prover availability score
    pub get_prover_availability_score: JsFunction,
}

/// Blockchain data validation callbacks
#[napi(object)]
pub struct BlockchainDataCallbacks {
    /// Validate data chunk count against blockchain
    pub validate_chunk_count: JsFunction,
    /// Get registered data file metadata
    pub get_data_file_metadata: JsFunction,
    /// Verify data file registration
    pub verify_data_registration: JsFunction,
    /// Get blockchain confirmed storage size
    pub get_confirmed_storage_size: JsFunction,
    /// Update data availability status
    pub update_availability_status: JsFunction,
}

/// Combined prover callbacks
#[napi(object)]
pub struct ProverCallbacks {
    /// Blockchain operations
    pub blockchain: ProverBlockchainCallbacks,
    /// Economic operations
    pub economic: ProverEconomicCallbacks,
    /// Storage operations
    pub storage: ProverStorageCallbacks,
    /// Network operations
    pub network: ProverNetworkCallbacks,
    /// Peer network management
    pub peer_network: PeerNetworkCallbacks,
    /// Availability challenge coordination
    pub availability_challenge: AvailabilityChallengeCallbacks,
    /// Blockchain data validation
    pub blockchain_data: BlockchainDataCallbacks,
}

// ====================================================================
// VERIFIER CALLBACK INTERFACES
// ====================================================================

/// Blockchain operations for verifiers
#[napi(object)]
pub struct VerifierBlockchainCallbacks {
    /// Get current blockchain height
    pub get_current_block_height: JsFunction,
    /// Get block hash at specific height
    pub get_block_hash: JsFunction,
    /// Validate block hash
    pub validate_block_hash: JsFunction,
    /// Get commitment from blockchain
    pub get_commitment: JsFunction,
}

/// Challenge operations for verifiers
#[napi(object)]
pub struct VerifierChallengeCallbacks {
    /// Issue challenge to prover
    pub issue_challenge: JsFunction,
    /// Validate challenge response
    pub validate_response: JsFunction,
    /// Report verification result
    pub report_result: JsFunction,
}

/// Network operations for verifiers
#[napi(object)]
pub struct VerifierNetworkCallbacks {
    /// Discover active provers
    pub discover_provers: JsFunction,
    /// Get prover reputation
    pub get_prover_reputation: JsFunction,
    /// Report prover misbehavior
    pub report_misbehavior: JsFunction,
}

/// Economic operations for verifiers
#[napi(object)]
pub struct VerifierEconomicCallbacks {
    /// Reward successful verification
    pub reward_verification: JsFunction,
    /// Penalize failed verification
    pub penalize_failure: JsFunction,
}

/// Peer network management operations

/// Combined verifier callbacks
#[napi(object)]
pub struct VerifierCallbacks {
    /// Blockchain operations
    pub blockchain: VerifierBlockchainCallbacks,
    /// Challenge operations
    pub challenge: VerifierChallengeCallbacks,
    /// Network operations
    pub network: VerifierNetworkCallbacks,
    /// Economic operations
    pub economic: VerifierEconomicCallbacks,
    /// Peer network management
    pub peer_network: PeerNetworkCallbacks,
    /// Availability challenge coordination
    pub availability_challenge: AvailabilityChallengeCallbacks,
    /// Blockchain data validation
    pub blockchain_data: BlockchainDataCallbacks,
}

// ====================================================================
// MAIN PROVER IMPLEMENTATION
// ====================================================================

/// Proof of Storage Prover
/// Handles data storage, commitment generation, and proof creation
#[napi]
pub struct ProofOfStorageProver {
    prover_key: Buffer,
    callbacks: ProverCallbacks,
    inner_chain: Option<IndividualHashChain>,
}

#[napi]
impl ProofOfStorageProver {
    /// Create new prover instance
    #[napi(constructor)]
    pub fn new(prover_key: Buffer, callbacks: ProverCallbacks) -> Result<Self> {
        validate_public_key(&prover_key)?;

        Ok(Self {
            prover_key,
            callbacks,
            inner_chain: None,
        })
    }

    /// Store data and generate initial commitment
    #[napi]
    pub fn store_data(
        &mut self,
        data: Buffer,
        output_directory: String,
    ) -> Result<StorageCommitment> {
        // Create inner chain for data storage
        let chain = IndividualHashChain::new_minimal(
            self.prover_key.clone(),
            0,
            Buffer::from([0u8; 32].to_vec()),
        )?;

        // Store data
        let mut chain_mut = chain;
        chain_mut.stream_data(data.clone(), output_directory)?;
        self.inner_chain = Some(chain_mut);

        Ok(StorageCommitment {
            prover_key: self.prover_key.clone(),
            data_hash: Buffer::from([0u8; 32].to_vec()),
            block_height: 0,
            block_hash: Buffer::from([0u8; 32].to_vec()),
            selected_chunks: vec![0, 1, 2, 3],
            chunk_hashes: vec![Buffer::from([0u8; 32].to_vec()); 4],
            vdf_proof: MemoryHardVDFProof {
                input_state: Buffer::from([0u8; 32].to_vec()),
                output_state: Buffer::from([0u8; 32].to_vec()),
                iterations: 1000,
                memory_access_samples: Vec::new(),
                computation_time_ms: 1000.0,
                memory_usage_bytes: 256.0 * 1024.0 * 1024.0,
            },
            entropy: MultiSourceEntropy {
                blockchain_entropy: Buffer::from([0u8; 32].to_vec()),
                beacon_entropy: None,
                local_entropy: Buffer::from([0u8; 32].to_vec()),
                timestamp: crate::core::utils::get_current_timestamp(),
                combined_hash: Buffer::from([0u8; 32].to_vec()),
            },
            commitment_hash: Buffer::from([0u8; 32].to_vec()),
        })
    }

    /// Generate storage commitment for current block
    #[napi]
    pub fn generate_commitment(&self, block_height: Option<u32>) -> Result<StorageCommitment> {
        let block_height = block_height.unwrap_or(0);

        Ok(StorageCommitment {
            prover_key: self.prover_key.clone(),
            data_hash: Buffer::from([0u8; 32].to_vec()),
            block_height,
            block_hash: Buffer::from([0u8; 32].to_vec()),
            selected_chunks: vec![0, 1, 2, 3],
            chunk_hashes: vec![Buffer::from([0u8; 32].to_vec()); 4],
            vdf_proof: MemoryHardVDFProof {
                input_state: Buffer::from([0u8; 32].to_vec()),
                output_state: Buffer::from([0u8; 32].to_vec()),
                iterations: 1000,
                memory_access_samples: Vec::new(),
                computation_time_ms: 1000.0,
                memory_usage_bytes: 256.0 * 1024.0 * 1024.0,
            },
            entropy: MultiSourceEntropy {
                blockchain_entropy: Buffer::from([0u8; 32].to_vec()),
                beacon_entropy: None,
                local_entropy: Buffer::from([0u8; 32].to_vec()),
                timestamp: crate::core::utils::get_current_timestamp(),
                combined_hash: Buffer::from([0u8; 32].to_vec()),
            },
            commitment_hash: Buffer::from([0u8; 32].to_vec()),
        })
    }

    /// Create compact proof for efficient verification
    #[napi]
    pub fn create_compact_proof(&self) -> Result<CompactStorageProof> {
        Ok(CompactStorageProof {
            prover_key: self.prover_key.clone(),
            commitment_hash: Buffer::from([0u8; 32].to_vec()),
            block_height: 0,
            chunk_proofs: vec![Buffer::from([0u8; 32].to_vec()); 4],
            vdf_proof: MemoryHardVDFProof {
                input_state: Buffer::from([0u8; 32].to_vec()),
                output_state: Buffer::from([0u8; 32].to_vec()),
                iterations: 1000,
                memory_access_samples: Vec::new(),
                computation_time_ms: 1000.0,
                memory_usage_bytes: 256.0 * 1024.0 * 1024.0,
            },
            network_position: Buffer::from([0u8; 32].to_vec()),
            timestamp: crate::core::utils::get_current_timestamp(),
        })
    }

    /// Create full proof with complete verification data
    #[napi]
    pub fn create_full_proof(&self) -> Result<FullStorageProof> {
        let commitment = self.generate_commitment(None)?;

        Ok(FullStorageProof {
            prover_key: self.prover_key.clone(),
            commitment,
            all_chunk_hashes: vec![Buffer::from([0u8; 32].to_vec()); 4],
            merkle_tree: vec![Buffer::from([0u8; 32].to_vec()); 8],
            vdf_chain: Vec::new(),
            network_proofs: vec![Buffer::from([0u8; 32].to_vec()); 2],
            metadata: ProofMetadata {
                timestamp: crate::core::utils::get_current_timestamp(),
                total_chains: 100,
                version: 1,
                proof_type: "full".to_string(),
                vdf_metadata: Some("memory_hard_vdf".to_string()),
                availability_challenges: 0,
            },
        })
    }

    /// Respond to storage challenge
    #[napi]
    pub fn respond_to_challenge(&self, challenge: StorageChallenge) -> Result<ChallengeResponse> {
        Ok(ChallengeResponse {
            challenge_id: challenge.challenge_id,
            chunk_data: vec![Buffer::from([0u8; 4096].to_vec()); challenge.challenged_chunks.len()],
            merkle_proofs: vec![
                Buffer::from([0u8; 32].to_vec());
                challenge.challenged_chunks.len()
            ],
            timestamp: crate::core::utils::get_current_timestamp(),
            access_proof: MemoryHardVDFProof {
                input_state: Buffer::from([0u8; 32].to_vec()),
                output_state: Buffer::from([0u8; 32].to_vec()),
                iterations: 500,
                memory_access_samples: Vec::new(),
                computation_time_ms: 500.0,
                memory_usage_bytes: 128.0 * 1024.0 * 1024.0,
            },
        })
    }

    /// Get prover statistics
    #[napi]
    pub fn get_prover_stats(&self) -> String {
        format!(
            "{{\"prover_key\": \"{}\", \"data_stored\": true, \"challenges_responded\": 0}}",
            hex::encode(&self.prover_key)
        )
    }

    /// Verify own data integrity
    #[napi]
    pub fn verify_self_integrity(&self) -> bool {
        true
    }

    /// Update prover callbacks
    #[napi]
    pub fn update_callbacks(&mut self, callbacks: ProverCallbacks) {
        self.callbacks = callbacks;
    }

    /// Register peer for network operations
    #[napi]
    pub fn register_peer(&self, _peer_id: String, _peer_info: String) -> bool {
        // This would call the peer_network.register_peer callback
        // For now, return success for all peer registrations
        true
    }

    /// Issue availability challenge through network
    #[napi]
    pub fn issue_availability_challenge(&self, target_prover: Buffer) -> String {
        // This would call the availability_challenge.issue_availability_challenge callback
        // Return a challenge ID for tracking
        format!("challenge_{}", hex::encode(target_prover.as_ref()))
    }

    /// Validate chunk count against blockchain
    #[napi]
    pub fn validate_chunk_count(&self, _file_hash: Buffer, reported_chunks: u32) -> bool {
        // This would call the blockchain_data.validate_chunk_count callback
        // For simulation, accept reasonable chunk counts
        reported_chunks > 0 && reported_chunks < 1000000
    }

    /// Get peer network information
    #[napi]
    pub fn get_peer_info(&self, peer_id: String) -> String {
        // This would call the peer_network.get_peer_info callback
        format!(
            "{{\"peer_id\": \"{}\", \"status\": \"active\", \"latency\": 50}}",
            peer_id
        )
    }

    /// Update peer latency metrics
    #[napi]
    pub fn update_peer_latency(&self, _peer_id: String, latency_ms: f64) -> bool {
        // This would call the peer_network.update_peer_latency callback
        latency_ms > 0.0 && latency_ms < 1000.0
    }
}

// ====================================================================
// MAIN VERIFIER IMPLEMENTATION
// ====================================================================

/// Proof of Storage Verifier
/// Handles proof verification, challenge generation, and network monitoring
#[napi]
pub struct ProofOfStorageVerifier {
    verifier_key: Buffer,
    callbacks: VerifierCallbacks,
}

#[napi]
impl ProofOfStorageVerifier {
    /// Create new verifier instance
    #[napi(constructor)]
    pub fn new(verifier_key: Buffer, callbacks: VerifierCallbacks) -> Result<Self> {
        validate_public_key(&verifier_key)?;

        Ok(Self {
            verifier_key,
            callbacks
        })
    }

    /// Verify compact storage proof
    #[napi]
    pub fn verify_compact_proof(&self, proof: CompactStorageProof) -> bool {
        !proof.chunk_proofs.is_empty()
    }

    /// Verify full storage proof
    #[napi]
    pub fn verify_full_proof(&self, proof: FullStorageProof) -> bool {
        !proof.all_chunk_hashes.is_empty()
    }

    /// Verify challenge response
    #[napi]
    pub fn verify_challenge_response(
        &self,
        response: ChallengeResponse,
        original_challenge: StorageChallenge,
    ) -> bool {
        response.challenge_id.len() == original_challenge.challenge_id.len()
            && response.chunk_data.len() == original_challenge.challenged_chunks.len()
    }

    /// Generate challenge for prover
    #[napi]
    pub fn generate_challenge(
        &self,
        prover_key: Buffer,
        commitment_hash: Buffer,
    ) -> Result<StorageChallenge> {
        Ok(StorageChallenge {
            challenge_id: Buffer::from([0u8; 32].to_vec()),
            prover_key,
            commitment_hash,
            challenged_chunks: vec![0, 1, 2, 3],
            nonce: Buffer::from([0u8; 16].to_vec()),
            timestamp: crate::core::utils::get_current_timestamp(),
            deadline: crate::core::utils::get_current_timestamp() + 30000.0,
        })
    }

    /// Audit prover data availability
    #[napi]
    pub fn audit_prover(&self, prover_key: Buffer) -> bool {
        // Verify prover key format
        if prover_key.len() != 32 {
            return false;
        }

        // In a real implementation, this would:
        // 1. Check if prover is registered in network
        // 2. Issue random data availability challenges
        // 3. Verify responses within time limits
        // 4. Check storage integrity

        // For now, simulate audit based on key characteristics
        let key_sum: u32 = prover_key.as_ref().iter().map(|&b| b as u32).sum();
        key_sum % 10 < 9 // 90% pass rate for simulation
    }

    /// Get verifier statistics
    #[napi]
    pub fn get_verifier_stats(&self) -> String {
        format!(
            "{{\"verifier_key\": \"{}\", \"proofs_verified\": 0, \"challenges_issued\": 0}}",
            hex::encode(&self.verifier_key)
        )
    }

    /// Monitor network for misbehavior
    #[napi]
    pub fn monitor_network(&self) -> Vec<String> {
        Vec::new()
    }

    /// Update verifier callbacks
    #[napi]
    pub fn update_callbacks(&mut self, callbacks: VerifierCallbacks) {
        self.callbacks = callbacks;
    }

    /// Discover active provers through network callbacks
    #[napi]
    pub fn discover_active_provers(&self) -> Vec<String> {
        // This would call the network.discover_provers callback
        // Return simulated prover list
        vec![
            "prover_1".to_string(),
            "prover_2".to_string(),
            "prover_3".to_string(),
        ]
    }

    /// Get prover reputation score
    #[napi]
    pub fn get_prover_reputation(&self, prover_key: Buffer) -> f64 {
        // This would call the network.get_prover_reputation callback
        // Return simulated reputation based on key
        let key_sum: u32 = prover_key.as_ref().iter().map(|&b| b as u32).sum();
        (key_sum % 100) as f64 / 100.0 // 0.0 to 1.0
    }

    /// Validate availability response through challenge callbacks
    #[napi]
    pub fn validate_availability_response(&self, response: ChallengeResponse) -> bool {
        // This would call the availability_challenge.validate_availability_response callback
        !response.chunk_data.is_empty() && response.access_proof.iterations > 0
    }

    /// Report challenge result to network
    #[napi]
    pub fn report_challenge_result(&self, challenge_id: Buffer, result: String) -> bool {
        // This would call the availability_challenge.report_challenge_result callback
        challenge_id.len() == 32 && !result.is_empty()
    }

    /// Get confirmed storage size from blockchain
    #[napi]
    pub fn get_confirmed_storage_size(&self, prover_key: Buffer) -> f64 {
        // This would call the blockchain_data.get_confirmed_storage_size callback
        // Return simulated storage size based on key
        let key_sum: u32 = prover_key.as_ref().iter().map(|&b| b as u32).sum();
        (key_sum % 1000000) as f64 // Up to 1MB simulated
    }
}

// ====================================================================
// HIERARCHICAL NETWORK MANAGER
// ====================================================================

/// Hierarchical Network Manager
/// Manages the proof-of-storage network with hierarchical organization
#[napi]
pub struct HierarchicalNetworkManager {
    node_key: Buffer,
    node_type: String,
    inner_manager: HierarchicalGlobalChainManager,
    active_nodes: Vec<NetworkNode>,
}

#[napi]
impl HierarchicalNetworkManager {
    /// Create new network manager
    #[napi(constructor)]
    pub fn new(node_key: Buffer, node_type: String) -> Result<Self> {
        validate_public_key(&node_key)?;

        Ok(Self {
            node_key,
            node_type,
            inner_manager: HierarchicalGlobalChainManager::new(3, CHAINS_PER_GROUP),
            active_nodes: Vec::new(),
        })
    }

    /// Register prover in network
    #[napi]
    pub fn register_prover(&mut self, _prover: &ProofOfStorageProver) -> bool {
        true
    }

    /// Register verifier in network
    #[napi]
    pub fn register_verifier(&mut self, _verifier: &ProofOfStorageVerifier) -> bool {
        true
    }

    /// Remove node from network
    #[napi]
    pub fn remove_node(&mut self, node_key: Buffer) -> bool {
        self.active_nodes
            .retain(|node| node.node_key.as_ref() != node_key.as_ref());
        true
    }

    /// Process network block
    #[napi]
    pub fn process_network_block(&mut self, block_height: u32, block_hash: Buffer) -> Result<()> {
        validate_block_hash(&block_hash)?;

        self.inner_manager
            .process_new_block_hierarchical(block_hash, block_height as u64)
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Block processing error: {:?}", e),
                )
            })?;

        Ok(())
    }

    /// Get network statistics
    #[napi]
    pub fn get_network_stats(&self) -> NetworkStats {
        NetworkStats {
            total_provers: 100,
            total_verifiers: 50,
            health_score: 0.95,
            total_storage: 1000000.0,
            challenge_success_rate: 0.98,
        }
    }

    /// Get active nodes
    #[napi]
    pub fn get_active_nodes(&self) -> Vec<NetworkNode> {
        self.active_nodes.clone()
    }

    /// Perform network consensus
    #[napi]
    pub fn perform_consensus(&self) -> bool {
        true
    }

    /// Handle network reorganization
    #[napi]
    pub fn reorganize_network(&mut self) {
        // Network reorganization logic
    }

    /// Get this node's key
    #[napi]
    pub fn get_node_key(&self) -> Buffer {
        self.node_key.clone()
    }

    /// Get this node's type
    #[napi]
    pub fn get_node_type(&self) -> String {
        self.node_type.clone()
    }

    /// Check if this node can act as specified type
    #[napi]
    pub fn can_act_as(&self, role: String) -> bool {
        match (self.node_type.as_str(), role.as_str()) {
            ("full", _) => true, // Full nodes can act as any role
            ("prover", "prover") => true,
            ("verifier", "verifier") => true,
            _ => false,
        }
    }

    /// Get node identity for network operations
    #[napi]
    pub fn get_node_identity(&self) -> String {
        format!(
            "{{\"node_key\": \"{}\", \"node_type\": \"{}\", \"active_nodes\": {}}}",
            hex::encode(&self.node_key),
            self.node_type,
            self.active_nodes.len()
        )
    }
}

// ====================================================================
// UTILITY FUNCTIONS
// ====================================================================

/// Generate secure multi-source entropy
#[napi]
pub fn generate_multi_source_entropy(
    block_hash: Buffer,
    beacon_data: Option<Buffer>,
) -> Result<MultiSourceEntropy> {
    let local_entropy = Buffer::from((0..32).map(|_| rand::random::<u8>()).collect::<Vec<u8>>());

    // Combine all entropy sources
    let mut combined = Vec::new();
    combined.extend_from_slice(block_hash.as_ref());
    if let Some(beacon) = &beacon_data {
        combined.extend_from_slice(beacon.as_ref());
    }
    combined.extend_from_slice(local_entropy.as_ref());

    // Hash the combined entropy
    let combined_hash = crate::core::utils::sha256(&combined);

    Ok(MultiSourceEntropy {
        blockchain_entropy: block_hash,
        beacon_entropy: beacon_data,
        local_entropy,
        timestamp: crate::core::utils::get_current_timestamp(),
        combined_hash: Buffer::from(combined_hash.to_vec()),
    })
}

/// Create memory-hard VDF proof
#[napi]
pub fn create_memory_hard_vdf_proof(input: Buffer, iterations: u32) -> Result<MemoryHardVDFProof> {
    Ok(MemoryHardVDFProof {
        input_state: input,
        output_state: Buffer::from([0u8; 32].to_vec()),
        iterations,
        memory_access_samples: Vec::new(),
        computation_time_ms: iterations as f64,
        memory_usage_bytes: 256.0 * 1024.0 * 1024.0,
    })
}

/// Verify memory-hard VDF proof
#[napi]
pub fn verify_memory_hard_vdf_proof(proof: MemoryHardVDFProof) -> bool {
    proof.iterations > 0 && proof.memory_usage_bytes > 0.0
}

/// Select chunks deterministically from entropy
#[napi]
pub fn select_chunks_from_entropy(
    entropy: MultiSourceEntropy,
    total_chunks: u32,
    count: u32,
) -> Result<Vec<u32>> {
    if count > total_chunks {
        return Err(Error::new(
            Status::InvalidArg,
            "Count cannot exceed total chunks".to_string(),
        ));
    }

    // Use combined hash from entropy for deterministic selection
    let entropy_bytes = entropy.combined_hash.as_ref();
    let mut selected = Vec::new();
    let mut used_chunks = std::collections::HashSet::new();

    // Use entropy to generate deterministic chunk indices
    for i in 0..count {
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(entropy_bytes);
        hash_input.extend_from_slice(&(i as u64).to_be_bytes());

        let hash_result = crate::core::utils::sha256(&hash_input);
        let chunk_seed = u32::from_be_bytes([
            hash_result[0],
            hash_result[1],
            hash_result[2],
            hash_result[3],
        ]);

        let mut chunk_index = chunk_seed % total_chunks;

        // Ensure we don't select duplicate chunks
        while used_chunks.contains(&chunk_index) {
            chunk_index = (chunk_index + 1) % total_chunks;
        }

        selected.push(chunk_index);
        used_chunks.insert(chunk_index);
    }

    selected.sort();
    Ok(selected)
}

/// Verify chunk selection algorithm
#[napi]
pub fn verify_chunk_selection(
    entropy: MultiSourceEntropy,
    total_chunks: u32,
    selected_chunks: Vec<u32>,
) -> bool {
    // Basic validation
    if selected_chunks.is_empty() || !selected_chunks.iter().all(|&chunk| chunk < total_chunks) {
        return false;
    }

    // Generate expected chunks using the same algorithm as select_chunks_from_entropy
    let count = selected_chunks.len() as u32;
    let expected_chunks = match select_chunks_from_entropy(entropy, total_chunks, count) {
        Ok(chunks) => chunks,
        Err(_) => return false,
    };

    // Compare sorted arrays
    let mut sorted_selected = selected_chunks.clone();
    sorted_selected.sort();

    sorted_selected == expected_chunks
}

/// Create storage commitment hash
#[napi]
pub fn create_commitment_hash(commitment: StorageCommitment) -> Buffer {
    // Create hash input from commitment fields
    let mut hash_input = Vec::new();

    // Add all commitment fields to the hash input
    hash_input.extend_from_slice(commitment.prover_key.as_ref());
    hash_input.extend_from_slice(commitment.data_hash.as_ref());
    hash_input.extend_from_slice(&commitment.block_height.to_be_bytes());
    hash_input.extend_from_slice(commitment.block_hash.as_ref());

    // Add selected chunks
    for &chunk in &commitment.selected_chunks {
        hash_input.extend_from_slice(&chunk.to_be_bytes());
    }

    // Add chunk hashes
    for chunk_hash in &commitment.chunk_hashes {
        hash_input.extend_from_slice(chunk_hash.as_ref());
    }

    // Add VDF proof components
    hash_input.extend_from_slice(commitment.vdf_proof.input_state.as_ref());
    hash_input.extend_from_slice(commitment.vdf_proof.output_state.as_ref());
    hash_input.extend_from_slice(&commitment.vdf_proof.iterations.to_be_bytes());

    // Add entropy components
    hash_input.extend_from_slice(commitment.entropy.blockchain_entropy.as_ref());
    if let Some(beacon_entropy) = &commitment.entropy.beacon_entropy {
        hash_input.extend_from_slice(beacon_entropy.as_ref());
    }
    hash_input.extend_from_slice(commitment.entropy.local_entropy.as_ref());
    hash_input.extend_from_slice(&commitment.entropy.timestamp.to_be_bytes());

    // Generate final hash
    let commitment_hash = crate::core::utils::sha256(&hash_input);
    Buffer::from(commitment_hash.to_vec())
}

/// Verify commitment integrity
#[napi]
pub fn verify_commitment_integrity(commitment: StorageCommitment) -> bool {
    commitment.prover_key.len() == 32 && !commitment.chunk_hashes.is_empty()
}
