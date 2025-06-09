use napi::bindgen_prelude::*;
use napi::Result;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::task;

#[macro_use]
extern crate napi_derive;

/// Bitcoin's maximum target (difficulty 1) - this is the easiest possible target
/// Using a more reasonable target for demonstration purposes
const MAX_TARGET: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

#[napi(object)]
#[derive(Clone)]
/// Result of a proof of work computation
pub struct ProofOfWorkResult {
    /// The nonce that was found
    pub nonce: BigInt,
    /// The resulting hash as hex string
    pub hash: String,
    /// Number of attempts made
    pub attempts: BigInt,
    /// Time taken in milliseconds
    #[napi(js_name = "time_ms")]
    pub time_ms: u32,
    /// The difficulty that was satisfied
    pub difficulty: f64,
    /// The target that was used (as hex string)
    pub target: String,
}

#[napi(object)]
/// Progress information for proof of work computation
pub struct ProofOfWorkProgress {
    /// Current number of attempts
    pub attempts: BigInt,
    /// Current nonce being tested
    pub nonce: BigInt,
    /// Time elapsed so far in milliseconds
    #[napi(js_name = "elapsed_ms")]
    pub elapsed_ms: u32,
    /// Estimated attempts per second
    #[napi(js_name = "attempts_per_second")]
    pub attempts_per_second: f64,
}

#[napi]
/// Handle for cancelling a proof of work computation
pub struct ProofOfWorkHandle {
    cancelled: Arc<AtomicBool>,
    progress_counter: Arc<AtomicU64>,
    result: Arc<std::sync::Mutex<Option<ProofOfWorkResult>>>,
    error: Arc<std::sync::Mutex<Option<String>>>,
    difficulty: f64,
}

#[napi]
impl ProofOfWorkHandle {
    /// Cancel the proof of work computation
    #[napi]
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Check if the computation has been cancelled
    #[napi]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Get the current number of attempts (approximate)
    #[napi]
    pub fn get_attempts(&self) -> BigInt {
        BigInt::from(self.progress_counter.load(Ordering::Relaxed))
    }

    /// Check if the computation has completed (found solution)
    #[napi]
    pub fn is_completed(&self) -> bool {
        self.progress_counter.load(Ordering::Relaxed) == u64::MAX
    }

    /// Check if there was an error (cancelled or max attempts reached)
    #[napi]
    pub fn has_error(&self) -> bool {
        if let Ok(error_lock) = self.error.lock() {
            error_lock.is_some()
        } else {
            false
        }
    }

    /// Get the error message if there was an error
    #[napi]
    pub fn get_error(&self) -> Option<String> {
        if let Ok(error_lock) = self.error.lock() {
            error_lock.clone()
        } else {
            None
        }
    }

    /// Get the result if the computation completed successfully
    #[napi]
    pub fn get_result(&self) -> Option<ProofOfWorkResult> {
        if let Ok(result_lock) = self.result.lock() {
            result_lock.clone()
        } else {
            None
        }
    }

    /// Get progress information
    #[napi]
    pub fn get_progress(&self) -> ProofOfWorkProgress {
        let attempts = self.progress_counter.load(Ordering::Relaxed);
        ProofOfWorkProgress {
            attempts: BigInt::from(if attempts == u64::MAX { 0u64 } else { attempts }),
            nonce: BigInt::from(0u64), // We can't easily track current nonce from outside
            elapsed_ms: 0,             // We can't easily track time from outside
            attempts_per_second: 0.0,
        }
    }

    /// Get the difficulty level for this computation
    #[napi]
    pub fn get_difficulty(&self) -> f64 {
        self.difficulty
    }
}

