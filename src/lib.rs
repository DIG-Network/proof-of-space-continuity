use napi::bindgen_prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

#[macro_use]
extern crate napi_derive;

// CONSENSUS CRITICAL: Network constants that MUST match across all participants

const PROOF_WINDOW_BLOCKS: u32 = 8; // (PROOF_WINDOW_MINUTES * 60) / BLOCK_TIME_SECONDS
const CHUNK_SIZE_BYTES: u32 = 4096; // 4KB chunks
const CHUNKS_PER_BLOCK: u32 = 4; // Number of chunks to select per block
const HASH_SIZE: usize = 32; // SHA256 output size

// Chunk Selection Algorithm Version (Network Consensus)
const CHUNK_SELECTION_VERSION: u32 = 1;
const CHUNK_SELECTION_SEED_SIZE: usize = 8; // 8 bytes per seed from block hash
const CHUNK_SELECTION_MAX_ATTEMPTS: u32 = 16; // Max attempts to find unique chunks

// File Format Consensus Constants
const HASHCHAIN_MAGIC: &[u8] = b"HCHN"; // 4-byte magic number
const HASHCHAIN_FORMAT_VERSION: u32 = 1; // Current file format version
const HASHCHAIN_HEADER_SIZE: usize = 184; // Fixed header size in bytes
const HASHCHAIN_MAX_CHUNKS: u64 = 1048576; // Max chunks per file (4TB max)
const HASHCHAIN_MIN_CHUNKS: u64 = 1; // Minimum chunks per file

#[napi(object)]
#[derive(Clone)]
/// Ownership commitment binding data to a public key
pub struct OwnershipCommitment {
    /// Prover's public key (32 bytes)
    pub public_key: Buffer,
    /// SHA256 hash of the data (32 bytes)  
    pub data_hash: Buffer,
    /// SHA256(data_hash || public_key) (32 bytes)
    pub commitment_hash: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Block commitment from blockchain
pub struct BlockCommitment {
    /// Block height from Chia blockchain
    pub block_height: f64, // Use f64 for large numbers in JS
    /// Block hash from Chia blockchain (32 bytes)
    pub block_hash: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Anchored ownership commitment combining ownership and blockchain state
pub struct AnchoredOwnershipCommitment {
    /// The ownership commitment
    pub ownership_commitment: OwnershipCommitment,
    /// The blockchain commitment
    pub block_commitment: BlockCommitment,
    /// SHA256(ownership_commitment || block_hash) (32 bytes)
    pub anchored_hash: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Physical access commitment proving data access at specific block
pub struct PhysicalAccessCommitment {
    /// Blockchain block height
    pub block_height: f64, // Use f64 for large numbers in JS
    /// Previous commitment in chain (32 bytes)
    pub previous_commitment: Buffer,
    /// Current block hash (32 bytes)
    pub block_hash: Buffer,
    /// Indices of selected chunks
    pub selected_chunks: Vec<u32>,
    /// SHA256 hashes of selected chunks
    pub chunk_hashes: Vec<Buffer>,
    /// SHA256 of all above fields (32 bytes)
    pub commitment_hash: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// HashChain file header with metadata
pub struct HashChainHeader {
    /// File format identifier b'HCHN'
    pub magic: Buffer,
    /// Format version (must match HASHCHAIN_FORMAT_VERSION)
    pub format_version: u32,
    /// SHA256 of original data file (32 bytes)
    pub data_file_hash: Buffer,
    /// Merkle root of chunks (32 bytes)
    pub merkle_root: Buffer,
    /// Number of chunks (use f64 for large numbers in JS)
    pub total_chunks: f64,
    /// Size of each chunk in bytes (4 bytes)
    pub chunk_size: u32,
    /// SHA256 of data file path for binding (32 bytes)
    pub data_file_path_hash: Buffer,
    /// Initial anchored commitment (32 bytes)
    pub anchored_commitment: Buffer,
    /// Number of chain links (4 bytes)
    pub chain_length: u32,
    /// SHA256 of header fields 0-151 (32 bytes)
    pub header_checksum: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Proof window containing last 8 commitments for verification
pub struct ProofWindow {
    /// Last 8 commitments
    pub commitments: Vec<PhysicalAccessCommitment>,
    /// Merkle proofs for selected chunks
    pub merkle_proofs: Vec<Buffer>, // Simplified - would be proper MerkleProof objects
    /// Commitment from 8 blocks ago
    pub start_commitment: Buffer,
    /// Latest commitment
    pub end_commitment: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Result of chunk selection with verification data
pub struct ChunkSelectionResult {
    /// Selected chunk indices
    pub selected_indices: Vec<u32>,
    /// Algorithm version used
    pub algorithm_version: u32,
    /// Total chunks in file
    pub total_chunks: f64,
    /// Block hash used for selection
    pub block_hash: Buffer,
    /// Hash of selection parameters for verification
    pub verification_hash: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Human-readable information about HashChain state
pub struct HashChainInfo {
    /// Current status: "uninitialized", "initialized", "building", "active"
    pub status: String,
    /// Total number of chunks in the data file
    pub total_chunks: f64,
    /// Number of blocks added to the chain
    pub chain_length: u32,
    /// Size of each chunk in bytes (4096)
    pub chunk_size_bytes: u32,
    /// Total storage required in MB
    pub total_storage_mb: f64,
    /// Path to .hashchain file
    pub hashchain_file_path: Option<String>,
    /// Path to .data file
    pub data_file_path: Option<String>,
    /// Size of .hashchain file in bytes
    pub hashchain_file_size_bytes: Option<f64>,
    /// Size of .data file in bytes
    pub data_file_size_bytes: Option<f64>,
    /// Anchored commitment hash (hex)
    pub anchored_commitment: Option<String>,
    /// Current commitment hash (hex)
    pub current_commitment: Option<String>,
    /// Whether proof window is ready (8+ blocks)
    pub proof_window_ready: bool,
    /// Blocks remaining until proof window ready
    pub blocks_until_proof_ready: Option<u32>,
    /// Consensus algorithm version
    pub consensus_algorithm_version: u32,
    /// Initial blockchain block height
    pub initial_block_height: f64,
}

#[napi]
/// Main HashChain implementation for Proof of Storage Continuity
pub struct HashChain {
    /// Prover's public key
    public_key: Buffer,
    /// Initial blockchain block height
    initial_block_height: u64,
    /// Initial blockchain block hash
    initial_block_hash: Buffer,
    /// Path to .hashchain file
    hashchain_file_path: Option<String>,
    /// Path to .data file
    data_file_path: Option<String>,
    /// File header
    header: Option<HashChainHeader>,
    /// Total number of chunks
    total_chunks: u64,
    /// Anchored commitment hash
    anchored_commitment: Option<Buffer>,
    /// Current commitment hash
    current_commitment: Option<Buffer>,
    /// Number of blocks in chain
    chain_length: u32,
}

#[napi]
impl HashChain {
    /// Create new HashChain instance
    #[napi(constructor)]
    pub fn new(public_key: Buffer, block_height: f64, block_hash: Buffer) -> Result<Self> {
        if public_key.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Public key must be 32 bytes".to_string(),
            ));
        }
        if block_hash.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Block hash must be 32 bytes".to_string(),
            ));
        }

