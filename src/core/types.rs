use napi::bindgen_prelude::*;
use napi_derive::napi;

// Production dependencies (currently unused but may be needed for future merkle implementations)
// use rs_merkle::{algorithms::Sha256 as MerkleSha256, MerkleProof, MerkleTree as RsMerkleTree};

// Enhanced Network Consensus Constants
pub const BLOCK_TIME_SECONDS: u32 = 52; // Blockchain average block time
pub const PROOF_WINDOW_MINUTES: u32 = 4; // Extended proof window
pub const PROOF_WINDOW_BLOCKS: u32 = 5; // (PROOF_WINDOW_MINUTES * 60) / BLOCK_TIME_SECONDS
pub const CHUNK_SIZE_BYTES: u32 = 4096; // 4KB chunks
pub const CHUNKS_PER_BLOCK: u32 = 16; // Enhanced to 16 chunks per block for anti-erasure coding resistance
pub const HASH_SIZE: usize = 32; // SHA256 output size

// Enhanced Security Constants
pub const MIN_FILE_SIZE: u64 = (CHUNKS_PER_BLOCK * CHUNK_SIZE_BYTES) as u64; // Minimum 16 chunks (64KB)
pub const MEMORY_HARD_VDF_MEMORY: usize = 256 * 1024 * 1024; // 256MB memory requirement
pub const MEMORY_HARD_ITERATIONS: u32 = 28_000_000; // Tuned for ~16 second compute to match block intervals
pub const CHUNK_SELECTION_VERSION: u32 = 2; // Updated version with enhanced security

// Chunk Selection Algorithm (Enhanced)
pub const CHUNK_SELECTION_SEED_SIZE: usize = 16; // Increased entropy
pub const CHUNK_SELECTION_MAX_ATTEMPTS: u32 = 32; // More attempts for unique chunks
pub const ENTROPY_SOURCES_COUNT: u32 = 3; // Multi-source entropy

// File Format Consensus Constants
pub const HASHCHAIN_MAGIC: &[u8] = b"HCH2"; // Updated magic for v2
pub const HASHCHAIN_FORMAT_VERSION: u32 = 2; // Enhanced format version
pub const HASHCHAIN_HEADER_SIZE: usize = 512; // Expanded header for new fields
pub const HASHCHAIN_MAX_CHUNKS: u64 = 1048576; // Max chunks per file (4TB max)
pub const HASHCHAIN_MIN_CHUNKS: u64 = 1; // Minimum 1 chunk (4KB)

// Hierarchical Temporal Proof Parameters (Enhanced)
pub const GLOBAL_ROOT_ITERATIONS: u32 = 20000; // Increased security
pub const REGIONAL_ITERATIONS: u32 = 10000; // Enhanced regional security
pub const GROUP_ITERATIONS: u32 = 2000; // Enhanced group security
pub const CHAINS_PER_GROUP: u32 = 1000; // Standard group size
pub const GROUPS_PER_REGION: u32 = 10; // Standard region size

// Availability Proof Constants
pub const AVAILABILITY_CHALLENGES_PER_BLOCK: u32 = 10;
pub const AVAILABILITY_RESPONSE_TIME_MS: u32 = 500; // 500ms response deadline
pub const AVAILABILITY_CHALLENGE_PROBABILITY: f64 = 0.1; // 10% of chains challenged per block

// Network Latency Proof Constants (Anti-outsourcing)
pub const NETWORK_LATENCY_SAMPLES: u32 = 5;
pub const NETWORK_LATENCY_MAX_MS: u32 = 100; // Maximum acceptable latency
pub const NETWORK_LATENCY_VARIANCE_MAX: f64 = 0.3; // Maximum variance in latency

// Economic Constants (Generic Token Units)
pub const CHECKPOINT_BOND_UNITS: u64 = 1000; // Bond amount in base token units
pub const AVAILABILITY_REWARD_UNITS: u64 = 1; // Reward for successful challenge
pub const CHAIN_REGISTRATION_UNITS: u64 = 100; // Registration deposit
pub const SLASHING_PENALTY_UNITS: u64 = 1000; // Penalty for invalid behavior

