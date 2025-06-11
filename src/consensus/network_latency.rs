use napi::bindgen_prelude::*;

use std::collections::HashMap;

use crate::core::{
    
    types::*,
    utils::compute_sha256,
};

/// Network latency proof system to prevent outsourcing attacks
/// Verifies that storage is geographically distributed and not centralized
pub struct NetworkLatencyProver {
    peer_connections: HashMap<String, PeerConnection>,
    max_acceptable_latency_ms: f64,
    variance_threshold: f64,
}

#[derive(Clone)]
pub struct PeerConnection {
    peer_id: String,
    address: String,
    last_latency_ms: f64,
    latency_history: Vec<f64>,
    connection_established: bool,
}

impl NetworkLatencyProver {
    /// Create new network latency prover
    pub fn new() -> Self {
        NetworkLatencyProver {
            peer_connections: HashMap::new(),
            max_acceptable_latency_ms: NETWORK_LATENCY_MAX_MS as f64,
            variance_threshold: NETWORK_LATENCY_VARIANCE_MAX,
        }
    }

    /// Add peer for latency monitoring
    pub fn add_peer(&mut self, peer_id: String, address: String) -> Result<()> {
        let peer = PeerConnection {
            peer_id: peer_id.clone(),
            address,
            last_latency_ms: 0.0,
            latency_history: Vec::new(),
            connection_established: false,
        };

        self.peer_connections.insert(peer_id, peer);
        Ok(())
    }

    /// Measure latency to specific peer
    pub fn measure_peer_latency(&mut self, peer_id: &str) -> Result<PeerLatencyMeasurement> {
        // Clone the address to avoid borrowing conflicts
        let peer_address = {
            let peer = self.peer_connections.get(peer_id)
                .ok_or_else(|| Error::new(Status::GenericFailure, "Peer not found".to_string()))?;
            peer.address.clone()
        };

        // Simulate network latency measurement
        // In real implementation, this would ping the peer
        let latency_ms = self.simulate_network_ping(&peer_address)?;
        
        // Now we can safely get mutable access to update the peer
        let peer = self.peer_connections.get_mut(peer_id)
            .ok_or_else(|| Error::new(Status::GenericFailure, "Peer not found".to_string()))?;
        
        // Update peer history
        peer.last_latency_ms = latency_ms;
        peer.latency_history.push(latency_ms);
        peer.connection_established = true;

        // Keep history limited
        if peer.latency_history.len() > 50 {
            peer.latency_history.remove(0);
        }

        let measurement = PeerLatencyMeasurement {
            peer_id: Buffer::from(peer_id.as_bytes().to_vec()),
            latency_ms,
            sample_count: peer.latency_history.len() as u32,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        };

        Ok(measurement)
    }

    /// Measure latency to all peers and generate proof
    pub fn generate_latency_proof(&mut self) -> Result<NetworkLatencyProof> {
        let mut peer_latencies = Vec::new();
        let mut total_latency = 0.0;
        let mut valid_measurements = 0;

        // Measure latency to each peer
        for peer_id in self.peer_connections.keys().cloned().collect::<Vec<_>>() {
            match self.measure_peer_latency(&peer_id) {
                Ok(measurement) => {
                    total_latency += measurement.latency_ms;
                    valid_measurements += 1;
                    peer_latencies.push(measurement);
                }
                Err(_) => {
                    // Skip failed measurements but continue with others
                }
            }
        }

        if valid_measurements == 0 {
            return Err(Error::new(
                Status::GenericFailure,
                "No valid latency measurements".to_string(),
            ));
        }

        let average_latency_ms = total_latency / valid_measurements as f64;

        // Calculate variance
        let variance = self.calculate_latency_variance(&peer_latencies, average_latency_ms);

        // Generate location proof (simplified)
        let location_proof = self.generate_location_proof(&peer_latencies)?;

        let proof = NetworkLatencyProof {
            peer_latencies,
            average_latency_ms,
            latency_variance: variance,
            measurement_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            location_proof: Some(Buffer::from(location_proof)),
        };

        Ok(proof)
    }

    /// Verify network latency proof for anti-outsourcing
    pub fn verify_latency_proof(&self, proof: &NetworkLatencyProof) -> Result<bool> {
        // Check minimum number of peers
        if proof.peer_latencies.len() < NETWORK_LATENCY_SAMPLES as usize {
            return Ok(false);
        }

        // Check average latency is within acceptable range
        if proof.average_latency_ms > self.max_acceptable_latency_ms {
            return Ok(false);
        }

        // Check latency variance is within acceptable range
        if proof.latency_variance > self.variance_threshold {
            return Ok(false);
        }

        // Verify individual measurements are reasonable
        for measurement in &proof.peer_latencies {
            if measurement.latency_ms > self.max_acceptable_latency_ms * 2.0 {
                return Ok(false);
            }
            if measurement.latency_ms < 1.0 {
                return Ok(false); // Too fast to be realistic
            }
        }

        // Check measurement timing
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        
        let measurement_age = current_time - proof.measurement_time;
        if measurement_age > 300.0 { // 5 minutes
            return Ok(false); // Too old
        }

        Ok(true)
    }

    /// Simulate network ping (in real implementation, would use actual network calls)
    fn simulate_network_ping(&self, _address: &str) -> Result<f64> {
        // Simulate realistic latency based on address characteristics
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        _address.hash(&mut hasher);
        let hash_value = hasher.finish();

        // Generate latency between 5ms and 95ms based on address hash
        let base_latency = 5.0 + (hash_value % 90) as f64;
        
        // Add some random variation (±10ms)
        let variation = (hash_value % 20) as f64 - 10.0;
        let final_latency = base_latency + variation;

        Ok(final_latency.max(1.0))
    }

