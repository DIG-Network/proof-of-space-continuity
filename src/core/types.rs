use napi::bindgen_prelude::*;
use napi_derive::napi;
// Production dependencies (currently unused but may be needed for future merkle implementations)
// use rs_merkle::{algorithms::Sha256 as MerkleSha256, MerkleProof, MerkleTree as RsMerkleTree};

// Network Consensus Constants (MUST match across all participants)
pub const BLOCK_TIME_SECONDS: u32 = 16;
pub const PROOF_WINDOW_MINUTES: u32 = 2;
pub const PROOF_WINDOW_BLOCKS: u32 = 8; // (PROOF_WINDOW_MINUTES * 60) / BLOCK_TIME_SECONDS
pub const CHUNK_SIZE_BYTES: u32 = 4096; // 4KB chunks
pub const CHUNKS_PER_BLOCK: u32 = 4; // Number of chunks to select per block
pub const HASH_SIZE: usize = 32; // SHA256 output size

// Chunk Selection Algorithm Version (Network Consensus)
pub const CHUNK_SELECTION_VERSION: u32 = 1;
pub const CHUNK_SELECTION_SEED_SIZE: usize = 8; // 8 bytes per seed from block hash
pub const CHUNK_SELECTION_MAX_ATTEMPTS: u32 = 16; // Max attempts to find unique chunks

// File Format Consensus Constants
pub const HASHCHAIN_MAGIC: &[u8] = b"HCHN"; // 4-byte magic number
pub const HASHCHAIN_FORMAT_VERSION: u32 = 1; // Current file format version
pub const HASHCHAIN_HEADER_SIZE: usize = 256; // Fixed header size in bytes
pub const HASHCHAIN_MAX_CHUNKS: u64 = 1048576; // Max chunks per file (4TB max)
pub const HASHCHAIN_MIN_CHUNKS: u64 = 1; // Minimum chunks per file

// Hierarchical Temporal Proof Parameters
pub const GLOBAL_ROOT_ITERATIONS: u32 = 10000; // Full iterations for root
pub const REGIONAL_ITERATIONS: u32 = 5000; // Medium iterations for regions
pub const GROUP_ITERATIONS: u32 = 1000; // Light iterations for groups
pub const CHAINS_PER_GROUP: u32 = 1000; // Standard group size
pub const GROUPS_PER_REGION: u32 = 10; // Standard region size

// Scalability Parameters
pub const MAX_CHAINS_PER_INSTANCE: u32 = 100000; // Support up to 100K chains
pub const GLOBAL_STATE_UPDATE_INTERVAL: u32 = 16; // Every block
pub const REMOVAL_DELAY_BLOCKS: u32 = 8; // Delay before chain removal

// Performance Targets
pub const BLOCK_PROCESSING_TARGET_MS: u32 = 5000; // <5 seconds for 100K chains
pub const PER_CHAIN_PROCESSING_TARGET_MS: u32 = 2; // <2ms per chain

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
    /// Prover's public key (32 bytes)
    pub public_key: Buffer,
    /// Initial blockchain block height (8 bytes)
    pub initial_block_height: f64,
    /// Initial blockchain block hash (32 bytes)
    pub initial_block_hash: Buffer,
    /// SHA256 of header fields (32 bytes)
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
/// Complete chain data structure
pub struct ChainData {
    /// Anchored commitment hash (hex)
    pub anchored_commitment: String,
    /// Initial blockchain block height
    pub initial_block_height: f64,
    /// Initial blockchain block hash (hex)
    pub initial_block_hash: String,
    /// Total number of chunks
    pub total_chunks: f64,
    /// Consensus algorithm version
    pub consensus_algorithm_version: u32,
    /// Chain length
    pub chain_length: u32,
    /// All commitments in the chain
    pub commitments: Vec<PhysicalAccessCommitment>,
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
    /// Complete chain data as structured object
    pub chain_data_json: Option<ChainData>,
}