// Scalability Parameters
pub const MAX_CHAINS_PER_INSTANCE: u32 = 100000; // Support up to 100K chains
pub const GLOBAL_STATE_UPDATE_INTERVAL: u32 = 5; // Every 5 blocks for enhanced security
pub const REMOVAL_DELAY_BLOCKS: u32 = 20; // Delay before chain removal
pub const INACTIVE_CHAIN_TIMEOUT_BLOCKS: u32 = 2070; // ~30 days in blocks
pub const STATE_CLEANUP_INTERVAL: u32 = 69; // Periodic cleanup

// Performance Targets (Enhanced)
pub const BLOCK_PROCESSING_TARGET_MS: u32 = 40000; // 40 seconds for enhanced processing
pub const PER_CHAIN_PROCESSING_TARGET_MS: u32 = 5; // <5ms per chain with enhanced security

// Callback Interface Types

/// Generic blockchain interface for blockchain operations
#[napi(object)]
#[derive(Clone)]
pub struct BlockchainCallback {
    /// Function name to call for getting block data
    pub get_block_data: String,
    /// Function name to call for submitting checkpoints
    pub submit_checkpoint: String,
    /// Function name to call for getting current block height
    pub get_current_height: String,
}

/// Generic token interface for economic operations
#[napi(object)]
#[derive(Clone)]
pub struct TokenCallback {
    /// Function name to call for creating bonds
    pub create_bond: String,
    /// Function name to call for releasing bonds
    pub release_bond: String,
    /// Function name to call for slashing bonds
    pub slash_bond: String,
    /// Function name to call for paying rewards
    pub pay_reward: String,
}

/// Generic randomness beacon interface
#[napi(object)]
#[derive(Clone)]
pub struct BeaconCallback {
    /// Function name to call for getting beacon randomness
    pub get_randomness: String,
    /// Function name to call for verifying beacon signature
    pub verify_randomness: String,
}

/// Generic network interface for latency proofs
#[napi(object)]
#[derive(Clone)]
pub struct NetworkCallback {
    /// Function name to call for measuring network latency
    pub measure_latency: String,
    /// Function name to call for verifying peer location
    pub verify_peer_location: String,
    /// Function name to call for getting network peers
    pub get_network_peers: String,
}

/// Multi-source entropy combining blockchain, beacon, and local sources
#[napi(object)]
#[derive(Clone)]
pub struct MultiSourceEntropy {
    /// Blockchain block hash entropy
    pub blockchain_entropy: Buffer,
    /// External beacon entropy (optional)
    pub beacon_entropy: Option<Buffer>,
    /// Local randomness
    pub local_entropy: Buffer,
    /// Timestamp when entropy was collected
    pub timestamp: f64,
    /// Combined entropy hash
    pub combined_hash: Buffer,
}

/// Memory-hard VDF proof structure
#[napi(object)]
#[derive(Clone)]
pub struct MemoryHardVDFProof {
    /// Input state to VDF
    pub input_state: Buffer,
    /// Output state from VDF
    pub output_state: Buffer,
    /// Number of iterations performed
    pub iterations: u32,
    /// Sample of memory access pattern (for verification)
    pub memory_access_samples: Vec<MemoryAccessSample>,
    /// Total computation time in milliseconds
    pub computation_time_ms: f64,
    /// Memory usage in bytes
    pub memory_usage_bytes: f64,
}

/// Sample of memory access for VDF verification
#[napi(object)]
#[derive(Clone)]
pub struct MemoryAccessSample {
    /// Iteration number when sample was taken
    pub iteration: u32,
    /// Memory read address
    pub read_address: f64,
    /// Memory write address
    pub write_address: f64,
    /// Hash of memory content at read address
    pub memory_content_hash: Buffer,
}

