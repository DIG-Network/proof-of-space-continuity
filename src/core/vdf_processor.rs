use crate::core::errors::HashChainResult;
use crate::core::utils::{compute_blake3, sign_data, ContinuousVDF};
use log::{debug, info, trace};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Shared VDF proof that demonstrates the VDF is running continuously
/// and hasn't been manipulated across chains
#[derive(Clone, Debug)]
pub struct SharedVDFProof {
    /// VDF state at the time of proof generation
    pub vdf_state: [u8; 32],
    /// Total iterations at proof time
    pub total_iterations: u64,
    /// Timestamp when proof was generated
    pub timestamp: f64,
    /// Signature of the proof using the prover's private key
    pub signature: Vec<u8>,
    /// Hash of all previous proofs (chain of proofs)
    pub proof_chain_hash: [u8; 32],
}

/// VDF processor that runs in the background with shared proof generation
pub struct VDFProcessor {
    vdf: Arc<Mutex<ContinuousVDF>>,
    target_iterations_per_second: u64,
    running: Arc<Mutex<bool>>,
    prover_private_key: Vec<u8>,
    shared_proofs: Arc<Mutex<Vec<SharedVDFProof>>>,
    last_proof_time: Arc<Mutex<f64>>,
    proof_interval_seconds: f64,
}

impl VDFProcessor {
    pub fn new(
        initial_state: [u8; 32],
        memory_kb: u32,
        target_iterations_per_second: u64,
        prover_private_key: Vec<u8>,
    ) -> Self {
        Self {
            vdf: Arc::new(Mutex::new(ContinuousVDF::new(initial_state, memory_kb))),
            target_iterations_per_second,
            running: Arc::new(Mutex::new(false)),
            prover_private_key,
            shared_proofs: Arc::new(Mutex::new(Vec::new())),
            last_proof_time: Arc::new(Mutex::new(0.0)),
            proof_interval_seconds: 10.0, // Generate shared proof every 10 seconds
        }
    }

    /// Start the VDF processor in a background thread
    pub fn start(&self) {
        let vdf = self.vdf.clone();
        let running = self.running.clone();
        let target_iterations = self.target_iterations_per_second;
        let prover_private_key = self.prover_private_key.clone();
        let shared_proofs = self.shared_proofs.clone();
        let last_proof_time = self.last_proof_time.clone();
        let proof_interval = self.proof_interval_seconds;

        *running.lock().unwrap() = true;

        thread::spawn(move || {
            let mut last_iteration_time = std::time::Instant::now();
            let target_interval = Duration::from_secs_f64(1.0 / target_iterations as f64);
            let mut iteration_count = 0u64;

            info!(
                "🚀 VDF Processor started - target: {} iterations/sec",
                target_iterations
            );

            while *running.lock().unwrap() {
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(last_iteration_time);

                if elapsed >= target_interval {
                    // Perform VDF iteration
                    let _current_state = {
                        let mut vdf_guard = vdf.lock().unwrap();
                        let state = vdf_guard.iterate();
                        let (_, total_iterations) = vdf_guard.get_state();

                        // Trace logging for VDF iterations
                        if iteration_count % 100 == 0 {
                            // Log every 100 iterations to avoid spam
                            trace!(
                                "[VDF TRACE] Iteration: {} | State: {} | Memory Access: {} bytes",
                                total_iterations,
                                hex::encode(&state[..8]),
                                vdf_guard.memory_size
                            );

                            // Also use eprintln! for trace level to ensure it appears in Node.js output
                            // Check if RUST_LOG contains "trace" to determine if trace logging is enabled
                            if std::env::var("RUST_LOG")
                                .unwrap_or_default()
                                .contains("trace")
                            {
                                eprintln!(
                                    "[VDF TRACE] Iteration: {} | State: {} | Memory Access: {} bytes",
                                    total_iterations,
                                    hex::encode(&state[..8]),
                                    vdf_guard.memory_size
                                );
                            }
                        }

                        state
                    };

                    iteration_count += 1;
                    last_iteration_time = now;

                    // Generate shared proof periodically
                    let current_time = crate::core::utils::get_current_timestamp();
                    let should_generate_proof = {
                        let last_proof = *last_proof_time.lock().unwrap();
                        current_time - last_proof >= proof_interval
                    };

                    if should_generate_proof {
                        if let Ok(proof) =
                            Self::generate_shared_proof(&vdf, &prover_private_key, &shared_proofs)
                        {
                            let mut proofs = shared_proofs.lock().unwrap();
                            proofs.push(proof.clone());

                            // Keep only last 100 proofs
                            if proofs.len() > 100 {
                                let excess = proofs.len() - 100;
                                proofs.drain(0..excess);
                            }

                            *last_proof_time.lock().unwrap() = current_time;

                            debug!(
                                "📋 Generated shared VDF proof: iterations={}, state={}",
                                proof.total_iterations,
                                hex::encode(&proof.vdf_state[..8])
                            );
                        }
                    }
                } else {
                    // Sleep until next iteration
                    let sleep_duration = target_interval - elapsed;
                    if sleep_duration > Duration::from_nanos(1) {
                        thread::sleep(sleep_duration);
                    }
                }
            }

            info!("🛑 VDF Processor stopped");
        });
    }