    /// Calculate latency variance
    fn calculate_latency_variance(&self, measurements: &[PeerLatencyMeasurement], average: f64) -> f64 {
        if measurements.len() < 2 {
            return 0.0;
        }

        let variance_sum: f64 = measurements
            .iter()
            .map(|m| (m.latency_ms - average).powi(2))
            .sum();

        variance_sum / measurements.len() as f64
    }

    /// Generate geographic location proof
    fn generate_location_proof(&self, measurements: &[PeerLatencyMeasurement]) -> Result<Vec<u8>> {
        // Simplified location proof based on latency patterns
        let mut proof_input = Vec::new();

        // Add latency measurements to proof
        for measurement in measurements {
            proof_input.extend_from_slice(&measurement.peer_id);
            proof_input.extend_from_slice(&measurement.latency_ms.to_be_bytes());
        }

        // Add timestamp for freshness
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        
        proof_input.extend_from_slice(&timestamp.to_be_bytes());
        proof_input.extend_from_slice(b"geographic_distribution_proof");

        Ok(compute_sha256(&proof_input).to_vec())
    }

    /// Get network latency statistics
    pub fn get_latency_stats(&self) -> LatencyStats {
        let connected_peers = self.peer_connections.values()
            .filter(|p| p.connection_established)
            .count() as u32;

        let average_latency = if connected_peers > 0 {
            self.peer_connections.values()
                .filter(|p| p.connection_established)
                .map(|p| p.last_latency_ms)
                .sum::<f64>() / connected_peers as f64
        } else {
            0.0
        };

        LatencyStats {
            total_peers: self.peer_connections.len() as u32,
            connected_peers,
            average_latency_ms: average_latency,
            max_acceptable_latency_ms: self.max_acceptable_latency_ms,
        }
    }

    /// Get peer by ID - uses the peer_id field
    pub fn get_peer_by_id(&self, peer_id: &str) -> Option<&PeerConnection> {
        self.peer_connections.get(peer_id)
    }

    /// List all peer IDs - uses the peer_id field
    pub fn list_peer_ids(&self) -> Vec<String> {
        self.peer_connections.values()
            .map(|peer| peer.peer_id.clone())
            .collect()
    }

    /// Remove peer by ID - uses the peer_id field  
    pub fn remove_peer(&mut self, peer_id: &str) -> bool {
        if let Some(_peer) = self.peer_connections.remove(peer_id) {
            // Log the peer_id being removed
            true
        } else {
            false
        }
    }

    /// Get peer connection status by ID - uses the peer_id field
    pub fn is_peer_connected(&self, peer_id: &str) -> bool {
        self.peer_connections.get(peer_id)
            .map(|peer| peer.connection_established)
            .unwrap_or(false)
    }

    /// Validate peer ID format - uses the peer_id field
    pub fn validate_peer_id(&self, peer_id: &str) -> bool {
        if let Some(peer) = self.peer_connections.get(peer_id) {
            // Validate the stored peer_id matches the lookup key
            peer.peer_id == peer_id && !peer.peer_id.is_empty()
        } else {
            false
        }
    }
}

/// Network latency statistics
#[derive(Debug, Clone)]
pub struct LatencyStats {
    pub total_peers: u32,
    pub connected_peers: u32,
    pub average_latency_ms: f64,
    pub max_acceptable_latency_ms: f64,
}

/// Network latency verification for consensus
pub fn verify_network_distribution(proof: &NetworkLatencyProof) -> Result<bool> {
    let prover = NetworkLatencyProver::new();
    prover.verify_latency_proof(proof)
}

/// Create network latency proof for block processing
pub fn create_network_proof(peer_addresses: Vec<String>) -> Result<NetworkLatencyProof> {
    let mut prover = NetworkLatencyProver::new();

    // Add all peers
    for (i, address) in peer_addresses.iter().enumerate() {
        prover.add_peer(format!("peer_{}", i), address.clone())?;
    }

    // Generate proof
    prover.generate_latency_proof()
}

/// Detect potential outsourcing based on latency patterns
pub fn detect_outsourcing_patterns(
    historical_proofs: &[NetworkLatencyProof]
) -> Result<OutsourcingRisk> {
    if historical_proofs.is_empty() {
        return Ok(OutsourcingRisk::Unknown);
    }

    let mut _consistent_patterns = 0;
    let mut suspicious_patterns = 0;

    for proof in historical_proofs {
        // Check for suspiciously consistent latencies
        let latency_variance = proof.latency_variance;
        
        if latency_variance < 1.0 {
            // Very consistent latencies might indicate centralized infrastructure
            suspicious_patterns += 1;
        } else if latency_variance > 50.0 {
            // High variance might indicate legitimate geographic distribution
            _consistent_patterns += 1;
        }

        // Check for impossibly fast latencies
        for measurement in &proof.peer_latencies {
            if measurement.latency_ms < 0.5 {
                suspicious_patterns += 1;
                break;
            }
        }
    }

    let total_proofs = historical_proofs.len();
    let suspicious_ratio = suspicious_patterns as f64 / total_proofs as f64;

    if suspicious_ratio > 0.5 {
        Ok(OutsourcingRisk::High)
    } else if suspicious_ratio > 0.2 {
        Ok(OutsourcingRisk::Medium)
    } else {
        Ok(OutsourcingRisk::Low)
    }
}

/// Outsourcing risk assessment
#[derive(Debug, Clone, PartialEq)]
pub enum OutsourcingRisk {
    Low,
    Medium,
    High,
    Unknown,
} 