/// Availability challenge structure
#[napi(object)]
#[derive(Clone)]
pub struct AvailabilityChallenge {
    /// Chain being challenged
    pub chain_id: Buffer,
    /// Chunk index to retrieve
    pub chunk_index: u32,
    /// Challenge nonce
    pub challenge_nonce: Buffer,
    /// Challenger identifier
    pub challenger_id: Buffer,
    /// Challenge timestamp
    pub challenge_time: f64,
    /// Response deadline
    pub deadline: f64,
    /// Reward amount for successful challenge
    pub reward_amount: f64,
}

/// Response to availability challenge
#[napi(object)]
#[derive(Clone)]
pub struct AvailabilityResponse {
    /// Challenge being responded to
    pub challenge_id: Buffer,
    /// Actual chunk data
    pub chunk_data: Buffer,
    /// Response timestamp
    pub response_time: f64,
    /// Proof of chunk authenticity
    pub authenticity_proof: Buffer,
}

/// Network latency proof for anti-outsourcing
#[napi(object)]
#[derive(Clone)]
pub struct NetworkLatencyProof {
    /// List of peer latency measurements
    pub peer_latencies: Vec<PeerLatencyMeasurement>,
    /// Average latency across peers
    pub average_latency_ms: f64,
    /// Latency variance
    pub latency_variance: f64,
    /// Timestamp of measurements
    pub measurement_time: f64,
    /// Geographic location proof (optional)
    pub location_proof: Option<Buffer>,
}

/// Individual peer latency measurement
#[napi(object)]
#[derive(Clone)]
pub struct PeerLatencyMeasurement {
    /// Peer identifier
    pub peer_id: Buffer,
    /// Round-trip latency in milliseconds
    pub latency_ms: f64,
    /// Number of samples taken
    pub sample_count: u32,
    /// Measurement timestamp
    pub timestamp: f64,
}

/// Enhanced ownership commitment with prover-specific encoding
#[napi(object)]
#[derive(Clone)]
pub struct EnhancedOwnershipCommitment {
    /// Prover's public key (32 bytes)
    pub public_key: Buffer,
    /// SHA256 hash of the encoded data (32 bytes)  
    pub encoded_data_hash: Buffer,
    /// SHA256 hash of the original data (32 bytes)
    pub original_data_hash: Buffer,
    /// Encoding parameters used
    pub encoding_params: Buffer,
    /// Enhanced commitment hash
    pub commitment_hash: Buffer,
}

/// Enhanced physical access commitment with memory-hard VDF
#[napi(object)]
#[derive(Clone)]
pub struct EnhancedPhysicalAccessCommitment {
    /// Blockchain block height
    pub block_height: f64,
    /// Previous commitment in chain (32 bytes)
    pub previous_commitment: Buffer,
    /// Current block hash (32 bytes)
    pub block_hash: Buffer,
    /// Multi-source entropy used
    pub entropy: MultiSourceEntropy,
    /// Enhanced chunk selection (16 chunks)
    pub selected_chunks: Vec<u32>,
    /// SHA256 hashes of selected chunks
    pub chunk_hashes: Vec<Buffer>,
    /// Memory-hard VDF proof
    pub vdf_proof: MemoryHardVDFProof,
    /// Availability challenge responses
    pub availability_responses: Vec<AvailabilityResponse>,
    /// Network latency proof
    pub network_latency_proof: NetworkLatencyProof,
    /// Enhanced commitment hash
    pub commitment_hash: Buffer,
}

/// Enhanced chunk selection result with multi-source entropy
#[napi(object)]
#[derive(Clone)]
pub struct EnhancedChunkSelectionResult {
    /// Selected chunk indices (16 chunks)
    pub selected_indices: Vec<u32>,
    /// Algorithm version used
    pub algorithm_version: u32,
    /// Total chunks in file
    pub total_chunks: f64,
    /// Multi-source entropy used
    pub entropy: MultiSourceEntropy,
    /// Hash of selection parameters for verification
    pub verification_hash: Buffer,
    /// Proof that selection is unpredictable
    pub unpredictability_proof: Buffer,
}