#[napi]
/// Computes proof of work asynchronously using Bitcoin's target-based system.
/// This function returns a handle for cancellation and unlimited attempts by default.
///
/// @param {Buffer} entropy_seed - The entropy seed (plotId) to bind the work to
/// @param {number} difficulty - The difficulty level (Bitcoin-style, where 1.0 is easiest)
/// @param {number} max_attempts - Maximum number of attempts before giving up (default: unlimited)
/// @param {boolean} log_attempts - Whether to log each hash attempt (default: false)
/// @param {boolean} double_sha - Whether to use double SHA-256 like Bitcoin (default: true)
/// @returns {ProofOfWorkHandle} Handle for cancellation and progress tracking
pub fn compute_proof_of_work_async(
    entropy_seed: Buffer,
    difficulty: f64,
    max_attempts: Option<u32>,
    log_attempts: Option<bool>,
    double_sha: Option<bool>,
) -> Result<ProofOfWorkHandle> {
    let max_attempts = max_attempts.map(|x| x as u64).unwrap_or(u64::MAX); // Default to unlimited
    let log_attempts = log_attempts.unwrap_or(false);
    let double_sha = double_sha.unwrap_or(true);

    if difficulty <= 0.0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Difficulty must be greater than 0".to_string(),
        ));
    }

    let entropy_seed_vec = entropy_seed.to_vec();
    let cancelled = Arc::new(AtomicBool::new(false));
    let progress_counter = Arc::new(AtomicU64::new(0));

    // Create the handle
    let handle = ProofOfWorkHandle {
        cancelled: cancelled.clone(),
        progress_counter: progress_counter.clone(),
        result: Arc::new(std::sync::Mutex::new(None)),
        error: Arc::new(std::sync::Mutex::new(None)),
        difficulty,
    };

    // Spawn the computation task
    let _computation_task = {
        let cancelled = cancelled.clone();
        let progress_counter = progress_counter.clone();
        let result_mutex = handle.result.clone();
        let error_mutex = handle.error.clone();

        task::spawn_blocking(move || {
            let target = difficulty_to_target(difficulty);
            let start_time = Instant::now();
            let mut attempts = 0u64;
            let mut nonce = 0u64;

            while attempts < max_attempts && !cancelled.load(Ordering::Relaxed) {
                // Combine entropy_seed and nonce
                let nonce_bytes = nonce.to_le_bytes();
                let mut data = Vec::new();
                data.extend_from_slice(&entropy_seed_vec);
                data.extend_from_slice(&nonce_bytes);

                // Compute hash (single or double SHA-256)
                let hash = if double_sha {
                    compute_double_sha256(&data)
                } else {
                    compute_sha256(&data)
                };
                let hash_hex = hex::encode(&hash);

                // Log attempt if requested
                if log_attempts {
                    println!(
                        "Attempt {}: nonce={}, hash={}, meets_target={}",
                        attempts + 1,
                        nonce,
                        hash_hex,
                        meets_bitcoin_target(&hash, &target)
                    );
                }

                // Check if hash meets Bitcoin target
                if meets_bitcoin_target(&hash, &target) {
                    let elapsed = start_time.elapsed();
                    let result = ProofOfWorkResult {
                        nonce: BigInt::from(nonce),
                        hash: hash_hex,
                        attempts: BigInt::from(attempts + 1),
                        time_ms: elapsed.as_millis() as u32,
                        difficulty,
                        target: hex::encode(&target),
                    };

                    // Store the result
                    if let Ok(mut result_lock) = result_mutex.lock() {
                        *result_lock = Some(result);
                    }

                    // Mark completion in progress counter
                    progress_counter.store(u64::MAX, Ordering::Relaxed);
                    return;
                }

                nonce += 1;
                attempts += 1;

                // Update progress counter every 1000 attempts
                if attempts % 1000 == 0 {
                    progress_counter.store(attempts, Ordering::Relaxed);
                }
            }

            // If we get here, we either hit max attempts or were cancelled
            if cancelled.load(Ordering::Relaxed) {
                if let Ok(mut error_lock) = error_mutex.lock() {
                    *error_lock = Some("Computation was cancelled".to_string());
                }
            } else {
                if let Ok(mut error_lock) = error_mutex.lock() {
                    *error_lock = Some(format!(
                        "Failed to find solution after {} attempts",
                        attempts
                    ));
                }
            }
        })
    };

    Ok(handle)
}

#[napi]
/// Verifies that a nonce produces a hash that meets the Bitcoin difficulty target.
///
/// @param {Buffer} entropy_seed - The entropy seed that was used
/// @param {number} nonce - The nonce to verify
/// @param {number} difficulty - The required difficulty level (Bitcoin-style)
/// @param {boolean} double_sha - Whether to use double SHA-256 like Bitcoin (default: true)
/// @returns {boolean} True if the nonce is valid for the given difficulty
pub fn verify_proof_of_work(
    entropy_seed: Buffer,
    nonce: u32,
    difficulty: f64,
    double_sha: Option<bool>,
) -> Result<bool> {
    let double_sha = double_sha.unwrap_or(true);
    let nonce_val = nonce as u64;

    if difficulty <= 0.0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Difficulty must be greater than 0".to_string(),
        ));
    }

    let target = difficulty_to_target(difficulty);

    // Combine entropy_seed and nonce
    let nonce_bytes = nonce_val.to_le_bytes();
    let mut data = Vec::new();
    data.extend_from_slice(&entropy_seed);
    data.extend_from_slice(&nonce_bytes);

    // Compute hash (single or double SHA-256)
    let hash = if double_sha {
        compute_double_sha256(&data)
    } else {
        compute_sha256(&data)
    };

    Ok(meets_bitcoin_target(&hash, &target))
}