// Core type aliases for hierarchical system
pub type ChainId = Vec<u8>;
pub type GroupId = String;
pub type RegionId = String;

/// Lightweight HashChain for hierarchical system
#[derive(Clone)]
pub struct LightweightHashChain {
    /// Chain identifier
    pub chain_id: ChainId,
    /// Owner's public key
    pub public_key: Buffer,
    /// Data file path
    pub data_file_path: String,
    /// Total chunks in file
    pub total_chunks: u64,
    /// Current commitment
    pub current_commitment: Option<Buffer>,
    /// Chain length
    pub chain_length: u32,
    /// Initial block info
    pub initial_block_height: u64,
    pub initial_block_hash: Buffer,
}

impl LightweightHashChain {
    pub fn get_chain_id(&self) -> ChainId {
        self.chain_id.clone()
    }
}

/// Ultra-compact proof for audits (exactly 136 bytes)
#[napi(object)]
#[derive(Clone)]
pub struct UltraCompactProof {
    /// Chain hash (32 bytes)
    pub chain_hash: Buffer,
    /// Chain length (8 bytes)
    pub chain_length: f64,
    /// Global proof reference (32 bytes)
    pub global_proof_reference: Buffer,
    /// Global block height (8 bytes)
    pub global_block_height: f64,
    /// Hierarchical position (32 bytes)
    pub hierarchical_position: Buffer,
    /// Total chains count (4 bytes)
    pub total_chains_count: u32,
    /// Proof timestamp (8 bytes)
    pub proof_timestamp: f64,
    /// Proof nonce (12 bytes)
    pub proof_nonce: Buffer,
}

impl UltraCompactProof {
    pub fn serialize(&self) -> Result<Buffer> {
        // Error and Status are already imported from napi::bindgen_prelude::*

        if self.chain_hash.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Chain hash must be 32 bytes".to_string(),
            ));
        }
        if self.global_proof_reference.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Global proof must be 32 bytes".to_string(),
            ));
        }
        if self.hierarchical_position.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Hierarchical position must be 32 bytes".to_string(),
            ));
        }
        if self.proof_nonce.len() != 12 {
            return Err(Error::new(
                Status::InvalidArg,
                "Proof nonce must be 12 bytes".to_string(),
            ));
        }

        let mut data = Vec::new();
        data.extend_from_slice(&self.chain_hash); // 32 bytes
        data.extend_from_slice(&(self.chain_length as u64).to_be_bytes()); // 8 bytes
        data.extend_from_slice(&self.global_proof_reference); // 32 bytes
        data.extend_from_slice(&(self.global_block_height as u64).to_be_bytes()); // 8 bytes
        data.extend_from_slice(&self.hierarchical_position); // 32 bytes
        data.extend_from_slice(&self.total_chains_count.to_be_bytes()); // 4 bytes
        data.extend_from_slice(&(self.proof_timestamp as u64).to_be_bytes()); // 8 bytes
        data.extend_from_slice(&self.proof_nonce); // 12 bytes

        if data.len() != 136 {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Proof must be exactly 136 bytes, got {}", data.len()),
            ));
        }

        Ok(Buffer::from(data))
    }
}

/// Performance metrics for monitoring
#[derive(Clone)]
pub struct PerformanceMetrics {
    /// Last block processing time
    pub last_block_time_ms: f64,
    /// Average chain processing time
    pub avg_chain_time_ms: f64,
    /// Hierarchical proof time
    pub hierarchical_proof_time_ms: f64,
    /// Total chains processed
    pub total_chains_processed: u32,
    /// Speedup factor achieved
    pub speedup_factor: f64,
}