/// Bond information for economic security
#[napi(object)]
#[derive(Clone)]
pub struct BondInfo {
    /// Bond identifier
    pub bond_id: Buffer,
    /// Amount bonded in base token units
    pub amount: f64,
    /// Bond holder's identifier
    pub holder_id: Buffer,
    /// Block height when bond was created
    pub creation_height: f64,
    /// Block height when bond can be released
    pub release_height: f64,
    /// Bond purpose/type
    pub bond_type: String,
}

/// Checkpoint data with enhanced security
#[napi(object)]
#[derive(Clone)]
pub struct EnhancedCheckpoint {
    /// Checkpoint hash (32 bytes)
    pub checkpoint_hash: Buffer,
    /// Block height of checkpoint
    pub block_height: f64,
    /// Global hierarchical root
    pub global_root: Buffer,
    /// Number of active chains
    pub chain_count: u32,
    /// Cumulative work proof
    pub cumulative_work: Buffer,
    /// Bond information
    pub bond_info: BondInfo,
    /// Enhanced security proofs
    pub security_proofs: Vec<Buffer>,
    /// Submitter identifier
    pub submitter_id: Buffer,
}

/// File encoding information for prover-specific storage
#[napi(object)]
#[derive(Clone)]
pub struct FileEncodingInfo {
    /// Original file hash
    pub original_hash: Buffer,
    /// Encoded file hash
    pub encoded_hash: Buffer,
    /// Prover's public key used for encoding
    pub prover_key: Buffer,
    /// Encoding algorithm version
    pub encoding_version: u32,
    /// Additional encoding parameters
    pub encoding_params: Buffer,
}

/// Enhanced chain metadata with new security features
#[napi(object)]
#[derive(Clone)]
pub struct EnhancedChainMetadata {
    /// Chain identifier
    pub chain_id: Buffer,
    /// Prover's public key
    pub prover_key: Buffer,
    /// File encoding information
    pub file_encoding: FileEncodingInfo,
    /// Registration bond information
    pub registration_bond: BondInfo,
    /// Last activity block height
    pub last_activity_height: f64,
    /// Availability challenge history
    pub availability_score: f64,
    /// Network latency history
    pub latency_score: f64,
    /// Chain status
    pub status: String,
}

// Legacy types maintained for compatibility
#[napi(object)]
#[derive(Clone)]
/// Original ownership commitment binding data to a public key
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
    /// Block height from blockchain
    pub block_height: f64,
    /// Block hash from blockchain (32 bytes)
    pub block_hash: Buffer,
    /// Optional timestamp
    pub timestamp: Option<f64>,
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
/// Original physical access commitment proving data access at specific block
pub struct PhysicalAccessCommitment {
    /// Blockchain block height
    pub block_height: f64,
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
    /// File format identifier b'HCH2'
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
/// Proof window containing last 5 commitments for verification (updated)
pub struct ProofWindow {
    /// Last 5 commitments (updated from 8)
    pub commitments: Vec<PhysicalAccessCommitment>,
    /// Merkle proofs for selected chunks
    pub merkle_proofs: Vec<Buffer>, // Simplified - would be proper MerkleProof objects
    /// Commitment from 5 blocks ago
    pub start_commitment: Buffer,
    /// Latest commitment
    pub end_commitment: Buffer,
}

#[napi(object)]
#[derive(Clone)]
/// Original result of chunk selection with verification data
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
    /// Whether proof window is ready (5+ blocks)
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

/// Type aliases for enhanced clarity and future maintenance
pub type ChainId = Vec<u8>;
pub type GroupId = String;
pub type RegionId = String;
pub type TokenAmount = u64;
pub type BlockHeight = u64;

/// Lightweight chain representation for hierarchical management
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
    /// File encoding information
    pub file_encoding: Option<FileEncodingInfo>,
    /// Last availability challenge score
    pub availability_score: f64,
    /// Last network latency score
    pub latency_score: f64,
}

