const { 
    ProofOfStorageProver, 
    ProofOfStorageVerifier, 
    HierarchicalNetworkManager
} = require('./index.js');

/**
 * Hashchain Progression Demonstration
 * 
 * This example demonstrates:
 * - Creating 10 separate hashchains (one per file)
 * - Adding new blocks every 5 seconds for 2 minutes
 * - Watching chain state evolution with beautiful Rust logging
 */

// Helper function to delay execution
function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// Generate 10 different test files
function generateTestFiles() {
    const files = [
        {
            name: "config.json",
            content: JSON.stringify({
                version: "1.0.0",
                networkType: "chia-mainnet",
                storageCapacity: "1TB",
                timestamp: Date.now()
            }, null, 2)
        },
        {
            name: "research_paper.md",
            content: `# Proof of Storage Continuity Research
            
## Abstract
This document explores blockchain-based storage verification mechanisms.
Generated at: ${new Date().toISOString()}

## Introduction
Decentralized storage networks require robust proof mechanisms...`
        },
        {
            name: "dataset.csv",
            content: `timestamp,temperature,humidity,pressure
${Date.now()},22.5,65.2,1013.25
${Date.now() + 1000},21.8,66.1,1012.87
${Date.now() + 2000},21.2,67.5,1011.92
${Date.now() + 3000},20.9,68.2,1010.95`
        },
        {
            name: "smart_contract.sol",
            content: `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract StorageVerification {
    mapping(bytes32 => bool) public commitments;
    uint256 public createdAt = ${Math.floor(Date.now() / 1000)};
    
    function verifyCommitment(bytes32 hash) public returns (bool) {
        return commitments[hash];
    }
}`
        },
        {
            name: "user_profiles.json",
            content: JSON.stringify({
                users: [
                    { id: 1, name: "Alice", role: "prover", joinedAt: Date.now() },
                    { id: 2, name: "Bob", role: "verifier", joinedAt: Date.now() + 1000 },
                    { id: 3, name: "Charlie", role: "admin", joinedAt: Date.now() + 2000 }
                ],
                metadata: { version: "1.1", updated: new Date().toISOString() }
            })
        },
        {
            name: "network_logs.txt",
            content: `Network Activity Log
Generated: ${new Date().toISOString()}

${Date.now()} - Node joined network
${Date.now() + 1000} - First commitment received  
${Date.now() + 2000} - Peer discovery completed
${Date.now() + 3000} - Challenge-response cycle initiated`
        },
        {
            name: "image_metadata.json",
            content: JSON.stringify({
                filename: "blockchain_visualization.png",
                size: "2.4MB",
                resolution: "1920x1080",
                format: "PNG",
                checksum: `sha256:${Date.now().toString(16)}`,
                created: new Date().toISOString()
            })
        },
        {
            name: "blockchain_state.bin",
            content: Buffer.from([
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
                ...Array.from({length: 64}, () => Math.floor(Math.random() * 256))
            ]).toString('hex')
        },
        {
            name: "consensus_data.xml",
            content: `<?xml version="1.0" encoding="UTF-8"?>
<consensus timestamp="${Date.now()}">
  <validators>
    <validator id="1" stake="1000000" status="active"/>
    <validator id="2" stake="750000" status="active"/>
    <validator id="3" stake="500000" status="pending"/>
  </validators>
  <metrics>
    <block_time>52</block_time>
    <finality_time>120</finality_time>
  </metrics>
</consensus>`
        },
        {
            name: "performance_stats.json",
            content: JSON.stringify({
                metrics: {
                    throughput: "1000 tx/s",
                    latency: "200ms",
                    storage_efficiency: "95%",
                    network_uptime: "99.9%"
                },
                timestamp: Date.now(),
                node_id: `node_${Math.random().toString(36).substr(2, 9)}`,
                version: "2.1.0"
            })
        }
    ];

    return files.map((file, index) => ({
        ...file,
        id: index + 1,
        data: Buffer.from(file.content, 'utf8'),
        size: Buffer.byteLength(file.content, 'utf8')
    }));
}

