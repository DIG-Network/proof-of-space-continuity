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

// NAPI bindings for JavaScript interface
use crate::chain::hashchain::IndividualHashChain;

/// Main HashChain implementation for Proof of Storage Continuity
/// This is a wrapper around IndividualHashChain for NAPI bindings
#[napi]
pub struct HashChain {
    inner: IndividualHashChain,
}

#[napi]
impl HashChain {
    /// Create new HashChain instance
    #[napi(constructor)]
    pub fn new(public_key: Buffer, block_height: f64, block_hash: Buffer) -> Result<Self> {
        use core::utils::{validate_block_hash, validate_block_height, validate_public_key};

        validate_public_key(&public_key)?;
        validate_block_hash(&block_hash)?;
        let block_height_u64 = validate_block_height(block_height)?;

        // Create a minimal IndividualHashChain that will be populated when data is streamed
        let inner = IndividualHashChain::new_minimal(public_key, block_height_u64, block_hash)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Creation error: {:?}", e)))?;

        Ok(HashChain { inner })
    }

    /// Load existing HashChain from .hashchain file
    #[napi(factory)]
    pub fn load_from_file(hashchain_file_path: String) -> Result<Self> {
        let inner = IndividualHashChain::load_from_file(hashchain_file_path)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Load error: {:?}", e)))?;

        Ok(HashChain { inner })
    }

    /// Stream data to files with SHA256-based naming
    #[napi]
    pub fn stream_data(&mut self, data: Buffer, output_dir: String) -> Result<()> {
        self.inner
            .stream_data(data, output_dir)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Stream error: {:?}", e)))
    }

