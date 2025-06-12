use napi::bindgen_prelude::*;
use thiserror::Error;

/// Comprehensive error handling for HashChain system
#[derive(Error, Debug)]
pub enum HashChainError {
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid public key: expected 32 bytes, got {0}")]
    InvalidPublicKeySize(usize),

    #[error("Invalid block hash: expected 32 bytes, got {0}")]
    InvalidBlockHashSize(usize),

    #[error("Invalid block height: {0} (must be non-negative)")]
    InvalidBlockHeight(f64),

    #[error("Chunk index {index} out of range [0, {max})")]
    ChunkIndexOutOfRange { index: u32, max: u64 },

    #[error("Too many chunks: {count} > {max}")]
    TooManyChunks { count: u64, max: u64 },

    #[error("Too few chunks: {count} < {min}")]
    TooFewChunks { count: u64, min: u64 },

    #[error("HashChain already has data - create new instance")]
    AlreadyHasData,

    #[error("No data has been streamed - call stream_data() first")]
    NoDataStreamed,

    #[error("Chain length {0} insufficient for proof window (need 8+ blocks)")]
    InsufficientChainLength(u32),

    #[error("Chain too short: {length} blocks, required {required}")]
    ChainTooShort { length: u32, required: u32 },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("File format error: {0}")]
    FileFormat(String),

    #[error("Corrupted file: {0}")]
    Corruption(String),

    #[error("Merkle tree error: {0}")]
    MerkleTree(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Verification failed: {reason}")]
    VerificationFailed { reason: String },

    // Hierarchical system specific errors
    #[error("Chain not found: {chain_id}")]
    ChainNotFound { chain_id: String },

    #[error("Group {group_id} is full (max: {max_chains})")]
    GroupFull { group_id: String, max_chains: u32 },

    #[error("Region {region_id} is full (max: {max_groups})")]
    RegionFull { region_id: String, max_groups: u32 },

    #[error("Hierarchical proof computation failed: {reason}")]
    HierarchicalProofFailed { reason: String },

    #[error("Chain lifecycle error: {reason}")]
    ChainLifecycle { reason: String },

    #[error("Performance target missed: {operation} took {actual_ms}ms (target: {target_ms}ms)")]
    PerformanceTarget {
        operation: String,
        actual_ms: u32,
        target_ms: u32,
    },

    #[error("Scale limit exceeded: {count} > {limit}")]
    ScaleLimit { count: u32, limit: u32 },

    #[error("Retention policy violation: {reason}")]
    RetentionPolicy { reason: String },

    #[error("Audit failed: {reason}")]
    AuditFailed { reason: String },

    #[error("Compact proof error: {reason}")]
    CompactProof { reason: String },

    #[error("Parallel processing error: {reason}")]
    ParallelProcessing { reason: String },

    #[error("Group assignment failed: {reason}")]
    GroupAssignment { reason: String },

    #[error("Chain registry error: {reason}")]
    ChainRegistry { reason: String },

    #[error("Global state error: {reason}")]
    GlobalState { reason: String },

    // Cryptographic errors for production implementation
    #[error("Invalid private key: expected 32 bytes, got {0}")]
    InvalidPrivateKeySize(usize),

    #[error("Invalid signature: expected 64 bytes, got {0}")]
    InvalidSignatureSize(usize),

    #[error("Cryptographic operation failed: {0}")]
    CryptographicError(String),

    #[error("VDF verification failed: {reason}")]
    VDFVerificationFailed { reason: String },

    #[error("Entropy generation failed: {reason}")]
    EntropyGenerationFailed { reason: String },

    #[error("Key derivation failed: {reason}")]
    KeyDerivationFailed { reason: String },

    #[error("Invalid proof parameters: {reason}")]
    InvalidProofParameters { reason: String },
}

/// Convert to NAPI error for JavaScript
impl From<HashChainError> for napi::Error {
    fn from(err: HashChainError) -> Self {
        match err {
            HashChainError::InvalidPublicKeySize(_)
            | HashChainError::InvalidBlockHashSize(_)
            | HashChainError::InvalidBlockHeight(_)
            | HashChainError::ChunkIndexOutOfRange { .. }
            | HashChainError::TooManyChunks { .. }
            | HashChainError::TooFewChunks { .. } => {
                napi::Error::new(napi::Status::InvalidArg, err.to_string())
            }

            HashChainError::AlreadyHasData
            | HashChainError::NoDataStreamed
            | HashChainError::InsufficientChainLength(_)
            | HashChainError::ChainTooShort { .. } => {
                napi::Error::new(napi::Status::InvalidArg, err.to_string())
            }

            HashChainError::Io(_) => {
                napi::Error::new(napi::Status::GenericFailure, err.to_string())
            }

            HashChainError::FileNotFound { .. } => {
                napi::Error::new(napi::Status::GenericFailure, err.to_string())
            }

            HashChainError::FileFormat(_) | HashChainError::Corruption(_) => {
                napi::Error::new(napi::Status::InvalidArg, err.to_string())
            }

            HashChainError::ChainNotFound { .. }
            | HashChainError::GroupFull { .. }
            | HashChainError::RegionFull { .. } => {
                napi::Error::new(napi::Status::InvalidArg, err.to_string())
            }

            HashChainError::PerformanceTarget { .. } | HashChainError::ScaleLimit { .. } => {
                napi::Error::new(napi::Status::GenericFailure, err.to_string())
            }

            HashChainError::AuditFailed { .. } | HashChainError::CompactProof { .. } => {
                napi::Error::new(napi::Status::InvalidArg, err.to_string())
            }

            _ => napi::Error::new(napi::Status::GenericFailure, err.to_string()),
        }
    }
}

/// Helper type alias for Results
pub type HashChainResult<T> = std::result::Result<T, HashChainError>;

/// Result wrapper for hierarchical operations
pub type HierarchicalResult<T> = std::result::Result<T, HashChainError>;

/// Performance monitoring helpers
pub fn check_performance_target(
    operation: &str,
    actual_ms: u32,
    target_ms: u32,
) -> HashChainResult<()> {
    if actual_ms > target_ms {
        Err(HashChainError::PerformanceTarget {
            operation: operation.to_string(),
            actual_ms,
            target_ms,
        })
    } else {
        Ok(())
    }
}

/// Scale checking helpers
pub fn check_scale_limit(count: u32, limit: u32, _entity: &str) -> HashChainResult<()> {
    if count > limit {
        Err(HashChainError::ScaleLimit { count, limit })
    } else {
        Ok(())
    }
}