// Simplified prover callbacks
const proverCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => Math.floor(Date.now() / 10000),
        getBlockHash: (height) => Buffer.from(`block_hash_${height}`.padEnd(32, '0')),
        getBlockchainEntropy: () => Buffer.from('blockchain_entropy_32_bytes_long'),
        submitCommitment: (commitment) => true
    },
    economic: {
        stakeTokens: (amount) => true,
        getStakeAmount: () => 1000000,
        onStakeSlashed: (amount) => true,
        claimRewards: () => 50000
    },
    storage: {
        storeChunk: (chunkIndex, data) => true,
        retrieveChunk: (chunkIndex) => Buffer.alloc(4096, chunkIndex % 256),
        verifyDataIntegrity: () => true,
        getStorageStats: () => ({ totalChunks: 1000, totalSize: 4096000, availableSpace: 1000000000 })
    },
    network: {
        announceAvailability: () => true,
        submitChallengeResponse: (response) => true,
        broadcastProof: (proof) => true
    },
    peerNetwork: {
        registerPeer: (peerId, metadata) => true,
        getPeerInfo: (peerId) => ({ peerId, nodeType: "prover", lastSeen: Date.now(), reputation: 90 }),
        updatePeerLatency: (peerId, latencyMs) => true,
        removePeer: (peerId) => true,
        getActivePeers: () => [Buffer.alloc(32, 1), Buffer.alloc(32, 2)]
    },
    availabilityChallenge: {
        issueAvailabilityChallenge: (proverKey, commitmentHash) => ({
            challengeId: Buffer.alloc(32, 0x01),
            proverKey,
            commitmentHash,
            challengedChunks: [0, 5, 10],
            nonce: Buffer.alloc(16, 0x02),
            timestamp: Date.now(),
            deadline: Date.now() + 60000
        }),
        validateAvailabilityResponse: (challenge, response) => true,
        getChallengeDifficulty: () => 1000,
        reportChallengeResult: (challengeId, success, metadata) => true,
        getProverAvailabilityScore: (proverKey) => 95
    },
    blockchainData: {
        validateChunkCount: (fileHash, reportedChunks) => reportedChunks > 0 && reportedChunks < 1000000,
        getDataFileMetadata: (fileHash) => ({
            fileHash, totalChunks: 1000, chunkSize: 4096, encodingVersion: 1, registrationHeight: 1000
        }),
        verifyDataRegistration: (fileHash) => true,
        getConfirmedStorageSize: () => 1000000,
        updateAvailabilityStatus: (fileHash, status) => true
    }
};

// Simplified verifier callbacks
const verifierCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => Math.floor(Date.now() / 10000),
        getBlockHash: (height) => Buffer.from(`block_hash_${height}`.padEnd(32, '0')),
        validateBlockHash: (hash) => true,
        getCommitment: (commitmentHash) => null
    },
    challenge: {
        issueChallenge: (proverKey, commitmentHash) => ({
            challengeId: Buffer.alloc(32, 0x01),
            proverKey,
            commitmentHash,
            challengedChunks: [0, 5, 10],
            nonce: Buffer.alloc(16, 0x02),
            timestamp: Date.now(),
            deadline: Date.now() + 60000
        }),
        validateResponse: (challenge, response) => true,
        reportResult: (challengeId, success) => true
    },
    network: {
        discoverProvers: () => [Buffer.alloc(32, 1), Buffer.alloc(32, 2)],
        getProverReputation: (proverKey) => 0.95,
        reportMisbehavior: (proverKey, evidence) => true
    },
    economic: {
        rewardVerification: (amount) => true,
        penalizeFailure: (amount) => true
    },
    peerNetwork: {
        registerPeer: (peerId, metadata) => true,
        getPeerInfo: (peerId) => ({ peerId, nodeType: "verifier", lastSeen: Date.now(), reputation: 90 }),
        updatePeerLatency: (peerId, latencyMs) => true,
        removePeer: (peerId) => true,
        getActivePeers: () => [Buffer.alloc(32, 3), Buffer.alloc(32, 4)]
    },
    availabilityChallenge: {
        issueAvailabilityChallenge: (proverKey, commitmentHash) => ({
            challengeId: Buffer.alloc(32, 0x01),
            proverKey,
            commitmentHash,
            challengedChunks: [0, 5, 10],
            nonce: Buffer.alloc(16, 0x02),
            timestamp: Date.now(),
            deadline: Date.now() + 60000
        }),
        validateAvailabilityResponse: (challenge, response) => true,
        getChallengeDifficulty: () => 1000,
        reportChallengeResult: (challengeId, success, metadata) => true,
        getProverAvailabilityScore: (proverKey) => 90
    },
    blockchainData: {
        validateChunkCount: (fileHash, reportedChunks) => reportedChunks > 0 && reportedChunks < 1000000,
        getDataFileMetadata: (fileHash) => ({
            fileHash, totalChunks: 1000, chunkSize: 4096, encodingVersion: 1, registrationHeight: 1000
        }),
        verifyDataRegistration: (fileHash) => true,
        getConfirmedStorageSize: () => 1000000,
        updateAvailabilityStatus: (fileHash, status) => true
    }
};