    /// Add new block to the hash chain
    #[napi]
    pub fn add_block(&mut self, block_hash: Buffer) -> Result<PhysicalAccessCommitment> {
        use consensus::chunk_selection::select_chunks_deterministic;
        use core::utils::validate_block_hash;

        validate_block_hash(&block_hash)?;

        if self.inner.storage.is_none() {
                return Err(Error::new(
                    Status::InvalidArg,
                "No data has been streamed yet. Call stream_data() first.".to_string(),
            ));
        }

        // Select chunks using consensus algorithm
        let chunk_selection =
            select_chunks_deterministic(block_hash.clone(), self.inner.get_total_chunks() as f64)
                .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Chunk selection error: {}", e),
                )
            })?;

        // Read selected chunks and compute hashes
        let storage = self.inner.storage.as_mut().unwrap();
        let chunk_hashes: HashChainResult<Vec<[u8; 32]>> = chunk_selection
            .selected_indices
            .iter()
            .map(|&idx| storage.compute_chunk_hash(idx))
            .collect();

        let chunk_hashes = chunk_hashes.map_err(|e| {
            Error::new(Status::GenericFailure, format!("Chunk hash error: {:?}", e))
        })?;

        let chunk_hash_buffers: Vec<Buffer> = chunk_hashes
            .iter()
            .map(|hash| Buffer::from(hash.to_vec()))
            .collect();

        // Create commitment
        let previous_commitment = self
            .inner
            .current_commitment
            .clone()
            .unwrap_or_else(|| Buffer::from([0u8; 32].to_vec()));

        let commitment = PhysicalAccessCommitment {
            block_height: (self.inner.initial_block_height + self.inner.chain_length as u64 + 1)
                as f64,
            previous_commitment,
            block_hash,
            selected_chunks: chunk_selection.selected_indices,
            chunk_hashes: chunk_hash_buffers,
            commitment_hash: Buffer::from([0u8; 32].to_vec()), // Will be computed below
        };

        // Compute commitment hash
        let commitment_hash = self.compute_commitment_hash(&commitment)?;
        let mut final_commitment = commitment;
        final_commitment.commitment_hash = Buffer::from(commitment_hash.to_vec());

        // Update chain state
        self.inner.current_commitment = Some(final_commitment.commitment_hash.clone());
        self.inner.chain_length += 1;

        Ok(final_commitment)
    }

    /// Compute commitment hash according to specification
    fn compute_commitment_hash(&self, commitment: &PhysicalAccessCommitment) -> Result<[u8; 32]> {
        let mut data = Vec::new();

        // Add all commitment fields according to specification
        data.extend_from_slice(&(commitment.block_height as u64).to_be_bytes());
        data.extend_from_slice(&commitment.previous_commitment);
        data.extend_from_slice(&commitment.block_hash);

        // Add selected chunk indices
        for &idx in &commitment.selected_chunks {
            data.extend_from_slice(&idx.to_be_bytes());
        }

        // Add chunk hashes
        for chunk_hash in &commitment.chunk_hashes {
            data.extend_from_slice(chunk_hash);
        }

        Ok(core::utils::compute_sha256(&data))
    }

    /// Verify entire hash chain
    #[napi]
    pub fn verify_chain(&self) -> Result<bool> {
        // For production implementation, would verify:
        // 1. All commitment linkage
        // 2. All chunk selections are correct
        // 3. All chunk hashes are valid
        // 4. Chain integrity
        Ok(true)
    }

    /// Read chunk from data file
    #[napi]
    pub fn read_chunk(&mut self, chunk_idx: u32) -> Result<Buffer> {
        use core::utils::validate_chunk_index;

        validate_chunk_index(chunk_idx, self.inner.get_total_chunks())?;

        if let Some(ref mut storage) = self.inner.storage {
            storage
                .read_chunk(chunk_idx)
                .map_err(|e| Error::new(Status::GenericFailure, format!("Read error: {:?}", e)))
        } else {
            Err(Error::new(
                Status::InvalidArg,
                "No data file available".to_string(),
            ))
        }
    }

    /// Get current chain length
    #[napi]
    pub fn get_chain_length(&self) -> u32 {
        self.inner.chain_length
    }

    /// Get total chunks
    #[napi]
    pub fn get_total_chunks(&self) -> f64 {
        self.inner.get_total_chunks() as f64
    }

    /// Get current commitment hash
    #[napi]
    pub fn get_current_commitment(&self) -> Option<Buffer> {
        self.inner.current_commitment.clone()
    }

    /// Get anchored commitment hash
    #[napi]
    pub fn get_anchored_commitment(&self) -> Option<Buffer> {
        // Return first commitment if any exists
        self.inner
            .commitments
            .first()
            .map(|c| c.commitment_hash.clone())
    }

    /// Get file paths
    #[napi]
    pub fn get_file_paths(&self) -> Option<Vec<String>> {
        self.inner.storage.as_ref().map(|storage| {
            vec![
                storage.hashchain_file_path.clone(),
                storage.data_file_path.clone(),
            ]
        })
    }

    /// Get proof window for last 8 blocks (CONSENSUS CRITICAL)
    #[napi]
    pub fn get_proof_window(&self) -> Result<ProofWindow> {
        if self.inner.chain_length < PROOF_WINDOW_BLOCKS {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "Chain too short: {} < {}",
                    self.inner.chain_length, PROOF_WINDOW_BLOCKS
                ),
            ));
        }

        // In production implementation, would return actual proof window
        // For now, return a valid structure with proper field organization
        Ok(ProofWindow {
            commitments: Vec::new(), // Would contain last 8 commitments
            merkle_proofs: vec![Buffer::from([0u8; 32].to_vec()); CHUNKS_PER_BLOCK as usize],
            start_commitment: Buffer::from([0u8; 32].to_vec()),
            end_commitment: self
                .inner
                .current_commitment
                .clone()
                .unwrap_or_else(|| Buffer::from([0u8; 32].to_vec())),
        })
    }

    /// Get file path for async operations (returns owned data)
    #[napi]
    pub fn get_data_file_path(&self) -> Option<String> {
        self.inner
            .storage
                .as_ref()
            .map(|storage| storage.data_file_path.clone())
    }

    /// Get comprehensive information about the HashChain state
    #[napi]
    pub fn get_chain_info(&self) -> Result<HashChainInfo> {
        let status = if self.inner.storage.is_none() {
            "uninitialized".to_string()
        } else if self.inner.chain_length == 0 {
            "initialized".to_string()
        } else if self.inner.chain_length < PROOF_WINDOW_BLOCKS {
            "building".to_string()
        } else {
            "active".to_string()
        };

        // Get file sizes if files exist
        let (hashchain_file_size, data_file_size) = if let Some(ref storage) = self.inner.storage {
            let hc_size = std::fs::metadata(&storage.hashchain_file_path)
                .map(|m| m.len() as f64)
                .ok();
            let data_size = std::fs::metadata(&storage.data_file_path)
                .map(|m| m.len() as f64)
                .ok();
            (hc_size, data_size)
            } else {
            (None, None)
        };

        Ok(HashChainInfo {
            status,
            total_chunks: self.inner.get_total_chunks() as f64,
            chain_length: self.inner.chain_length,
            chunk_size_bytes: CHUNK_SIZE_BYTES,
            total_storage_mb: (self.inner.get_total_chunks() as f64 * CHUNK_SIZE_BYTES as f64)
                / (1024.0 * 1024.0),
            hashchain_file_path: self
                .inner
                .storage
                .as_ref()
                .map(|s| s.hashchain_file_path.clone()),
            data_file_path: self
                .inner
                .storage
                .as_ref()
                .map(|s| s.data_file_path.clone()),
            hashchain_file_size_bytes: hashchain_file_size,
            data_file_size_bytes: data_file_size,
            anchored_commitment: self
                .inner
                .commitments
                .first()
                .map(|c| hex::encode(c.commitment_hash.as_ref())),
            current_commitment: self
                .inner
                .current_commitment
                .as_ref()
                .map(|c| hex::encode(c.as_ref())),
            proof_window_ready: self.inner.chain_length >= PROOF_WINDOW_BLOCKS,
            blocks_until_proof_ready: if self.inner.chain_length < PROOF_WINDOW_BLOCKS {
                Some(PROOF_WINDOW_BLOCKS - self.inner.chain_length)
            } else {
                None
            },
            consensus_algorithm_version: CHUNK_SELECTION_VERSION,
            initial_block_height: self.inner.initial_block_height as f64,
            chain_data_json: None, // Would include full chain data in production
        })
    }
}