impl LightweightHashChain {
    pub fn get_chain_id(&self) -> ChainId {
        self.chain_id.clone()
    }
}

/// Ultra-compact proof for audits (exactly 136 bytes) - Enhanced
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
        let mut data = Vec::new();

        // Chain hash (32 bytes)
        data.extend_from_slice(self.chain_hash.as_ref());

        // Chain length (8 bytes)
        data.extend_from_slice(&self.chain_length.to_le_bytes());

        // Global proof reference (32 bytes)
        data.extend_from_slice(self.global_proof_reference.as_ref());

        // Global block height (8 bytes)
        data.extend_from_slice(&self.global_block_height.to_le_bytes());

        // Hierarchical position (32 bytes)
        data.extend_from_slice(self.hierarchical_position.as_ref());

        // Total chains count (4 bytes)
        data.extend_from_slice(&self.total_chains_count.to_le_bytes());

        // Proof timestamp (8 bytes)
        data.extend_from_slice(&self.proof_timestamp.to_le_bytes());

        // Proof nonce (12 bytes)
        data.extend_from_slice(self.proof_nonce.as_ref());

        // Verify exactly 136 bytes
        if data.len() != 136 {
            return Err(Error::new(
                Status::GenericFailure,
                format!("Invalid proof size: {} bytes (expected 136)", data.len()),
            ));
        }

        Ok(Buffer::from(data))
    }

    pub fn deserialize(data: Buffer) -> Result<Self> {
        if data.len() != 136 {
            return Err(Error::new(
                Status::GenericFailure,
                format!("Invalid proof size: {} bytes (expected 136)", data.len()),
            ));
        }

        let data = data.as_ref();
        let mut offset = 0;

        // Chain hash (32 bytes)
        let chain_hash = Buffer::from(data[offset..offset + 32].to_vec());
        offset += 32;

        // Chain length (8 bytes)
        let chain_length = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        // Global proof reference (32 bytes)
        let global_proof_reference = Buffer::from(data[offset..offset + 32].to_vec());
        offset += 32;

        // Global block height (8 bytes)
        let global_block_height = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        // Hierarchical position (32 bytes)
        let hierarchical_position = Buffer::from(data[offset..offset + 32].to_vec());
        offset += 32;

        // Total chains count (4 bytes)
        let total_chains_count = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        // Proof timestamp (8 bytes)
        let proof_timestamp = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        // Proof nonce (12 bytes)
        let proof_nonce = Buffer::from(data[offset..offset + 12].to_vec());

        Ok(Self {
            chain_hash,
            chain_length,
            global_proof_reference,
            global_block_height,
            hierarchical_position,
            total_chains_count,
            proof_timestamp,
            proof_nonce,
        })
    }
}

/// Performance metrics for tracking system efficiency
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceMetrics {
    /// Last block processing time
    pub last_block_time_ms: f64,
    /// Average chain processing time
    pub avg_chain_time_ms: f64,
    /// Hierarchical proof time
    pub hierarchical_proof_time_ms: f64,
    /// Memory-hard VDF time
    pub vdf_time_ms: f64,
    /// Availability challenge response time
    pub availability_response_time_ms: f64,
    /// Total chains processed
    pub total_chains_processed: u32,
    /// Speedup factor achieved
    pub speedup_factor: f64,
}

/// Format B: Compact Proof (Enhanced - ~2KB)
#[napi(object)]
#[derive(Clone)]
pub struct CompactProof {
    /// Chain hash (32 bytes)
    pub chain_hash: Buffer,
    /// Chain length (8 bytes)
    pub chain_length: f64,
    /// Last 5 commitments (5 * ~200 = 1000 bytes)
    pub proof_window: Vec<PhysicalAccessCommitment>,
    /// Group proof hash (32 bytes)
    pub group_proof: Buffer,
    /// Regional proof hash (32 bytes)
    pub regional_proof: Buffer,
    /// Global proof reference (32 bytes)
    pub global_proof_reference: Buffer,
    /// Merkle path to group (up to 512 bytes)
    pub merkle_path: Vec<Buffer>,
    /// Enhanced proof metadata (128 bytes)
    pub metadata: ProofMetadata,
}