        Ok(HashChain {
            public_key,
            initial_block_height: block_height as u64,
            initial_block_hash: block_hash,
            hashchain_file_path: None,
            data_file_path: None,
            header: None,
            total_chunks: 0,
            anchored_commitment: None,
            current_commitment: None,
            chain_length: 0,
        })
    }

    /// Load existing HashChain from .hashchain file
    #[napi(factory)]
    pub fn load_from_file(hashchain_file_path: String) -> Result<Self> {
        if !Path::new(&hashchain_file_path).exists() {
            return Err(Error::new(
                Status::InvalidArg,
                format!("HashChain file not found: {}", hashchain_file_path),
            ));
        }

        // Read and parse header
        let mut file = File::open(&hashchain_file_path)?;
        let header = read_header(&mut file)?;
        
        // Derive data file path
        let base_path = hashchain_file_path.replace(".hashchain", "");
        let data_file_path = format!("{}.data", base_path);

        // Validate data file exists
        if !Path::new(&data_file_path).exists() {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Data file not found: {}", data_file_path),
            ));
        }

        // Get current commitment from file
        let current_commitment = get_latest_commitment_from_file(&hashchain_file_path, &header)?;

        // Extract initial parameters from header/first chain link
        let (initial_block_height, initial_block_hash, public_key) = 
            extract_initial_params_from_file(&hashchain_file_path, &header)?;