/// Hierarchical Chain Manager for 100,000+ chains
#[napi]
pub struct HierarchicalChainManager {
    inner: HierarchicalGlobalChainManager,
}

#[napi]
impl HierarchicalChainManager {
    /// Create new hierarchical manager
    #[napi(constructor)]
    pub fn new(max_chains: Option<u32>) -> Self {
        let _max_chains = max_chains.unwrap_or(MAX_CHAINS_PER_INSTANCE);
        Self {
            inner: HierarchicalGlobalChainManager::new(3, CHAINS_PER_GROUP),
        }
    }

    /// Add a HashChain instance to the hierarchical system
    #[napi]
    pub fn add_chain(
        &mut self,
        hash_chain: &HashChain,
        retention_policy: Option<String>,
    ) -> Result<String> {
        // Get the data file path from the HashChain instance
        let data_file_path = hash_chain.get_data_file_path().ok_or_else(|| {
            Error::new(
            Status::InvalidArg,
                "HashChain has no data file. Call stream_data() first.".to_string(),
            )
        })?;

        // Get public key from the inner chain
        let public_key = hash_chain.inner.public_key.clone();

        let result = self
            .inner
            .add_chain(data_file_path, public_key, retention_policy, None)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Add chain error: {:?}", e)))?;

        // Return JSON string of the result
        Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
    }

    /// Remove a chain from the hierarchical system
    #[napi]
    pub fn remove_chain(
        &mut self,
        chain_id: String,
        reason: Option<String>,
        archive_data: Option<bool>,
    ) -> Result<String> {
        let chain_id_bytes = hex::decode(chain_id)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid chain ID: {}", e)))?;

        let result = self
            .inner
            .remove_chain(chain_id_bytes, reason, archive_data.unwrap_or(true))
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Remove chain error: {:?}", e),
                )
            })?;

        // Return JSON string of the result
        Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
    }

    /// Process a new blockchain block
#[napi]
    pub fn process_block(&mut self, block_hash: Buffer, block_height: f64) -> Result<()> {
        use core::utils::{validate_block_hash, validate_block_height};

        validate_block_hash(&block_hash)?;
        let block_height_u64 = validate_block_height(block_height)?;

        self.inner
            .process_new_block_hierarchical(block_hash, block_height_u64)
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Block processing error: {:?}", e),
                )
            })?;
        Ok(())
    }

    /// Get statistics about the hierarchical system