    /// Generate a shared VDF proof that demonstrates continuous operation
    fn generate_shared_proof(
        vdf: &Arc<Mutex<ContinuousVDF>>,
        prover_private_key: &[u8],
        existing_proofs: &Arc<Mutex<Vec<SharedVDFProof>>>,
    ) -> HashChainResult<SharedVDFProof> {
        let (vdf_state, total_iterations) = {
            let vdf_guard = vdf.lock().unwrap();
            vdf_guard.get_state()
        };

        let timestamp = crate::core::utils::get_current_timestamp();

        // Create proof chain hash from previous proofs
        let proof_chain_hash = {
            let proofs = existing_proofs.lock().unwrap();
            if proofs.is_empty() {
                compute_blake3(b"genesis_vdf_proof")
            } else {
                let last_proof = &proofs[proofs.len() - 1];
                compute_blake3(
                    &[
                        &last_proof.proof_chain_hash[..],
                        &last_proof.vdf_state[..],
                        &last_proof.total_iterations.to_be_bytes(),
                    ]
                    .concat(),
                )
            }
        };

        // Create proof data to sign
        let proof_data = [
            &vdf_state[..],
            &total_iterations.to_be_bytes(),
            &timestamp.to_be_bytes(),
            &proof_chain_hash[..],
        ]
        .concat();

        // Sign the proof
        let signature = sign_data(prover_private_key, &proof_data)?;

        Ok(SharedVDFProof {
            vdf_state,
            total_iterations,
            timestamp,
            signature,
            proof_chain_hash,
        })
    }

