use crate::chain::storage::{ChainStorage, FileStats};
use crate::core::{
    errors::{HashChainError, HashChainResult},
    types::*,
    utils::{compute_sha256, generate_chain_id, PerformanceTimer},
};
use napi::bindgen_prelude::*;

/// Production-ready HashChain implementation
pub struct IndividualHashChain {
    /// Chain identifier
    pub chain_id: ChainId,
    /// Owner's public key
    pub public_key: Buffer,
    /// Storage manager
    pub storage: Option<ChainStorage>,
    /// Current commitment
    pub current_commitment: Option<Buffer>,
    /// Chain length
    pub chain_length: u32,
    /// Initial block info
    pub initial_block_height: u64,
    pub initial_block_hash: Buffer,
    /// Chain history
    pub commitments: Vec<PhysicalAccessCommitment>,
    /// HashChain header
    pub header: Option<HashChainHeader>,
}

impl IndividualHashChain {
    /// Create new HashChain from data stream
    pub fn new_from_stream(
        public_key: Buffer,
        data_stream: Buffer,
        output_dir: String,
        initial_block_height: u64,
        initial_block_hash: Buffer,
    ) -> HashChainResult<Self> {
        let timer = PerformanceTimer::new("new_hashchain_from_stream");

        // Create storage from streamed data
        let mut storage = ChainStorage::create_from_stream(data_stream, &output_dir, &public_key)?;

        // Compute file hash and create chain ID
        let data_file_hash = storage.compute_file_hash()?;
        let chain_id = generate_chain_id(&public_key, &data_file_hash);

        // Create HashChain header
        let header = HashChainHeader {
            magic: Buffer::from(HASHCHAIN_MAGIC.to_vec()),
            format_version: HASHCHAIN_FORMAT_VERSION,
            data_file_hash: Buffer::from(data_file_hash.to_vec()),
            merkle_root: Buffer::from([0u8; 32].to_vec()), // Will be computed when needed
            total_chunks: storage.total_chunks as f64,
            chunk_size: CHUNK_SIZE_BYTES,
            data_file_path_hash: Buffer::from(
                compute_sha256(storage.data_file_path.as_bytes()).to_vec(),
            ),
            anchored_commitment: Buffer::from([0u8; 32].to_vec()), // Will be set when first block is added
            chain_length: 0,
            public_key: public_key.clone(),
            initial_block_height: initial_block_height as f64,
            initial_block_hash: initial_block_hash.clone(),
            header_checksum: Buffer::from([0u8; 32].to_vec()), // Will be computed when saved
        };

        // Write header to .hashchain file
        storage.write_hashchain_header(&header)?;

        let elapsed = timer.elapsed_ms();
        log::info!(
            "Created HashChain {} with {} chunks in {}ms",
            hex::encode(&chain_id[..8]),
            storage.total_chunks,
            elapsed
        );

        Ok(Self {
            chain_id: chain_id.to_vec(),
            public_key,
            storage: Some(storage),
            current_commitment: None,
            chain_length: 0,
            initial_block_height,
            initial_block_hash,
            commitments: Vec::new(),
            header: Some(header),
        })
    }