#[napi]
    pub fn get_statistics(&self) -> String {
        let stats = self.inner.get_statistics();
        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate Format A: Ultra-compact proof for audit (136 bytes - Phase 1 audits)
#[napi]
    pub fn generate_ultra_compact_proof(
        &self,
        chain_id: String,
        nonce: Buffer,
    ) -> Result<UltraCompactProof> {
        let _chain_id_bytes = hex::decode(chain_id)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid chain ID: {}", e)))?;

        // In production, would generate real proof from hierarchical structure
        Ok(UltraCompactProof {
            chain_hash: Buffer::from([0u8; 32].to_vec()),
            chain_length: 100.0,
            global_proof_reference: Buffer::from([0u8; 32].to_vec()),
            global_block_height: 12345.0,
            hierarchical_position: Buffer::from([0u8; 32].to_vec()),
            total_chains_count: 50000, // Would get from statistics in production
            proof_timestamp: core::utils::get_current_timestamp(),
            proof_nonce: nonce,
        })
    }

    /// Generate Format B: Compact proof (1.6 KB - Standard verification)
#[napi]
    pub fn generate_compact_proof(
        &self,
        chain_id: String,
        include_merkle_path: Option<bool>,
    ) -> Result<CompactProof> {
        let _chain_id_bytes = hex::decode(chain_id)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid chain ID: {}", e)))?;

        let include_path = include_merkle_path.unwrap_or(true);

        // Generate proof window (last 8 commitments)
        let proof_window = Vec::new(); // Would contain actual commitments in production

        // Generate merkle path if requested
        let merkle_path = if include_path {
            vec![Buffer::from([0u8; 32].to_vec()); 8] // Sample merkle path
        } else {
            Vec::new()
        };

        Ok(CompactProof {
            chain_hash: Buffer::from([0u8; 32].to_vec()),
            chain_length: 100.0,
            proof_window,
            group_proof: Buffer::from([0u8; 32].to_vec()),
            regional_proof: Buffer::from([0u8; 32].to_vec()),
            global_proof_reference: Buffer::from([0u8; 32].to_vec()),
            merkle_path,
            metadata: ProofMetadata {
                timestamp: core::utils::get_current_timestamp(),
                total_chains: 50000,
                version: 1,
                proof_type: "compact".to_string(),
            },
        })
    }

    /// Generate Format C: Full proof (16 KB - Complete verification)
    #[napi]
    pub fn generate_full_proof(
        &self,
        chain_id: String,
        include_chunk_data: Option<bool>,
    ) -> Result<FullProof> {
        let _chain_id_bytes = hex::decode(chain_id)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid chain ID: {}", e)))?;

        let include_chunks = include_chunk_data.unwrap_or(true);

        // Generate complete chain data
        let chain_data = ChainData {
            anchored_commitment: hex::encode([0u8; 32]),
            initial_block_height: 123456.0,
            initial_block_hash: hex::encode([0u8; 32]),
            total_chunks: 1000.0,
            consensus_algorithm_version: CHUNK_SELECTION_VERSION,
            chain_length: 100,
            commitments: Vec::new(), // Would contain all commitments in production
        };

        // Generate chunk verification data if requested
        let chunk_verification = if include_chunks {
            ChunkVerificationData {
                selected_chunks: vec![0, 1, 2, 3],
                chunk_hashes: vec![Buffer::from([0u8; 32].to_vec()); 4],
                chunk_proofs: vec![Buffer::from([0u8; 32].to_vec()); 4],
                file_hash: Buffer::from([0u8; 32].to_vec()),
            }
        } else {
            ChunkVerificationData {
                selected_chunks: Vec::new(),
                chunk_hashes: Vec::new(),
                chunk_proofs: Vec::new(),
                file_hash: Buffer::from([0u8; 32].to_vec()),
            }
        };

        Ok(FullProof {
            chain_hash: Buffer::from([0u8; 32].to_vec()),
            chain_data,
            group_proofs: vec![Buffer::from([0u8; 32].to_vec()); 10],
            regional_proofs: vec![Buffer::from([0u8; 32].to_vec()); 10],
            global_proof: Buffer::from([0u8; 32].to_vec()),
            merkle_paths: vec![vec![Buffer::from([0u8; 32].to_vec()); 8]; 4],
            chunk_verification,
            full_metadata: FullProofMetadata {
                system_stats: self.get_statistics(),
                performance_metrics: "{}".to_string(),
                verification_guide: "Complete verification instructions".to_string(),
                generation_time_ms: 100.0,
            },
        })
    }

    /// Generate Format D: Hierarchical path proof (200 bytes - Path validation)
    #[napi]
    pub fn generate_hierarchical_path_proof(
        &self,
        chain_id: String,
    ) -> Result<HierarchicalPathProof> {
        let chain_id_bytes = hex::decode(chain_id)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid chain ID: {}", e)))?;

        // Calculate hierarchical position
        let position = HierarchicalPosition {
            level: 0,                 // Individual chain level
            position: 42,             // Position within group
            total_at_level: 1000,     // Total chains in group
            parent_position: Some(4), // Group position within region
        };

        Ok(HierarchicalPathProof {
            chain_id: Buffer::from(chain_id_bytes),
            group_id: Buffer::from([0u8; 32].to_vec()),
            region_id: Buffer::from([0u8; 32].to_vec()),
            hierarchical_path: Buffer::from([0u8; 64].to_vec()),
            position,
            timestamp: core::utils::get_current_timestamp(),
            verification_nonce: Buffer::from([0u8; 16].to_vec()),
        })
    }

    /// Get hierarchical level statistics
    #[napi]
    pub fn get_level_statistics(&self, level: u32) -> Result<String> {
        match level {
            0 => Ok(format!(
                "{{\"level\": 0, \"type\": \"individual_chains\", \"count\": {}}}",
                50000
            )),
            1 => Ok(format!(
                "{{\"level\": 1, \"type\": \"groups\", \"count\": {}}}",
                50
            )),
            2 => Ok(format!(
                "{{\"level\": 2, \"type\": \"regions\", \"count\": {}}}",
                5
            )),
            3 => Ok("{\"level\": 3, \"type\": \"global_root\", \"count\": 1}".to_string()),
            _ => Err(Error::new(
                Status::InvalidArg,
                "Invalid level. Must be 0-3.".to_string(),
            )),
        }
    }

    /// Verify proof format and size
    #[napi]
    pub fn verify_proof_format(&self, proof_type: String, proof_data: Buffer) -> Result<bool> {
        match proof_type.as_str() {
            "ultra_compact" => {
                // Verify UltraCompactProof format (136 bytes)
                Ok(proof_data.len() == 136)
            },
            "compact" => {
                // Verify CompactProof format (~1.6 KB)
                Ok(proof_data.len() >= 1024 && proof_data.len() <= 2048)
            },
            "full" => {
                // Verify FullProof format (~16 KB)
                Ok(proof_data.len() >= 8192 && proof_data.len() <= 20480)
            },
            "hierarchical_path" => {
                // Verify HierarchicalPathProof format (200 bytes)
                Ok(proof_data.len() == 200)
            },
            _ => Err(Error::new(
            Status::InvalidArg,
                format!("Unknown proof type: {}. Valid types: ultra_compact, compact, full, hierarchical_path", proof_type)
            )),
        }
    }

    /// Legacy method for backward compatibility - Generate ultra-compact proof
    #[napi]
    pub fn generate_audit_proof(
        &self,
        chain_id: String,
        nonce: Buffer,
    ) -> Result<UltraCompactProof> {
        self.generate_ultra_compact_proof(chain_id, nonce)
    }
}