/// Format C: Full Proof (Enhanced - ~32KB)
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
    /// Complete merkle paths for verification (up to 16 KB)
    pub merkle_paths: Vec<Vec<Buffer>>,
    /// Chunk verification data with enhanced security (up to 8 KB)
    pub chunk_verification: ChunkVerificationData,
    /// Full metadata and statistics (up to 4 KB)
    pub full_metadata: FullProofMetadata,
}

/// Format D: Hierarchical Path Proof (Enhanced - 256 bytes)
#[napi(object)]
#[derive(Clone)]
pub struct HierarchicalPathProof {
    /// Chain ID (32 bytes)
    pub chain_id: Buffer,
    /// Group ID hash (32 bytes)
    pub group_id: Buffer,
    /// Region ID hash (32 bytes)
    pub region_id: Buffer,
    /// Path to global root (96 bytes)
    pub hierarchical_path: Buffer,
    /// Current position in hierarchy (32 bytes)
    pub position: HierarchicalPosition,
    /// Validation timestamp (8 bytes)
    pub timestamp: f64,
    /// Path verification nonce (16 bytes)
    pub verification_nonce: Buffer,
    /// Enhanced security proof (8 bytes)
    pub security_proof: Buffer,
}

/// Enhanced proof metadata
#[napi(object)]
#[derive(Clone)]
pub struct ProofMetadata {
    /// Proof generation timestamp
    pub timestamp: f64,
    /// Total chains in system
    pub total_chains: u32,
    /// Algorithm version (v2 enhanced)
    pub version: u32,
    /// Proof type identifier
    pub proof_type: String,
    /// Memory-hard VDF metadata
    pub vdf_metadata: Option<String>,
    /// Availability challenge count
    pub availability_challenges: u32,
}

/// Enhanced chunk verification data
#[napi(object)]
#[derive(Clone)]
pub struct ChunkVerificationData {
    /// Selected chunk indices (16 chunks)
    pub selected_chunks: Vec<u32>,
    /// Chunk hashes
    pub chunk_hashes: Vec<Buffer>,
    /// Chunk merkle proofs
    pub chunk_proofs: Vec<Buffer>,
    /// File integrity hash
    pub file_hash: Buffer,
    /// Prover-specific encoding proof
    pub encoding_proof: Buffer,
    /// Memory-hard VDF proof for chunk access
    pub vdf_proof: Option<MemoryHardVDFProof>,
}

/// Enhanced full proof metadata
#[napi(object)]
#[derive(Clone)]
pub struct FullProofMetadata {
    /// Detailed system statistics
    pub system_stats: String, // JSON string
    /// Performance metrics including VDF
    pub performance_metrics: String, // JSON string
    /// Verification instructions
    pub verification_guide: String,
    /// Proof generation time
    pub generation_time_ms: f64,
    /// Memory usage during generation
    pub memory_usage_mb: f64,
    /// Enhanced security features used
    pub security_features: Vec<String>,
}

/// Enhanced hierarchical position
#[napi(object)]
#[derive(Clone)]
pub struct HierarchicalPosition {
    /// Level in hierarchy (0-3)
    pub level: u32,
    /// Position at this level
    pub position: u32,
    /// Total items at this level
    pub total_at_level: u32,
    /// Parent position
    pub parent_position: Option<u32>,
    /// Enhanced security score
    pub security_score: Option<f64>,
}

// ====================================================================
// NEW PROVER/VERIFIER INTERFACE TYPES
// ====================================================================