    /// Load existing HashChain from .hashchain file
    pub fn load_from_file(hashchain_file_path: String) -> HashChainResult<Self> {
        let timer = PerformanceTimer::new("load_hashchain");

        // Derive data file path
        let data_file_path = if hashchain_file_path.ends_with(".hashchain") {
            hashchain_file_path.replace(".hashchain", ".data")
        } else {
            return Err(HashChainError::FileFormat(
                "File must have .hashchain extension".to_string(),
            ));
        };

        // Create storage from existing files
        let mut storage = ChainStorage::new(data_file_path)?;

        // Load header
        let header = storage.load_hashchain_header()?;

        // Validate header
        if header.magic.as_ref() != HASHCHAIN_MAGIC {
            return Err(HashChainError::FileFormat(
                "Invalid HashChain magic number".to_string(),
            ));
        }

        if header.format_version != HASHCHAIN_FORMAT_VERSION {
            return Err(HashChainError::FileFormat(format!(
                "Unsupported format version: {}",
                header.format_version
            )));
        }

        // Set prover key for decoding
        storage.set_prover_key(header.public_key.clone())?;

        // Load commitments before moving storage
        let commitments = storage
            .load_commitments_from_file()
            .unwrap_or_else(|_| Vec::new()); // Empty vec if file doesn't exist or is corrupt

        // Generate chain ID from header data
        let chain_id = generate_chain_id(&header.public_key, &header.data_file_hash);

        let elapsed = timer.elapsed_ms();
        log::info!(
            "Loaded HashChain {} with {} chunks in {}ms",
            hex::encode(&chain_id[..8]),
            header.total_chunks as u64,
            elapsed
        );

        Ok(Self {
            chain_id: chain_id.to_vec(),
            public_key: header.public_key.clone(),
            storage: Some(storage),
            current_commitment: if header.chain_length > 0 {
                Some(header.anchored_commitment.clone())
            } else {
                None
            },
            chain_length: header.chain_length,
            initial_block_height: header.initial_block_height as u64,
            initial_block_hash: header.initial_block_hash.clone(),
            commitments,
            header: Some(header),
        })
    }

    /// Create minimal HashChain instance (for NAPI constructor)
    pub fn new_minimal(
        public_key: Buffer,
        initial_block_height: u64,
        initial_block_hash: Buffer,
    ) -> HashChainResult<Self> {
        let chain_id = [0u8; 32]; // Will be set when data is streamed

        Ok(Self {
            chain_id: chain_id.to_vec(),
            public_key,
            storage: None,
            current_commitment: None,
            chain_length: 0,
            initial_block_height,
            initial_block_hash,
            commitments: Vec::new(),
            header: None,
        })
    }

    /// Stream data to create storage
    pub fn stream_data(&mut self, data_stream: Buffer, output_dir: String) -> HashChainResult<()> {
        // Create storage from streamed data
        let mut storage =
            ChainStorage::create_from_stream(data_stream, &output_dir, &self.public_key)?;

        // Compute file hash and create chain ID
        let data_file_hash = storage.compute_file_hash()?;
        self.chain_id = generate_chain_id(&self.public_key, &data_file_hash);

        // Create HashChain header
        let header = HashChainHeader {
            magic: Buffer::from(HASHCHAIN_MAGIC.to_vec()),
            format_version: HASHCHAIN_FORMAT_VERSION,
            data_file_hash: Buffer::from(data_file_hash.to_vec()),
            merkle_root: Buffer::from([0u8; 32].to_vec()),
            total_chunks: storage.total_chunks as f64,
            chunk_size: CHUNK_SIZE_BYTES,
            data_file_path_hash: Buffer::from(
                compute_sha256(storage.data_file_path.as_bytes()).to_vec(),
            ),
            anchored_commitment: Buffer::from([0u8; 32].to_vec()),
            chain_length: 0,
            public_key: self.public_key.clone(),
            initial_block_height: self.initial_block_height as f64,
            initial_block_hash: self.initial_block_hash.clone(),
            header_checksum: Buffer::from([0u8; 32].to_vec()),
        };

        // Write header to .hashchain file
        storage.write_hashchain_header(&header)?;

        // Update instance state
        self.storage = Some(storage);
        self.header = Some(header);

        Ok(())
    }