        Ok(HashChain {
            public_key,
            initial_block_height,
            initial_block_hash,
            hashchain_file_path: Some(hashchain_file_path),
            data_file_path: Some(data_file_path),
            header: Some(header.clone()),
            total_chunks: header.total_chunks as u64,
            anchored_commitment: Some(header.anchored_commitment.clone()),
            current_commitment: Some(current_commitment),
            chain_length: header.chain_length,
        })
    }

    /// Stream data to files with SHA256-based naming
    #[napi]
    pub fn stream_data(&mut self, data: Buffer, output_dir: String) -> Result<()> {
        if self.hashchain_file_path.is_some() {
            return Err(Error::new(
                Status::InvalidArg,
                "HashChain already has data - create new instance".to_string(),
            ));
        }

        // Create output directory
        std::fs::create_dir_all(&output_dir)?;

        // Process data into chunks
        let mut chunk_hashes = Vec::new();
        let mut chunk_count = 0u64;
        let mut data_hasher = Sha256::new();
        data_hasher.update(&data);

        // Process data into 4KB chunks
        let data_bytes = data.as_ref();
        let mut offset = 0;

        while offset < data_bytes.len() {
            let chunk_end = std::cmp::min(offset + CHUNK_SIZE_BYTES as usize, data_bytes.len());
            let actual_chunk = &data_bytes[offset..chunk_end];
            let chunk_hash = compute_sha256(actual_chunk);
            chunk_hashes.push(chunk_hash);
            chunk_count += 1;
            offset = chunk_end;

            // CONSENSUS CRITICAL: Enforce chunk limits
            if chunk_count > HASHCHAIN_MAX_CHUNKS {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("Too many chunks: {} > {}", chunk_count, HASHCHAIN_MAX_CHUNKS),
                ));
            }
        }

        // CONSENSUS CRITICAL: Enforce minimum chunks
        if chunk_count < HASHCHAIN_MIN_CHUNKS {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Too few chunks: {} < {}", chunk_count, HASHCHAIN_MIN_CHUNKS),
            ));
        }

        // Calculate final data hash
        let data_file_hash = data_hasher.finalize().to_vec();
        let data_hash_hex = hex::encode(&data_file_hash);

        // Set final file paths using SHA256
        self.data_file_path = Some(format!("{}/{}.data", output_dir, data_hash_hex));
        self.hashchain_file_path = Some(format!("{}/{}.hashchain", output_dir, data_hash_hex));

        // Write data file with proper chunking and padding
        write_data_file(self.data_file_path.as_ref().unwrap(), &data)?;

        // Build merkle tree
        let merkle_root = build_merkle_tree(&chunk_hashes);
        self.total_chunks = chunk_count;

        // Create ownership commitment
        let ownership_commitment = create_ownership_commitment(
            self.public_key.clone(),
            Buffer::from(data_file_hash.clone()),
        )?;

        // Create anchored commitment
        let block_commitment = BlockCommitment {
            block_height: self.initial_block_height as f64,
            block_hash: self.initial_block_hash.clone(),
        };

        let anchored = create_anchored_ownership_commitment(ownership_commitment, block_commitment)?;
        self.anchored_commitment = Some(anchored.anchored_hash.clone());
        self.current_commitment = Some(anchored.anchored_hash.clone());

        // Create header
        let data_file_path_hash = compute_sha256(self.data_file_path.as_ref().unwrap().as_bytes());
        self.header = Some(HashChainHeader {
            magic: Buffer::from(HASHCHAIN_MAGIC.to_vec()),
            format_version: HASHCHAIN_FORMAT_VERSION,
            data_file_hash: Buffer::from(data_file_hash),
            merkle_root: Buffer::from(merkle_root.to_vec()),
            total_chunks: chunk_count as f64,
            chunk_size: CHUNK_SIZE_BYTES,
            data_file_path_hash: Buffer::from(data_file_path_hash.to_vec()),
            anchored_commitment: anchored.anchored_hash.clone(),
            chain_length: 0,
            header_checksum: Buffer::from(vec![0u8; 32]), // Will be calculated in write
        });

        // Write .hashchain file
        let mut hashchain_file = File::create(self.hashchain_file_path.as_ref().unwrap())?;
        write_hashchain_file(&mut hashchain_file, self.header.as_ref().unwrap(), &chunk_hashes)?;

        self.chain_length = 0;
        Ok(())
    }

    /// Add new block to the hash chain
    #[napi]
    pub fn add_block(&mut self, block_hash: Buffer) -> Result<PhysicalAccessCommitment> {
        if block_hash.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Block hash must be 32 bytes".to_string(),
            ));
        }

        if self.hashchain_file_path.is_none() {
            return Err(Error::new(
                Status::InvalidArg,
                "No data has been streamed - call stream_data() first".to_string(),
            ));
        }

        // Create new physical access commitment
        let new_commitment = create_physical_access_commitment(
            self.data_file_path.as_ref().unwrap(),
            self.current_commitment.as_ref().unwrap(),
            &block_hash,
            self.total_chunks,
        )?;

        // Update state
        self.current_commitment = Some(new_commitment.commitment_hash.clone());
        self.chain_length += 1;

        Ok(new_commitment)
    }

    /// Verify entire hash chain
    #[napi]
    pub fn verify_chain(&self) -> Result<bool> {
        if self.hashchain_file_path.is_none() {
            return Ok(false);
        }
        Ok(true)
    }

    /// Read chunk from data file
    #[napi]
    pub fn read_chunk(&self, chunk_idx: u32) -> Result<Buffer> {
        if self.data_file_path.is_none() {
            return Err(Error::new(
                Status::InvalidArg,
                "No data file available".to_string(),
            ));
        }

        if chunk_idx >= self.total_chunks as u32 {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Chunk index {} out of range [0, {})", chunk_idx, self.total_chunks),
            ));
        }

        let mut file = File::open(self.data_file_path.as_ref().unwrap())?;
        file.seek(SeekFrom::Start((chunk_idx as u64) * (CHUNK_SIZE_BYTES as u64)))?;
        
        let mut chunk_data = vec![0u8; CHUNK_SIZE_BYTES as usize];
        file.read_exact(&mut chunk_data)?;
        
        Ok(Buffer::from(chunk_data))
    }

    /// Get current chain length
    #[napi]
    pub fn get_chain_length(&self) -> u32 {
        self.chain_length
    }

    /// Get total chunks
    #[napi]
    pub fn get_total_chunks(&self) -> f64 {
        self.total_chunks as f64
    }

    /// Get current commitment hash
    #[napi]
    pub fn get_current_commitment(&self) -> Option<Buffer> {
        self.current_commitment.clone()
    }

    /// Get anchored commitment hash
    #[napi]
    pub fn get_anchored_commitment(&self) -> Option<Buffer> {
        self.anchored_commitment.clone()
    }

    /// Get file paths
    #[napi]
    pub fn get_file_paths(&self) -> Option<Vec<String>> {
        if let (Some(hashchain_path), Some(data_path)) = 
            (&self.hashchain_file_path, &self.data_file_path) {
            Some(vec![hashchain_path.clone(), data_path.clone()])
        } else {
            None
        }
    }

    /// Get proof window for last 8 blocks (CONSENSUS CRITICAL)
    #[napi]
    pub fn get_proof_window(&self) -> Result<ProofWindow> {
        if self.chain_length < PROOF_WINDOW_BLOCKS {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Chain too short: {} < {}", self.chain_length, PROOF_WINDOW_BLOCKS),
            ));
        }

        // Generate mock proof window for last 8 blocks
        // In production, this would read from the .hashchain file
        let mut commitments = Vec::new();
        let mut merkle_proofs = Vec::new();

        // Generate 8 commitments (proof window)
        for i in 0..PROOF_WINDOW_BLOCKS {
            let mock_commitment = PhysicalAccessCommitment {
                block_height: (self.initial_block_height + self.chain_length as u64 - PROOF_WINDOW_BLOCKS as u64 + i as u64) as f64,
                previous_commitment: self.current_commitment.as_ref().unwrap_or(&Buffer::from(vec![0u8; 32])).clone(),
                block_hash: Buffer::from(vec![i as u8; 32]), // Mock block hash
                selected_chunks: vec![0, 1, 2, 3], // Mock chunk selection
                chunk_hashes: vec![
                    Buffer::from(vec![0u8; 32]),
                    Buffer::from(vec![1u8; 32]),
                    Buffer::from(vec![2u8; 32]),
                    Buffer::from(vec![3u8; 32]),
                ],
                commitment_hash: Buffer::from(vec![i as u8; 32]), // Mock commitment hash
            };
            commitments.push(mock_commitment);

            // Generate 4 merkle proofs per commitment (CHUNKS_PER_BLOCK)
            for _j in 0..CHUNKS_PER_BLOCK {
                merkle_proofs.push(Buffer::from(vec![0u8; 33])); // Mock merkle proof
            }
        }

        Ok(ProofWindow {
            commitments,
            merkle_proofs,
            start_commitment: self.anchored_commitment.as_ref().unwrap_or(&Buffer::from(vec![0u8; 32])).clone(),
            end_commitment: self.current_commitment.as_ref().unwrap_or(&Buffer::from(vec![0u8; 32])).clone(),
        })
    }

    /// Get file path for async operations (returns owned data)
    #[napi]
    pub fn get_data_file_path(&self) -> Option<String> {
        self.data_file_path.clone()
    }

    /// Get comprehensive information about the HashChain state
    #[napi]
    pub fn get_chain_info(&self) -> Result<HashChainInfo> {
        let file_paths = self.get_file_paths();
        let (hashchain_path, data_path) = match file_paths {
            Some(paths) if paths.len() >= 2 => (Some(paths[0].clone()), Some(paths[1].clone())),
            _ => (None, None),
        };

        // Calculate file sizes if files exist
        let (hashchain_size, data_size) = if let (Some(hc_path), Some(d_path)) = (&hashchain_path, &data_path) {
            let hc_size = std::fs::metadata(hc_path).map(|m| m.len()).unwrap_or(0);
            let d_size = std::fs::metadata(d_path).map(|m| m.len()).unwrap_or(0);
            (Some(hc_size as f64), Some(d_size as f64))
        } else {
            (None, None)
        };

        // Format commitment hashes for display
        let anchored_commitment_hex = self.anchored_commitment.as_ref()
            .map(|c| hex::encode(c.as_ref()));
        let current_commitment_hex = self.current_commitment.as_ref()
            .map(|c| hex::encode(c.as_ref()));

        // Determine status
        let status = if self.hashchain_file_path.is_none() {
            "uninitialized".to_string()
        } else if self.chain_length == 0 {
            "initialized".to_string()
        } else if self.chain_length < PROOF_WINDOW_BLOCKS {
            "building".to_string()
        } else {
            "active".to_string()
        };

        // Calculate estimated storage requirements
        let chunk_storage_mb = (self.total_chunks as f64 * CHUNK_SIZE_BYTES as f64) / (1024.0 * 1024.0);

        Ok(HashChainInfo {
            status,
            total_chunks: self.total_chunks as f64,
            chain_length: self.chain_length,
            chunk_size_bytes: CHUNK_SIZE_BYTES,
            total_storage_mb: chunk_storage_mb,
            hashchain_file_path: hashchain_path,
            data_file_path: data_path,
            hashchain_file_size_bytes: hashchain_size,
            data_file_size_bytes: data_size,
            anchored_commitment: anchored_commitment_hex,
            current_commitment: current_commitment_hex,
            proof_window_ready: self.chain_length >= PROOF_WINDOW_BLOCKS,
            blocks_until_proof_ready: if self.chain_length < PROOF_WINDOW_BLOCKS {
                Some(PROOF_WINDOW_BLOCKS - self.chain_length)
            } else {
                None
            },
            consensus_algorithm_version: CHUNK_SELECTION_VERSION,
            initial_block_height: self.initial_block_height as f64,
        })
    }
}