// Main demonstration function
async function demonstrateHashchainProgression() {
    console.log("\n🚀 Starting 10-Chain Hashchain Progression Demo");
    console.log("📊 Creating 10 files → 10 hashchains → 24 rounds → 240 total blocks");
    console.log("⏱️  Timeline: 5 second intervals for 2 minutes");
    console.log("=".repeat(70));
    
    try {
        // Generate 10 test files
        const testFiles = generateTestFiles();
        console.log(`\n📁 Generated ${testFiles.length} test files:`);
        testFiles.forEach((file, i) => {
            console.log(`   ${i + 1}. ${file.name} (${file.size} bytes)`);
        });

        // Create prover instances for each file
        console.log("\n🔧 Initializing provers and creating initial hashchains...");
        const provers = [];
        const commitments = [];
        
        for (let i = 0; i < testFiles.length; i++) {
            // Create exactly 32-byte key
            const keyBuffer = Buffer.alloc(32);
            const keyString = `prover_${i}_key`;
            const keyBytes = Buffer.from(keyString, 'utf8');
            keyBytes.copy(keyBuffer, 0, 0, Math.min(keyBytes.length, 32));
            const proverKey = keyBuffer;
            const prover = new ProofOfStorageProver(proverKey, proverCallbacks);
            
            // Store the file and create initial commitment
            const commitment = prover.storeData(testFiles[i].data, `./demo_output_${i}`);
            
            // Generate unique chain ID and add to chain tracker
            const chainId = prover.generateChainId(testFiles[i].name);
            prover.addCommitmentToChain(chainId, commitment, testFiles[i].name, testFiles[i].size);
            
            provers.push({ prover, chainId, fileName: testFiles[i].name });
            commitments.push(commitment);
            
            console.log(`   ✅ Chain ${i + 1}: ${testFiles[i].name} → Commitment: ${commitment.commitmentHash.toString('hex').substring(0, 16)}...`);
        }

        // Create verifier and network manager
        // Create exactly 32-byte verifier key
        const verifierKeyBuffer = Buffer.alloc(32);
        const verifierKeyString = 'verifier_demo_key';
        const verifierKeyBytes = Buffer.from(verifierKeyString, 'utf8');
        verifierKeyBytes.copy(verifierKeyBuffer, 0, 0, Math.min(verifierKeyBytes.length, 32));
        const verifierKey = verifierKeyBuffer;
        const verifier = new ProofOfStorageVerifier(verifierKey, verifierCallbacks);
        
        // Create network manager key
        const networkKeyBuffer = Buffer.alloc(32);
        const networkKeyString = 'network_manager_key';
        const networkKeyBytes = Buffer.from(networkKeyString, 'utf8');
        networkKeyBytes.copy(networkKeyBuffer, 0, 0, Math.min(networkKeyBytes.length, 32));
        const networkManager = new HierarchicalNetworkManager(networkKeyBuffer, "prover");

        console.log("\n🌐 Starting 2-minute progression (24 rounds × 10 chains = 240 blocks)");
        console.log("⏰ Block interval: 5 seconds");
        console.log("=".repeat(70));

        const startTime = Date.now();
        const duration = 2 * 60 * 1000; // 2 minutes
        const interval = 5 * 1000; // 5 seconds
        let round = 0;
        let totalBlocks = 0;

        while (Date.now() - startTime < duration) {
            round++;
            const roundStartTime = Date.now();
            
            console.log(`\n🔄 === ROUND ${round}/24 === (${Math.floor((Date.now() - startTime) / 1000)}s elapsed)`);
            
            // Add a new block to each chain
            for (let i = 0; i < provers.length; i++) {
                const { prover, chainId, fileName } = provers[i];
                const file = testFiles[i];
                
                // Generate new commitment (simulating new block)
                const newCommitment = prover.generateCommitment(round);
                commitments[i] = newCommitment;
                
                // Add the new commitment to the chain tracker
                prover.addCommitmentToChain(chainId, newCommitment, fileName, file.size);
                
                // Increment block height in tracker
                prover.incrementBlockHeight();
                
                totalBlocks++;
                
                // Periodically test other operations
                if (round % 3 === 0) {
                    // Test proof generation and verification every 3rd round
                    if (i === 0) { // Only test with first prover to avoid spam
                        const compactProof = prover.createCompactProof();
                        const isValid = verifier.verifyCompactProof(compactProof);
                        console.log(`   🛡️  Proof verification (Chain 1): ${isValid ? 'PASSED' : 'FAILED'}`);
                    }
                }
                
                if (round % 4 === 0) {
                    // Test network operations every 4th round
                    if (i === 1) { // Only test with second prover
                        prover.registerPeer(`peer_${round}_${i}`, "peer_metadata");
                        const challengeId = prover.issueAvailabilityChallenge(Buffer.alloc(32, i));
                    }
                }
            }
            
            // Display progress
            const avgBlockTime = (Date.now() - roundStartTime) / provers.length;
            console.log(`   📈 Added ${provers.length} blocks in ${Date.now() - roundStartTime}ms (avg: ${avgBlockTime.toFixed(1)}ms/block)`);
            console.log(`   🎯 Total blocks: ${totalBlocks}, Chains: ${provers.length}, Round: ${round}/24`);
            
            // Show chain state every 5th round
            if (round % 5 === 0) {
                console.log(`\n📊 === CHAIN STATE SNAPSHOT (Round ${round}) ===`);
                provers[0].prover.displayChainState(); // Show state from first prover
                
                const stats = provers[0].prover.getLoggingStats();
                const parsedStats = JSON.parse(stats);
                console.log(`📈 Statistics: ${parsedStats.total_chains} chains, ${parsedStats.total_commitments} commitments, ${parsedStats.commitments_per_second.toFixed(2)}/s`);
            }
            
            // Wait for next interval
            const timeToWait = interval - (Date.now() - roundStartTime);
            if (timeToWait > 0) {
                await delay(timeToWait);
            }
        }

        const totalTime = Date.now() - startTime;
        
        console.log("\n🏆 === FINAL RESULTS ===");
        console.log(`⏱️  Total runtime: ${(totalTime / 1000).toFixed(1)} seconds`);
        console.log(`📦 Total rounds: ${round}`);
        console.log(`🔗 Total blocks created: ${totalBlocks}`);
        console.log(`📊 Average blocks per second: ${(totalBlocks / (totalTime / 1000)).toFixed(2)}`);
        console.log(`🌐 Chains: ${provers.length}`);
        console.log(`📈 Blocks per chain: ${(totalBlocks / provers.length).toFixed(1)}`);
        
        // Final verification
        console.log("\n🔍 === FINAL VERIFICATION ===");
        let totalVerified = 0;
        for (let i = 0; i < Math.min(3, provers.length); i++) {
            const proof = provers[i].prover.createCompactProof();
            const isValid = verifier.verifyCompactProof(proof);
            console.log(`   Chain ${i + 1} (${testFiles[i].name}): ${isValid ? '✅ VALID' : '❌ INVALID'}`);
            if (isValid) totalVerified++;
        }
        
        // Final network stats
        const networkStats = networkManager.getNetworkStats();
        console.log(`\n🌐 Network Health: ${(networkStats.healthScore * 100).toFixed(1)}%`);
        console.log(`🔒 Storage: ${(networkStats.totalStorage / 1024 / 1024).toFixed(2)}MB committed`);
        
        console.log("\n" + "=".repeat(70));
        console.log("🎉 Hashchain Progression Demo Completed Successfully!");
        
        return {
            success: true,
            totalBlocks,
            totalChains: provers.length,
            rounds: round,
            runtime: totalTime,
            verified: totalVerified
        };
        
    } catch (error) {
        console.error("\n❌ Demo failed:", error.message);
        console.error("=".repeat(70));
        return {
            success: false,
            error: error.message
        };
    }
}

// Run the demonstration
if (require.main === module) {
    demonstrateHashchainProgression()
        .then(result => {
            if (result.success) {
                console.log(`\n✅ SUCCESS: ${result.totalBlocks} blocks across ${result.totalChains} chains`);
                console.log(`📊 Performance: ${(result.totalBlocks / (result.runtime / 1000)).toFixed(2)} blocks/second`);
            }
        })
        .catch(error => {
            console.error("❌ Unexpected error:", error);
        });
}

module.exports = {
    demonstrateHashchainProgression,
    generateTestFiles,
    proverCallbacks,
    verifierCallbacks
}; 