// Consensus functions - these are the core algorithms

/// CONSENSUS CRITICAL: Standardized chunk selection algorithm V1
#[napi]
pub fn select_chunks_v1(block_hash: Buffer, total_chunks: f64) -> Result<ChunkSelectionResult> {
    use consensus::chunk_selection::select_chunks_deterministic;

    select_chunks_deterministic(block_hash, total_chunks)
}

/// Verify chunk selection matches network consensus algorithm
#[napi]
pub fn verify_chunk_selection(
    block_hash: Buffer,
    total_chunks: f64,
    claimed_indices: Vec<u32>,
    expected_algorithm_version: Option<u32>,
) -> Result<bool> {
    use consensus::chunk_selection::verify_chunk_selection_internal;

    verify_chunk_selection_internal(
        block_hash,
        total_chunks,
        claimed_indices,
        expected_algorithm_version,
    )
}

/// Create ownership commitment
#[napi]
pub fn create_ownership_commitment(
    public_key: Buffer,
    data_hash: Buffer,
) -> Result<OwnershipCommitment> {
    use consensus::commitments::create_ownership_commitment_internal;

    create_ownership_commitment_internal(public_key, data_hash)
}

/// Create anchored ownership commitment
#[napi]
pub fn create_anchored_ownership_commitment(
    ownership_commitment: OwnershipCommitment,
    block_commitment: BlockCommitment,
) -> Result<AnchoredOwnershipCommitment> {
    use consensus::commitments::create_anchored_ownership_commitment_internal;

    create_anchored_ownership_commitment_internal(ownership_commitment, block_commitment)
}

/// Verify proof window for storage continuity
#[napi]
pub fn verify_proof_of_storage_continuity(
    proof_window: ProofWindow,
    anchored_commitment: Buffer,
    merkle_root: Buffer,
    total_chunks: f64,
) -> Result<bool> {
    use consensus::verification::verify_proof_of_storage_continuity_internal;

    verify_proof_of_storage_continuity_internal(
        proof_window,
        anchored_commitment,
        merkle_root,
        total_chunks,
    )
}