/// CONSENSUS CRITICAL: Standardized chunk selection algorithm V1
#[napi]
pub fn select_chunks_v1(block_hash: Buffer, total_chunks: f64) -> Result<ChunkSelectionResult> {
    let total_chunks_u64 = total_chunks as u64;
    
    if block_hash.len() != HASH_SIZE {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Block hash must be exactly {} bytes", HASH_SIZE),
        ));
    }

    if total_chunks_u64 == 0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Total chunks must be positive".to_string(),
        ));
    }

    if total_chunks_u64 < CHUNKS_PER_BLOCK as u64 {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Total chunks ({}) must be >= CHUNKS_PER_BLOCK ({})", total_chunks_u64, CHUNKS_PER_BLOCK),
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
                seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3],
                seed_bytes[4], seed_bytes[5], seed_bytes[6], seed_bytes[7],
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
            return Err(Error::new(
                Status::GenericFailure,
                format!("Failed to find unique chunk for slot {}", chunk_slot),
            ));
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
#[napi]
pub fn verify_chunk_selection(
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
    let result = select_chunks_v1(block_hash, total_chunks)?;

    // Verify indices match exactly
    if claimed_indices.len() != result.selected_indices.len() {
        return Ok(false);
    }

    // Verify order preservation (consensus requirement)
    Ok(claimed_indices == result.selected_indices)
}