/// Storage commitment proving data possession for new interface
#[napi(object)]
#[derive(Clone)]
pub struct StorageCommitment {
    /// Prover's public key
    pub prover_key: Buffer,
    /// Data file hash
    pub data_hash: Buffer,
    /// Block height when committed
    pub block_height: u32,
    /// Block hash for entropy
    pub block_hash: Buffer,
    /// Selected chunk indices
    pub selected_chunks: Vec<u32>,
    /// Hashes of selected chunks
    pub chunk_hashes: Vec<Buffer>,
    /// Memory-hard VDF proof
    pub vdf_proof: MemoryHardVDFProof,
    /// Multi-source entropy used
    pub entropy: MultiSourceEntropy,
    /// Commitment hash
    pub commitment_hash: Buffer,
}

/// Challenge issued to prover for data availability
#[napi(object)]
#[derive(Clone)]
pub struct StorageChallenge {
    /// Challenge identifier
    pub challenge_id: Buffer,
    /// Target prover
    pub prover_key: Buffer,
    /// Data commitment being challenged
    pub commitment_hash: Buffer,
    /// Specific chunk indices to prove
    pub challenged_chunks: Vec<u32>,
    /// Challenge nonce for uniqueness
    pub nonce: Buffer,
    /// Challenge timestamp
    pub timestamp: f64,
    /// Response deadline
    pub deadline: f64,
}

/// Proof response to storage challenge
#[napi(object)]
#[derive(Clone)]
pub struct ChallengeResponse {
    /// Challenge being responded to
    pub challenge_id: Buffer,
    /// Actual chunk data
    pub chunk_data: Vec<Buffer>,
    /// Merkle proofs for chunks
    pub merkle_proofs: Vec<Buffer>,
    /// Response timestamp
    pub timestamp: f64,
    /// VDF proof of timely access
    pub access_proof: MemoryHardVDFProof,
}

/// Compact proof for efficient verification
#[napi(object)]
#[derive(Clone)]
pub struct CompactStorageProof {
    /// Prover identification
    pub prover_key: Buffer,
    /// Data commitment hash
    pub commitment_hash: Buffer,
    /// Block height reference
    pub block_height: u32,
    /// Sampled chunk proofs (subset)
    pub chunk_proofs: Vec<Buffer>,
    /// Aggregated VDF proof
    pub vdf_proof: MemoryHardVDFProof,
    /// Network position in hierarchy
    pub network_position: Buffer,
    /// Proof generation timestamp
    pub timestamp: f64,
}

/// Full verification proof with complete data
#[napi(object)]
#[derive(Clone)]
pub struct FullStorageProof {
    /// Prover identification
    pub prover_key: Buffer,
    /// Complete storage commitment
    pub commitment: StorageCommitment,
    /// All chunk hashes
    pub all_chunk_hashes: Vec<Buffer>,
    /// Complete merkle tree
    pub merkle_tree: Vec<Buffer>,
    /// Full VDF computation chain
    pub vdf_chain: Vec<MemoryHardVDFProof>,
    /// Network consensus proofs
    pub network_proofs: Vec<Buffer>,
    /// Metadata and statistics
    pub metadata: ProofMetadata,
}

/// Network node information
#[napi(object)]
#[derive(Clone)]
pub struct NetworkNode {
    /// Node public key
    pub node_key: Buffer,
    /// Node type: "prover" | "verifier" | "both"
    pub node_type: String,
    /// Node reputation score
    pub reputation: f64,
    /// Last activity timestamp
    pub last_activity: f64,
    /// Network position
    pub position: Buffer,
}

/// Network statistics
#[napi(object)]
#[derive(Clone)]
pub struct NetworkStats {
    /// Total active provers
    pub total_provers: u32,
    /// Total active verifiers
    pub total_verifiers: u32,
    /// Network health score
    pub health_score: f64,
    /// Total storage committed
    pub total_storage: f64,
    /// Challenge success rate
    pub challenge_success_rate: f64,
}