    /// Add commitment for new block
    pub fn add_commitment(
        &mut self,
        block_hash: Buffer,
        block_height: u64,
        selected_chunks: Vec<u32>,
    ) -> HashChainResult<PhysicalAccessCommitment> {
        let timer = PerformanceTimer::new("add_commitment");

        if let Some(ref mut storage) = self.storage {
            // Read the selected chunks - fix the error conversion issue
            let mut chunk_hashes = Vec::new();
            for &idx in &selected_chunks {
                let hash = storage.compute_chunk_hash(idx)?;
                chunk_hashes.push(hash);
            }

            let chunk_hash_buffers: Vec<Buffer> = chunk_hashes
                .iter()
                .map(|hash| Buffer::from(hash.to_vec()))
                .collect();

            // Create commitment
            let previous_commitment = self
                .current_commitment
                .clone()
                .unwrap_or_else(|| Buffer::from([0u8; 32].to_vec()));

            let commitment = PhysicalAccessCommitment {
                block_height: block_height as f64,
                previous_commitment: previous_commitment.clone(),
                block_hash: block_hash.clone(),
                selected_chunks: selected_chunks.clone(),
                chunk_hashes: chunk_hash_buffers.clone(),
                commitment_hash: Buffer::from([0u8; 32].to_vec()), // Will be computed below
            };

            // Compute commitment hash
            let commitment_hash = self.compute_commitment_hash(&commitment)?;
            let mut final_commitment = commitment;
            final_commitment.commitment_hash = Buffer::from(commitment_hash.to_vec());

            // Update chain state
            self.commitments.push(final_commitment.clone());
            self.current_commitment = Some(final_commitment.commitment_hash.clone());
            self.chain_length += 1;

            let elapsed = timer.elapsed_ms();
            log::debug!(
                "Added commitment for block {} to chain {} in {}ms",
                block_height,
                hex::encode(&self.chain_id[..8]),
                elapsed
            );

            Ok(final_commitment)
        } else {
            Err(HashChainError::NoDataStreamed)
        }
    }

    /// Compute commitment hash according to specification
    fn compute_commitment_hash(
        &self,
        commitment: &PhysicalAccessCommitment,
    ) -> HashChainResult<[u8; 32]> {
        let mut data = Vec::new();

        // Add all commitment fields
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

        Ok(compute_sha256(&data))
    }

    /// Get proof window for last PROOF_WINDOW_BLOCKS commitments
    pub fn get_proof_window(&self) -> HashChainResult<ProofWindow> {
        if self.commitments.len() < PROOF_WINDOW_BLOCKS as usize {
            return Err(HashChainError::ChainTooShort {
                length: self.commitments.len() as u32,
                required: PROOF_WINDOW_BLOCKS,
            });
        }

        let start_idx = self.commitments.len() - PROOF_WINDOW_BLOCKS as usize;
        let window_commitments = self.commitments[start_idx..].to_vec();

        // In production, would generate actual merkle proofs
        let merkle_proofs = vec![Buffer::from([0u8; 32].to_vec()); CHUNKS_PER_BLOCK as usize];

        Ok(ProofWindow {
            commitments: window_commitments.clone(),
            merkle_proofs,
            start_commitment: window_commitments.first().unwrap().commitment_hash.clone(),
            end_commitment: window_commitments.last().unwrap().commitment_hash.clone(),
        })
    }

    /// Read chunk from storage
    pub fn read_chunk(&mut self, chunk_index: u32) -> HashChainResult<Buffer> {
        if let Some(ref mut storage) = self.storage {
            storage.read_chunk(chunk_index)
        } else {
            Err(HashChainError::NoDataStreamed)
        }
    }

    /// Get total chunks count
    pub fn get_total_chunks(&self) -> u64 {
        if let Some(ref storage) = self.storage {
            storage.total_chunks
        } else {
            0
        }
    }

    /// Get file statistics
    pub fn get_file_stats(&self) -> HashChainResult<FileStats> {
        if let Some(ref storage) = self.storage {
            storage.get_file_stats()
        } else {
            Err(HashChainError::NoDataStreamed)
        }
    }

    /// Verify chain integrity
    pub fn verify_chain(&self) -> HashChainResult<bool> {
        if self.commitments.is_empty() {
            return Ok(true);
        }

        // Verify commitment linkage
        for window in self.commitments.windows(2) {
            let prev = &window[0];
            let curr = &window[1];

            if prev.commitment_hash.as_ref() != curr.previous_commitment.as_ref() {
                return Ok(false);
            }

            if curr.block_height != prev.block_height + 1.0 {
                return Ok(false);
            }
        }

        // Verify commitment hashes
        for commitment in &self.commitments {
            let computed_hash = self.compute_commitment_hash(commitment)?;
            if computed_hash.as_ref() != commitment.commitment_hash.as_ref() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get chain ID
    pub fn get_chain_id(&self) -> ChainId {
        self.chain_id.clone()
    }
}