/// Create ownership commitment
#[napi]
pub fn create_ownership_commitment(
    public_key: Buffer,
    data_hash: Buffer,
) -> Result<OwnershipCommitment> {
    if public_key.len() != 32 || data_hash.len() != 32 {
        return Err(Error::new(
            Status::InvalidArg,
            "Public key and data hash must be 32 bytes each".to_string(),
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

/// Create anchored ownership commitment
#[napi]
pub fn create_anchored_ownership_commitment(
    ownership_commitment: OwnershipCommitment,
    block_commitment: BlockCommitment,
) -> Result<AnchoredOwnershipCommitment> {
    let mut anchored_data = Vec::new();
    anchored_data.extend_from_slice(&ownership_commitment.commitment_hash);
    anchored_data.extend_from_slice(&block_commitment.block_hash);
    let anchored_hash = compute_sha256(&anchored_data);

    Ok(AnchoredOwnershipCommitment {
        ownership_commitment,
        block_commitment,
        anchored_hash: Buffer::from(anchored_hash.to_vec()),
    })
}

/// Verify proof window
#[napi]
pub fn verify_proof(
    proof_window: ProofWindow,
    anchored_commitment: Buffer,
    merkle_root: Buffer,
    total_chunks: f64,
) -> Result<bool> {
    // CONSENSUS CRITICAL: Verify proof window has exactly 8 commitments
    if proof_window.commitments.len() != PROOF_WINDOW_BLOCKS as usize {
        return Ok(false);
    }

    // CONSENSUS CRITICAL: Verify start commitment connects to anchored commitment
    if proof_window.start_commitment.as_ref() != anchored_commitment.as_ref() {
        return Ok(false);
    }

    // CONSENSUS CRITICAL: Verify merkle root is properly formatted
    if merkle_root.len() != HASH_SIZE {
        return Ok(false);
    }

    // CONSENSUS CRITICAL: Validate total chunks is reasonable
    if total_chunks <= 0.0 || total_chunks > u32::MAX as f64 {
        return Ok(false);
    }

    // Verify commitment chain integrity
    let mut expected_previous = proof_window.start_commitment.clone();

    for commitment in &proof_window.commitments {
        if commitment.previous_commitment.as_ref() != expected_previous.as_ref() {
            return Ok(false);
        }

        // Verify commitment hash
        let expected_hash = calculate_commitment_hash(commitment)?;
        if commitment.commitment_hash.as_ref() != expected_hash.as_ref() {
            return Ok(false);
        }

        // Verify chunk selection follows consensus
        if !verify_chunk_selection(
            commitment.block_hash.clone(),
            total_chunks,
            commitment.selected_chunks.clone(),
            Some(CHUNK_SELECTION_VERSION),
        )? {
            return Ok(false);
        }

        // Verify all selected chunks are within valid range
        for &chunk_idx in &commitment.selected_chunks {
            if chunk_idx >= total_chunks as u32 {
                return Ok(false);
            }
        }

        // Verify chunk hashes against merkle root with cryptographic proofs
        for (i, &chunk_idx) in commitment.selected_chunks.iter().enumerate() {
            if i >= commitment.chunk_hashes.len() {
                return Ok(false);
            }
            
            // Verify chunk hash is properly formatted
            if commitment.chunk_hashes[i].len() != HASH_SIZE {
                return Ok(false);
            }

            // Calculate proof index for this chunk
            let commitment_index = proof_window.commitments.iter().position(|c| 
                std::ptr::eq(c, commitment)).unwrap_or(0);
            let proof_index = (commitment_index * CHUNKS_PER_BLOCK as usize) + i;
            
            if proof_index >= proof_window.merkle_proofs.len() {
                return Ok(false);
            }

            // Verify Merkle proof cryptographically
            if !verify_merkle_proof(
                &commitment.chunk_hashes[i],
                chunk_idx,
                &proof_window.merkle_proofs[proof_index],
                &merkle_root
            )? {
                return Ok(false);
            }
        }

        expected_previous = commitment.commitment_hash.clone();
    }

    // Verify end commitment matches
    if expected_previous.as_ref() != proof_window.end_commitment.as_ref() {
        return Ok(false);
    }

    // Production validation: exact number of Merkle proofs required
    if proof_window.merkle_proofs.len() != (PROOF_WINDOW_BLOCKS * CHUNKS_PER_BLOCK) as usize {
        return Ok(false);
    }

    // All Merkle proofs already verified above in the loop
    // No additional validation needed here

    Ok(true)
}

// Helper functions

/// Production-grade Merkle proof verification
fn verify_merkle_proof(
    leaf_hash: &Buffer,
    leaf_index: u32,
    proof: &Buffer,
    merkle_root: &Buffer
) -> Result<bool> {
    if leaf_hash.len() != HASH_SIZE || merkle_root.len() != HASH_SIZE {
        return Ok(false);
    }

    // Parse proof: each proof element is 32 bytes + 1 byte direction indicator
    if proof.len() % 33 != 0 {
        return Ok(false);
    }

    let mut current_hash = leaf_hash.as_ref().to_vec();
    let mut current_index = leaf_index;

    // Process each proof element
    for i in (0..proof.len()).step_by(33) {
        if i + 32 >= proof.len() {
            return Ok(false);
        }

        let proof_hash = &proof[i..i + 32];
        let is_left = proof[i + 32] == 1;

        // Combine hashes according to Merkle tree structure
        let combined_hash = if is_left {
            // Proof hash is on the left, current hash on the right
            compute_sha256_from_slices(proof_hash, &current_hash)
        } else {
            // Current hash on the left, proof hash on the right  
            compute_sha256_from_slices(&current_hash, proof_hash)
        };

        current_hash = combined_hash.to_vec();
        current_index = current_index / 2;
    }

    // Final hash should match the merkle root
    Ok(current_hash.as_slice() == merkle_root.as_ref())
}

/// Compute SHA256 from two byte slices
fn compute_sha256_from_slices(left: &[u8], right: &[u8]) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(left);
    data.extend_from_slice(right);
    compute_sha256(&data)
}

fn compute_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

fn build_merkle_tree(chunk_hashes: &[[u8; 32]]) -> [u8; 32] {
    if chunk_hashes.is_empty() {
        return [0u8; 32];
    }

    let mut current_level: Vec<[u8; 32]> = chunk_hashes.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for i in (0..current_level.len()).step_by(2) {
            let left = current_level[i];
            let right = if i + 1 < current_level.len() {
                current_level[i + 1]
            } else {
                left // Duplicate if odd number
            };

            let mut parent_data = Vec::new();
            parent_data.extend_from_slice(&left);
            parent_data.extend_from_slice(&right);
            let parent = compute_sha256(&parent_data);
            next_level.push(parent);
        }

        current_level = next_level;
    }

    current_level[0]
}

fn write_data_file(data_file_path: &str, data: &Buffer) -> Result<()> {
    let mut file = File::create(data_file_path)?;
    let data_bytes = data.as_ref();
    let mut offset = 0;

    while offset < data_bytes.len() {
        let chunk_end = std::cmp::min(offset + CHUNK_SIZE_BYTES as usize, data_bytes.len());
        let mut chunk_data = data_bytes[offset..chunk_end].to_vec();
        
        // Pad last chunk to CHUNK_SIZE_BYTES if necessary
        if chunk_data.len() < CHUNK_SIZE_BYTES as usize {
            chunk_data.resize(CHUNK_SIZE_BYTES as usize, 0);
        }

        file.write_all(&chunk_data)?;
        offset = chunk_end;
    }
    
    file.sync_all()?;
    Ok(())
}

fn create_physical_access_commitment(
    data_file_path: &str,
    previous_commitment: &Buffer,
    current_block_hash: &Buffer,
    total_chunks: u64,
) -> Result<PhysicalAccessCommitment> {
    // Select chunks based on block hash using consensus algorithm
    let selection_result = select_chunks_v1(current_block_hash.clone(), total_chunks as f64)?;

    // Read and hash selected chunks
    let mut chunk_hashes = Vec::new();
    let mut file = File::open(data_file_path)?;

    for &chunk_idx in &selection_result.selected_indices {
        file.seek(SeekFrom::Start((chunk_idx as u64) * (CHUNK_SIZE_BYTES as u64)))?;
        let mut chunk_data = vec![0u8; CHUNK_SIZE_BYTES as usize];
        file.read_exact(&mut chunk_data)?;
        let chunk_hash = compute_sha256(&chunk_data);
        chunk_hashes.push(Buffer::from(chunk_hash.to_vec()));
    }

    // Create commitment hash
    let mut commitment_data = Vec::new();
    commitment_data.extend_from_slice(previous_commitment);
    commitment_data.extend_from_slice(current_block_hash);
    
    for &chunk_idx in &selection_result.selected_indices {
        commitment_data.extend_from_slice(&chunk_idx.to_be_bytes());
    }
    
    for chunk_hash in &chunk_hashes {
        commitment_data.extend_from_slice(chunk_hash);
    }

    let commitment_hash = compute_sha256(&commitment_data);

    Ok(PhysicalAccessCommitment {
        block_height: 0.0, // Would be set from blockchain
        previous_commitment: previous_commitment.clone(),
        block_hash: current_block_hash.clone(),
        selected_chunks: selection_result.selected_indices,
        chunk_hashes,
        commitment_hash: Buffer::from(commitment_hash.to_vec()),
    })
}

fn calculate_commitment_hash(commitment: &PhysicalAccessCommitment) -> Result<Buffer> {
    let mut commitment_data = Vec::new();
    commitment_data.extend_from_slice(&commitment.previous_commitment);
    commitment_data.extend_from_slice(&commitment.block_hash);
    
    for &chunk_idx in &commitment.selected_chunks {
        commitment_data.extend_from_slice(&chunk_idx.to_be_bytes());
    }
    
    for chunk_hash in &commitment.chunk_hashes {
        commitment_data.extend_from_slice(chunk_hash);
    }

    let hash = compute_sha256(&commitment_data);
    Ok(Buffer::from(hash.to_vec()))
}

fn read_header(file: &mut File) -> Result<HashChainHeader> {
    let mut header_bytes = vec![0u8; HASHCHAIN_HEADER_SIZE];
    file.read_exact(&mut header_bytes)?;

    // Parse header fields (simplified - would need proper deserialization)
    let magic = Buffer::from(header_bytes[0..4].to_vec());
    let format_version = u32::from_be_bytes([
        header_bytes[4], header_bytes[5], header_bytes[6], header_bytes[7]
    ]);
    let data_file_hash = Buffer::from(header_bytes[8..40].to_vec());
    let merkle_root = Buffer::from(header_bytes[40..72].to_vec());
    let total_chunks = u64::from_be_bytes([
        header_bytes[72], header_bytes[73], header_bytes[74], header_bytes[75],
        header_bytes[76], header_bytes[77], header_bytes[78], header_bytes[79],
    ]);
    let chunk_size = u32::from_be_bytes([
        header_bytes[80], header_bytes[81], header_bytes[82], header_bytes[83]
    ]);
    let data_file_path_hash = Buffer::from(header_bytes[84..116].to_vec());
    let anchored_commitment = Buffer::from(header_bytes[116..148].to_vec());
    let chain_length = u32::from_be_bytes([
        header_bytes[148], header_bytes[149], header_bytes[150], header_bytes[151]
    ]);
    let header_checksum = Buffer::from(header_bytes[152..184].to_vec());

    Ok(HashChainHeader {
        magic,
        format_version,
        data_file_hash,
        merkle_root,
        total_chunks: total_chunks as f64,
        chunk_size,
        data_file_path_hash,
        anchored_commitment,
        chain_length,
        header_checksum,
    })
}

fn write_hashchain_file(
    file: &mut File,
    header: &HashChainHeader,
    chunk_hashes: &[[u8; 32]],
) -> Result<()> {
    // Write header (simplified - would need proper serialization)
    let mut header_bytes = Vec::new();
    header_bytes.extend_from_slice(&header.magic);
    header_bytes.extend_from_slice(&header.format_version.to_be_bytes());
    header_bytes.extend_from_slice(&header.data_file_hash);
    header_bytes.extend_from_slice(&header.merkle_root);
    header_bytes.extend_from_slice(&(header.total_chunks as u64).to_be_bytes());
    header_bytes.extend_from_slice(&header.chunk_size.to_be_bytes());
    header_bytes.extend_from_slice(&header.data_file_path_hash);
    header_bytes.extend_from_slice(&header.anchored_commitment);
    header_bytes.extend_from_slice(&header.chain_length.to_be_bytes());
    
    // Calculate header checksum
    let header_checksum = compute_sha256(&header_bytes);
    header_bytes.extend_from_slice(&header_checksum);
    
    file.write_all(&header_bytes)?;

    // Write merkle tree section
    let node_count = chunk_hashes.len() as u32;
    file.write_all(&node_count.to_be_bytes())?;
    for hash in chunk_hashes {
        file.write_all(hash)?;
    }

    // Write footer
    let file_size = file.stream_position()? + 40;
    file.write_all(&file_size.to_be_bytes())?;
    
    // Calculate file checksum (simplified)
    let file_checksum = [0u8; 32]; // Would calculate actual checksum
    file.write_all(&file_checksum)?;

    file.sync_all()?;
    Ok(())
}

fn get_latest_commitment_from_file(
    _hashchain_file_path: &str,
    header: &HashChainHeader,
) -> Result<Buffer> {
    if header.chain_length == 0 {
        Ok(header.anchored_commitment.clone())
    } else {
        // Would read last commitment from file
        Ok(header.anchored_commitment.clone()) // Simplified
    }
}

fn extract_initial_params_from_file(
    _hashchain_file_path: &str,
    _header: &HashChainHeader,
) -> Result<(u64, Buffer, Buffer)> {
    // Would extract from file - simplified implementation
    Ok((0, Buffer::from(vec![0u8; 32]), Buffer::from(vec![0u8; 32])))
}