#[napi]
/// Convert a Bitcoin-style difficulty to the corresponding target value.
///
/// @param {number} difficulty - The difficulty level (Bitcoin-style)
/// @returns {string} The target as a hex string
pub fn difficulty_to_target_hex(difficulty: f64) -> Result<String> {
    if difficulty <= 0.0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Difficulty must be greater than 0".to_string(),
        ));
    }

    let target = difficulty_to_target(difficulty);
    Ok(hex::encode(&target))
}

#[napi]
/// Calculate the difficulty that a given hash would satisfy.
///
/// @param {Buffer} hash - The hash to analyze
/// @returns {number} The difficulty level this hash would satisfy
pub fn hash_to_difficulty(hash: Buffer) -> Result<f64> {
    if hash.len() != 32 {
        return Err(Error::new(
            Status::InvalidArg,
            "Hash must be exactly 32 bytes".to_string(),
        ));
    }

    let difficulty = target_to_difficulty(&hash);
    Ok(difficulty)
}

/// Convert Bitcoin difficulty to target using a simplified formula
/// target = max_target >> (difficulty - 1) leading zero bits
fn difficulty_to_target(difficulty: f64) -> [u8; 32] {
    if difficulty <= 0.0 {
        return MAX_TARGET;
    }

    let mut result = [0u8; 32];

    if difficulty == 1.0 {
        // Difficulty 1.0: target requires at least 8 leading zero bits (1 byte)
        result[0] = 0x00;
        for i in 1..32 {
            result[i] = 0xff;
        }
    } else if difficulty == 2.0 {
        // Difficulty 2.0: target requires at least 12 leading zero bits
        result[0] = 0x00;
        result[1] = 0x0f;
        for i in 2..32 {
            result[i] = 0xff;
        }
    } else if difficulty == 4.0 {
        // Difficulty 4.0: target requires at least 16 leading zero bits (2 bytes)
        result[0] = 0x00;
        result[1] = 0x00;
        for i in 2..32 {
            result[i] = 0xff;
        }
    } else if difficulty == 8.0 {
        // Difficulty 8.0: target requires at least 20 leading zero bits
        result[0] = 0x00;
        result[1] = 0x00;
        result[2] = 0x0f;
        for i in 3..32 {
            result[i] = 0xff;
        }
    } else if difficulty == 16.0 {
        // Difficulty 16.0: target requires at least 24 leading zero bits (3 bytes)
        result[0] = 0x00;
        result[1] = 0x00;
        result[2] = 0x00;
        for i in 3..32 {
            result[i] = 0xff;
        }
    } else if difficulty >= 32.0 {
        // Very high difficulty: target requires many leading zero bits
        let zero_bytes = ((difficulty.log2() as usize) + 1).min(28);
        for i in 0..zero_bytes {
            result[i] = 0x00;
        }
        for i in zero_bytes..32 {
            result[i] = 0xff;
        }
    } else {
        // Intermediate difficulties: interpolate
        let zero_bits = (8.0 + (difficulty - 1.0) * 4.0) as usize;
        let zero_bytes = zero_bits / 8;
        let remaining_bits = zero_bits % 8;

        for i in 0..zero_bytes {
            result[i] = 0x00;
        }

        if zero_bytes < 32 && remaining_bits > 0 {
            result[zero_bytes] = 0xff >> remaining_bits;
        }

        for i in (zero_bytes + 1)..32 {
            result[i] = 0xff;
        }
    }

    result
}

/// Convert a target back to difficulty
fn target_to_difficulty(target: &[u8]) -> f64 {
    // Find the most significant non-zero byte
    let mut msb_index = 0;
    for (i, &byte) in target.iter().enumerate() {
        if byte != 0 {
            msb_index = i;
            break;
        }
    }

    // Very rough approximation of difficulty from target
    // Real Bitcoin uses precise big integer arithmetic
    let leading_zeros = msb_index * 8;
    let approximate_difficulty = 2.0_f64.powi(leading_zeros as i32);

    approximate_difficulty.max(1.0)
}

/// Check if a hash meets the Bitcoin target (hash must be less than or equal to target)
fn meets_bitcoin_target(hash: &[u8], target: &[u8; 32]) -> bool {
    // Compare hash to target as big-endian numbers
    // Hash must be <= target to be valid

    for i in 0..32 {
        if hash[i] < target[i] {
            return true; // hash < target, valid
        } else if hash[i] > target[i] {
            return false; // hash > target, invalid
        }
        // If equal, continue to next byte
    }

    true // hash == target, valid
}

/// Compute single SHA-256 hash
fn compute_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Compute double SHA-256 hash (like Bitcoin)
fn compute_double_sha256(data: &[u8]) -> [u8; 32] {
    let first_hash = compute_sha256(data);
    compute_sha256(&first_hash)
}