    /// Stop the VDF processor
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }

    /// Get current VDF state and iteration count
    pub fn get_state(&self) -> ([u8; 32], u64) {
        self.vdf.lock().unwrap().get_state()
    }

    /// Sign a block against the current VDF state
    pub fn sign_block(
        &self,
        block_height: u64,
        block_hash: [u8; 32],
        required_iterations: u64,
    ) -> Result<[u8; 32], String> {
        match self
            .vdf
            .lock()
            .unwrap()
            .sign_block(block_height, block_hash, required_iterations)
        {
            Ok(signature) => Ok(signature),
            Err(e) => Err(format!("VDF signing failed: {:?}", e)),
        }
    }

    /// Verify a block signature
    pub fn verify_block_signature(
        &self,
        block_height: u64,
        block_hash: [u8; 32],
        signature: [u8; 32],
        required_iterations: u64,
    ) -> bool {
        self.vdf.lock().unwrap().verify_block_signature(
            block_height,
            block_hash,
            signature,
            required_iterations,
        )
    }

    /// Get the latest shared VDF proof
    pub fn get_latest_shared_proof(&self) -> Option<SharedVDFProof> {
        let proofs = self.shared_proofs.lock().unwrap();
        proofs.last().cloned()
    }

    /// Get all shared VDF proofs (for verification)
    pub fn get_all_shared_proofs(&self) -> Vec<SharedVDFProof> {
        self.shared_proofs.lock().unwrap().clone()
    }

    /// Verify the integrity of the shared VDF proof chain
    pub fn verify_shared_proof_chain(&self, prover_public_key: &[u8]) -> bool {
        let proofs = self.shared_proofs.lock().unwrap();

        if proofs.is_empty() {
            return true; // Empty chain is valid
        }

        let mut expected_chain_hash = compute_blake3(b"genesis_vdf_proof");

        for (i, proof) in proofs.iter().enumerate() {
            // Verify proof chain hash
            if i > 0 && proof.proof_chain_hash != expected_chain_hash {
                debug!("❌ Shared VDF proof chain broken at index {}", i);
                return false;
            }

            // Verify signature
            let proof_data = [
                &proof.vdf_state[..],
                &proof.total_iterations.to_be_bytes(),
                &proof.timestamp.to_be_bytes(),
                &proof.proof_chain_hash[..],
            ]
            .concat();

            if let Ok(valid) = crate::core::utils::verify_signature(
                prover_public_key,
                &proof_data,
                &proof.signature,
            ) {
                if !valid {
                    debug!("❌ Invalid signature in shared VDF proof at index {}", i);
                    return false;
                }
            } else {
                debug!(
                    "❌ Failed to verify signature in shared VDF proof at index {}",
                    i
                );
                return false;
            }

            // Update expected chain hash for next iteration
            expected_chain_hash = compute_blake3(
                &[
                    &proof.proof_chain_hash[..],
                    &proof.vdf_state[..],
                    &proof.total_iterations.to_be_bytes(),
                ]
                .concat(),
            );
        }

        debug!(
            "✅ Shared VDF proof chain verified successfully ({} proofs)",
            proofs.len()
        );
        true
    }

    /// Get VDF performance statistics
    pub fn get_performance_stats(&self) -> VDFPerformanceStats {
        let (_, total_iterations) = self.get_state();
        let start_time = {
            let vdf_guard = self.vdf.lock().unwrap();
            vdf_guard.start_time
        };

        let elapsed_seconds = start_time.elapsed().as_secs_f64();
        let actual_iterations_per_second = if elapsed_seconds > 0.0 {
            total_iterations as f64 / elapsed_seconds
        } else {
            0.0
        };

        VDFPerformanceStats {
            total_iterations,
            elapsed_seconds,
            target_iterations_per_second: self.target_iterations_per_second,
            actual_iterations_per_second,
            shared_proofs_count: self.shared_proofs.lock().unwrap().len(),
        }
    }
}

/// VDF performance statistics
#[derive(Debug, Clone)]
pub struct VDFPerformanceStats {
    pub total_iterations: u64,
    pub elapsed_seconds: f64,
    pub target_iterations_per_second: u64,
    pub actual_iterations_per_second: f64,
    pub shared_proofs_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vdf_processor_with_shared_proofs() {
        // Generate test keys
        let private_key = [1u8; 32];
        let public_key = {
            use ed25519_dalek::{PublicKey, SecretKey};
            let secret = SecretKey::from_bytes(&private_key).unwrap();
            PublicKey::from(&secret).to_bytes().to_vec()
        };

        let initial_state = [1u8; 32];
        let processor = VDFProcessor::new(
            initial_state,
            256,  // 256KB memory
            1000, // 1000 iterations per second
            private_key.to_vec(),
        );

        // Start processor
        processor.start();

        // Wait for some iterations
        thread::sleep(Duration::from_millis(100));

        // Get state
        let (_state, iterations) = processor.get_state();
        assert!(iterations > 0);

        // Wait for a shared proof to be generated
        thread::sleep(Duration::from_secs(1));

        // Verify shared proof chain
        assert!(processor.verify_shared_proof_chain(&public_key));

        // Get performance stats
        let stats = processor.get_performance_stats();
        assert!(stats.actual_iterations_per_second > 0.0);

        // Stop processor
        processor.stop();
    }
}