/// Format B: Compact Proof (1.6 KB - Standard verification)
#[napi(object)]
#[derive(Clone)]
pub struct CompactProof {
    /// Chain hash (32 bytes)
    pub chain_hash: Buffer,
    /// Chain length (8 bytes)
    pub chain_length: f64,
    /// Last 8 commitments (8 * 136 = 1088 bytes)
    pub proof_window: Vec<PhysicalAccessCommitment>,
    /// Group proof hash (32 bytes)
    pub group_proof: Buffer,
    /// Regional proof hash (32 bytes)
    pub regional_proof: Buffer,
    /// Global proof reference (32 bytes)
    pub global_proof_reference: Buffer,
    /// Merkle path to group (up to 256 bytes)
    pub merkle_path: Vec<Buffer>,
    /// Proof metadata (64 bytes)
    pub metadata: ProofMetadata,
}

/// Format C: Full Proof (16 KB - Complete verification)
#[napi(object)]
#[derive(Clone)]
pub struct FullProof {
    /// Chain hash (32 bytes)
    pub chain_hash: Buffer,
    /// Complete chain data
    pub chain_data: ChainData,
    /// All group proofs in region (10 * 32 = 320 bytes)
    pub group_proofs: Vec<Buffer>,
    /// All regional proofs (10 * 32 = 320 bytes)
    pub regional_proofs: Vec<Buffer>,
    /// Global hierarchical proof (32 bytes)
    pub global_proof: Buffer,
    /// Complete merkle paths for verification (up to 8 KB)
    pub merkle_paths: Vec<Vec<Buffer>>,
    /// Chunk verification data (up to 4 KB)
    pub chunk_verification: ChunkVerificationData,
    /// Full metadata and statistics (up to 2 KB)
    pub full_metadata: FullProofMetadata,
}

/// Format D: Hierarchical Path Proof (200 bytes - Path validation)
#[napi(object)]
#[derive(Clone)]
pub struct HierarchicalPathProof {
    /// Chain ID (32 bytes)
    pub chain_id: Buffer,
    /// Group ID hash (32 bytes)
    pub group_id: Buffer,
    /// Region ID hash (32 bytes)
    pub region_id: Buffer,
    /// Path to global root (64 bytes)
    pub hierarchical_path: Buffer,
    /// Current position in hierarchy (8 bytes)
    pub position: HierarchicalPosition,
    /// Validation timestamp (8 bytes)
    pub timestamp: f64,
    /// Path verification nonce (16 bytes)
    pub verification_nonce: Buffer,
}

/// Proof metadata for compact proofs
#[napi(object)]
#[derive(Clone, Debug)]
pub struct ProofMetadata {
    /// Proof generation timestamp
    pub timestamp: f64,
    /// Total chains in system
    pub total_chains: u32,
    /// Algorithm version
    pub version: u32,
    /// Proof type identifier
    pub proof_type: String,
}

/// Chunk verification data for full proofs
#[napi(object)]
#[derive(Clone)]
pub struct ChunkVerificationData {
    /// Selected chunk indices
    pub selected_chunks: Vec<u32>,
    /// Chunk hashes
    pub chunk_hashes: Vec<Buffer>,
    /// Chunk merkle proofs
    pub chunk_proofs: Vec<Buffer>,
    /// File integrity hash
    pub file_hash: Buffer,
}

/// Full proof metadata with statistics
#[napi(object)]
#[derive(Clone, Debug)]
pub struct FullProofMetadata {
    /// Detailed system statistics
    pub system_stats: String, // JSON string
    /// Performance metrics
    pub performance_metrics: String, // JSON string
    /// Verification instructions
    pub verification_guide: String,
    /// Proof generation time
    pub generation_time_ms: f64,
}

/// Hierarchical position in the proof tree
#[napi(object)]
#[derive(Clone, Debug)]
pub struct HierarchicalPosition {
    /// Level in hierarchy (0-3)
    pub level: u32,
    /// Position at this level
    pub position: u32,
    /// Total items at this level
    pub total_at_level: u32,
    /// Parent position
    pub parent_position: Option<u32>,
}
