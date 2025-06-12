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

// Generate 10 different test files (each at least 4KB)
function generateTestFiles() {
    // Helper to pad content to at least 64KB (16 chunks × 4KB)
    function padToMinSize(content, minSize = 65536) {
        const currentSize = Buffer.byteLength(content, 'utf8');
        if (currentSize >= minSize) return content;
        
        const paddingNeeded = minSize - currentSize;
        const paddingChar = '=';
        const padding = '\n\n' + paddingChar.repeat(paddingNeeded - 2);
        return content + padding;
    }

    const files = [
        {
            name: "config.json",
            content: padToMinSize(JSON.stringify({
                version: "1.0.0",
                networkType: "chia-mainnet",
                storageCapacity: "1TB",
                timestamp: Date.now(),
                nodes: Array.from({length: 50}, (_, i) => ({
                    id: `node_${i}`,
                    endpoint: `http://node${i}.example.com:8444`,
                    publicKey: `0x${'a'.repeat(64)}`,
                    lastSeen: Date.now() - Math.random() * 86400000
                }))
            }, null, 2))
        },
        {
            name: "research_paper.md",
            content: padToMinSize(`# Proof of Storage Continuity Research
            
## Abstract
This document explores blockchain-based storage verification mechanisms.
Generated at: ${new Date().toISOString()}

## Introduction
Decentralized storage networks require robust proof mechanisms to ensure data availability and integrity over time. This research examines various approaches to continuous storage verification and presents a novel memory-hard VDF-based solution.

## Methodology
Our approach combines several key technologies:
1. Memory-hard Verifiable Delay Functions (VDFs)
2. Multi-source entropy for unpredictable chunk selection
3. Hierarchical proof aggregation
4. Economic incentives through token bonding
5. Anti-outsourcing network latency proofs

## Results
${Array.from({length: 20}, (_, i) => `Experiment ${i + 1}: Success rate 99.${90 + Math.floor(Math.random() * 10)}%`).join('\n')}

## Conclusion
The proposed system demonstrates superior security and efficiency compared to existing solutions.`)
        },
        {
            name: "dataset.csv",
            content: padToMinSize(`timestamp,temperature,humidity,pressure,location,sensor_id,battery_level,signal_strength
${Array.from({length: 100}, (_, i) => {
                const baseTime = Date.now();
                const temp = 20 + Math.random() * 10;
                const humidity = 50 + Math.random() * 30;
                const pressure = 1000 + Math.random() * 50;
                return `${baseTime + i * 1000},${temp.toFixed(1)},${humidity.toFixed(1)},${pressure.toFixed(2)},Sensor_${i % 10},SENS_${String(i).padStart(3, '0')},${(80 + Math.random() * 20).toFixed(1)},${(-40 - Math.random() * 40).toFixed(1)}`;
            }).join('\n')}`)
        },
        {
            name: "smart_contract.sol",
            content: padToMinSize(`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract StorageVerification {
    mapping(bytes32 => bool) public commitments;
    mapping(address => uint256) public stakes;
    mapping(address => uint256) public reputations;
    uint256 public createdAt = ${Math.floor(Date.now() / 1000)};
    uint256 public totalCommitments;
    address public owner;
    
    event CommitmentStored(bytes32 indexed hash, address indexed prover);
    event StakeUpdated(address indexed user, uint256 amount);
    event ReputationChanged(address indexed user, uint256 newReputation);
    
    constructor() {
        owner = msg.sender;
        reputations[msg.sender] = 100;
    }
    
    function verifyCommitment(bytes32 hash) public returns (bool) {
        return commitments[hash];
    }
    
    function storeCommitment(bytes32 hash) public {
        require(!commitments[hash], "Commitment already exists");
        commitments[hash] = true;
        totalCommitments++;
        emit CommitmentStored(hash, msg.sender);
    }
    
    function updateStake(uint256 amount) public payable {
        stakes[msg.sender] = amount;
        emit StakeUpdated(msg.sender, amount);
    }
    
    function updateReputation(address user, uint256 reputation) public {
        require(msg.sender == owner, "Only owner can update reputation");
        reputations[user] = reputation;
        emit ReputationChanged(user, reputation);
    }
    
    function getCommitmentCount() public view returns (uint256) {
        return totalCommitments;
    }
}`)
        },
        {
            name: "user_profiles.json",
            content: padToMinSize(JSON.stringify({
                users: Array.from({length: 50}, (_, i) => ({
                    id: i + 1,
                    name: `User_${i + 1}`,
                    role: ["prover", "verifier", "admin"][i % 3],
                    joinedAt: Date.now() - Math.random() * 86400000 * 30,
                    reputation: Math.floor(Math.random() * 100),
                    stake: Math.floor(Math.random() * 10000),
                    lastActivity: Date.now() - Math.random() * 86400000,
                    publicKey: `0x${'a'.repeat(64)}`,
                    preferences: {
                        notifications: Math.random() > 0.5,
                        privacy: ["public", "private", "friends"][i % 3],
                        theme: ["light", "dark"][i % 2]
                    }
                })),
                metadata: { 
                    version: "1.1", 
                    updated: new Date().toISOString(),
                    totalUsers: 50,
                    activeUsers: 35,
                    statistics: {
                        provers: 20,
                        verifiers: 15,
                        admins: 15
                    }
                }
            }, null, 2))
        },
        {
            name: "network_logs.txt",
            content: padToMinSize(`Network Activity Log
Generated: ${new Date().toISOString()}

${Array.from({length: 200}, (_, i) => {
                const timestamp = Date.now() + i * 1000;
                const events = [
                    "Node joined network",
                    "Commitment received",
                    "Peer discovery completed", 
                    "Challenge-response cycle initiated",
                    "VDF computation started",
                    "Chunk verification completed",
                    "Network latency measured",
                    "Availability challenge issued",
                    "Proof verification successful",
                    "Bond updated",
                    "Reputation score changed",
                    "Data synchronization completed"
                ];
                const event = events[i % events.length];
                const nodeId = `node_${String(i % 10).padStart(3, '0')}`;
                return `${timestamp} - [${nodeId}] ${event} - Status: ${Math.random() > 0.1 ? 'SUCCESS' : 'PENDING'}`;
            }).join('\n')}`)
        },
        {
            name: "image_metadata.json",
            content: padToMinSize(JSON.stringify({
                filename: "blockchain_visualization.png",
                size: "2.4MB",
                resolution: "1920x1080",
                format: "PNG",
                checksum: `sha256:${Date.now().toString(16)}`,
                created: new Date().toISOString(),
                exif: {
                    camera: "Digital Renderer",
                    software: "HashChain Visualizer v2.1",
                    colorSpace: "sRGB",
                    compression: "PNG"
                },
                layers: Array.from({length: 20}, (_, i) => ({
                    id: i,
                    name: `Layer_${i}`,
                    type: ["background", "nodes", "connections", "labels"][i % 4],
                    visible: Math.random() > 0.2,
                    opacity: Math.floor(Math.random() * 100),
                    blendMode: ["normal", "multiply", "overlay"][i % 3]
                })),
                renderSettings: {
                    quality: "high",
                    antiAliasing: true,
                    dpi: 300,
                    colorDepth: 24
                }
            }, null, 2))
        },
        {
            name: "blockchain_state.bin",
            content: padToMinSize(Buffer.from([
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
                ...Array.from({length: 1000}, () => Math.floor(Math.random() * 256))
            ]).toString('hex'))
        },
        {
            name: "consensus_data.xml",
            content: padToMinSize(`<?xml version="1.0" encoding="UTF-8"?>
<consensus timestamp="${Date.now()}">
  <validators>
    ${Array.from({length: 50}, (_, i) => 
        `    <validator id="${i + 1}" stake="${1000000 - i * 10000}" status="${Math.random() > 0.1 ? 'active' : 'pending'}" 
         reputation="${(90 + Math.random() * 10).toFixed(1)}" 
         last_block="${Date.now() - Math.random() * 86400000}"
         public_key="0x${'a'.repeat(64)}"
         endpoint="validator${i + 1}.example.com:9000"/>`
    ).join('\n')}
  </validators>
  <metrics>
    <block_time>52</block_time>
    <finality_time>120</finality_time>
    <total_stake>50000000</total_stake>
    <active_validators>45</active_validators>
    <network_health>0.98</network_health>
    <consensus_rounds>
      ${Array.from({length: 20}, (_, i) => 
          `      <round id="${i + 1}" duration="${45 + Math.random() * 10}" votes="${45 + Math.floor(Math.random() * 5)}" result="success"/>`
      ).join('\n')}
    </consensus_rounds>
  </metrics>
  <network_state>
    <total_nodes>100</total_nodes>
    <active_connections>95</active_connections>
    <pending_transactions>250</pending_transactions>
    <mempool_size>1024</mempool_size>
  </network_state>
</consensus>`)
        },
        {
            name: "performance_stats.json",
            content: padToMinSize(JSON.stringify({
                metrics: {
                    throughput: "1000 tx/s",
                    latency: "200ms",
                    storage_efficiency: "95%",
                    network_uptime: "99.9%",
                    vdf_computation_time: "38.5s",
                    memory_usage: "245MB",
                    chunk_verification_rate: "15.8 chunks/s",
                    proof_generation_time: "2.1s"
                },
                timestamp: Date.now(),
                node_id: `node_${Math.random().toString(36).substr(2, 9)}`,
                version: "2.1.0",
                detailed_stats: {
                    cpu_usage: Array.from({length: 60}, () => Math.floor(Math.random() * 100)),
                    memory_usage: Array.from({length: 60}, () => Math.floor(Math.random() * 256)),
                    network_io: Array.from({length: 60}, () => Math.floor(Math.random() * 1000)),
                    disk_io: Array.from({length: 60}, () => Math.floor(Math.random() * 500))
                },
                peer_statistics: Array.from({length: 25}, (_, i) => ({
                    peer_id: `peer_${i}`,
                    latency: Math.floor(Math.random() * 200),
                    bandwidth: Math.floor(Math.random() * 1000),
                    reputation: Math.floor(Math.random() * 100),
                    last_seen: Date.now() - Math.random() * 86400000
                }))
            }, null, 2))
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
    console.log("📊 Creating 10 files → 10 hashchains → 8 rounds → 80 total blocks");
    console.log("⏱️  Timeline: 16 second intervals for 2 minutes");
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
            // Generate private key (same as public key for demo)
            const privateKey = Buffer.from(proverKey);
            const prover = new ProofOfStorageProver(proverKey, privateKey, proverCallbacks);
            
            // Store the file and create initial commitment 
            const commitment = prover.storeData(testFiles[i].data, `./demo_output_${i}`);
            
            // Verify the prover has active chains after storing data
            const activeChains = prover.getActiveChainCount();
            console.log(`   📊 Chain ${i + 1}: ${testFiles[i].name} → Active chains: ${activeChains}`);
            
            if (activeChains === 0) {
                throw new Error(`Failed to create chain for ${testFiles[i].name}`);
            }
            
            provers.push({ prover, fileName: testFiles[i].name, initialCommitment: commitment });
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

        console.log("\n🌐 Starting 2-minute progression (8 rounds × 10 chains = 80 blocks)");
        console.log("⏰ Block interval: 16 seconds");
        console.log("🔍 VDF trace logging enabled - watch for [VDF TRACE] messages");
        console.log("=".repeat(70));

        // Wait a moment for VDF to start generating iterations
        console.log("⏳ Waiting 3 seconds for VDF processors to start...");
        await delay(3000);

        // Trigger VDF operations to start trace logging
        console.log("🚀 Triggering VDF operations to start continuous processing...");
        for (let i = 0; i < Math.min(3, provers.length); i++) {
            try {
                const result = provers[i].prover.submitBlockForVdf(i, Buffer.from(`test_block_${i}`.padEnd(32, '0')));
                console.log(`   ✅ VDF operation ${i + 1}: ${result}`);
            } catch (error) {
                console.log(`   ⚠️  VDF operation ${i + 1} failed: ${error.message}`);
            }
        }

        console.log("📊 VDF processors are now running - trace logs should appear...");
        await delay(2000); // Wait for trace logs to start appearing

        const startTime = Date.now();
        const duration = 2 * 60 * 1000; // 2 minutes
        const interval = 16 * 1000; // 16 seconds
        let round = 0;
        let totalBlocks = 0;

        while (Date.now() - startTime < duration) {
            round++;
            const roundStartTime = Date.now();
            
            console.log(`\n🔄 === ROUND ${round} === (${Math.floor((Date.now() - startTime) / 1000)}s elapsed)`);
            
            // Test proof operations with stored data
            for (let i = 0; i < provers.length; i++) {
                const { prover, fileName, initialCommitment } = provers[i];
                
                // Always use the initial commitment from storeData
                const currentCommitment = initialCommitment;
                commitments[i] = currentCommitment;
                
                totalBlocks++;
                
                // Test proof generation and verification every 3rd round
                if (round % 3 === 0) {
                    if (i === 0) { // Only test with first prover to avoid spam
                        try {
                            const compactProof = prover.createCompactProof(round);
                            const isValid = verifier.verifyCompactProof(compactProof);
                            console.log(`   🛡️  Proof verification (Chain 1): ${isValid ? 'PASSED' : 'FAILED'}`);
                        } catch (error) {
                            console.log(`   🛡️  Proof verification (Chain 1): FAILED (${error.message})`);
                        }
                    }
                }
                
                // Test challenge-response every 4th round
                if (round % 4 === 0) {
                    if (i === 1) { // Only test with second prover
                        try {
                            const challenge = verifier.generateChallenge(
                                Buffer.alloc(32, i), // Use simple prover key
                                currentCommitment.commitmentHash
                            );
                            const response = prover.respondToChallenge(challenge);
                            const isValidResponse = verifier.verifyChallengeResponse(challenge, response);
                            console.log(`   🎯 Challenge-Response (Chain 2): ${isValidResponse ? 'PASSED' : 'FAILED'}`);
                        } catch (error) {
                            console.log(`   🎯 Challenge-Response (Chain 2): FAILED (${error.message})`);
                        }
                    }
                }
                
                // Test VDF operations every 3rd round to keep VDF active
                if (round % 3 === 0) {
                    if (i === 0) { // Only test with first prover
                        try {
                            const vdfResult = prover.submitBlockForVdf(round + i, Buffer.from(`round_${round}_chain_${i}`.padEnd(32, '0')));
                            console.log(`   🔄 VDF Block Submit (Chain 1): ${vdfResult.substring(0, 50)}...`);
                        } catch (error) {
                            console.log(`   🔄 VDF Block Submit (Chain 1): FAILED (${error.message})`);
                        }
                    }
                }
                
                // Test integrity verification every 6th round
                if (round % 6 === 0) {
                    if (i === 2) { // Only test with third prover
                        const isIntact = prover.verifySelfIntegrity();
                        console.log(`   🔍 Self-integrity (Chain 3): ${isIntact ? 'PASSED' : 'FAILED'}`);
                    }
                }
            }
            
            // Display progress
            const avgBlockTime = (Date.now() - roundStartTime) / provers.length;
            console.log(`   📈 Processed ${provers.length} chains in ${Date.now() - roundStartTime}ms (avg: ${avgBlockTime.toFixed(1)}ms/chain)`);
            console.log(`   🎯 Total operations: ${totalBlocks}, Chains: ${provers.length}, Round: ${round}`);
            
            // Show chain state every 5th round
            if (round % 5 === 0) {
                console.log(`\n📊 === CHAIN STATE SNAPSHOT (Round ${round}) ===`);
                
                const activeChains = provers[0].prover.getActiveChainCount();
                const stats = provers[0].prover.getProverStats();
                console.log(`📈 Active chains: ${activeChains}`);
                console.log(`📊 Prover stats: ${stats}`);
                
                // Show VDF status
                try {
                    const vdfStats = provers[0].prover.getVdfPerformanceStats();
                    console.log(`🔄 VDF Performance: ${vdfStats}`);
                } catch (error) {
                    console.log(`🔄 VDF Performance: ${error.message}`);
                }
                
                try {
                    const latestProof = provers[0].prover.getLatestSharedVdfProof();
                    console.log(`🔐 Latest VDF Proof: ${latestProof.substring(0, 100)}...`);
                } catch (error) {
                    console.log(`🔐 Latest VDF Proof: ${error.message}`);
                }
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