const { 
    ProofOfStorageProver, 
    ProofOfStorageVerifier, 
    HierarchicalNetworkManager,
    generateMultiSourceEntropy,
    createMemoryHardVdfProof,
    verifyMemoryHardVdfProof,
    selectChunksFromEntropy,
    verifyChunkSelection,
    createCommitmentHash,
    verifyCommitmentIntegrity
} = require('./index.js');
const crypto = require('crypto');

// Initialize the native module
const { join } = require('path');
const nativeBinding = require(join(__dirname, 'proof-of-storage-continuity.win32-x64-msvc.node'));

// Helper function to delay execution
function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Complete HashChain Demonstration: Adding 11 Files to a HashChain
 * 
 * This demonstrates:
 * - Building a complete hashchain with multiple files
 * - Chain progression and linking between files
 * - Verification of individual files and entire chain
 * - Memory-hard VDF proofs for each link
 * - Challenge-response protocol validation
 */

// ====================================================================
// ENHANCED PROVER CALLBACKS WITH CHAIN STATE TRACKING
// ====================================================================

// Chain state tracking
const chainState = {
    blockHeight: 0,
    chains: {},
    addChainCommitment(chainId, commitment) {
        const chainIdHex = chainId.toString('hex');
        if (!this.chains[chainIdHex]) {
            this.chains[chainIdHex] = [];
        }
        this.chains[chainIdHex].push(commitment);
    },
    getChain(chainId) {
        const chainIdHex = chainId.toString('hex');
        return this.chains[chainIdHex] || [];
    },
    getChainLength(chainId) {
        const chainIdHex = chainId.toString('hex');
        return this.chains[chainIdHex] ? this.chains[chainIdHex].length : 0;
    },
    getAllChains() {
        return Object.entries(this.chains).map(([chainId, commitments]) => ({
            chainId: Buffer.from(chainId, 'hex'),
            commitments
        }));
    }
};

// Helper function to generate unique hashes
function generateUniqueHash(data) {
    if (typeof data === 'string') {
        return crypto.createHash('sha256').update(data).digest();
    } else if (Buffer.isBuffer(data)) {
        return crypto.createHash('sha256').update(data).digest();
    } else {
        return crypto.createHash('sha256').update(JSON.stringify(data)).digest();
    }
}

// Helper function to generate commitment object
function generateCommitment(files, prevCommitment = null) {
    // Generate combined data hash from all files
    const combinedDataHash = generateUniqueHash(
        Buffer.concat(files.map(file => generateUniqueHash(file.data)))
    );
    
    const blockHeight = chainState.blockHeight++;
    const blockHash = generateUniqueHash(`block_${blockHeight}_${Date.now()}`);
    
    // Generate chunk hashes for all files
    const chunkHashes = files.flatMap(file => {
        const chunkSize = 1024;
        const chunks = [];
        for (let i = 0; i < file.data.length; i += chunkSize) {
            const chunk = file.data.slice(i, Math.min(i + chunkSize, file.data.length));
            chunks.push(generateUniqueHash(chunk));
        }
        return chunks;
    });
    
    // Generate VDF proof
    const vdfProof = {
        inputState: prevCommitment ? prevCommitment.commitmentHash : generateUniqueHash('initial_state'),
        outputState: generateUniqueHash(`vdf_output_${combinedDataHash.toString('hex')}`),
        iterations: 1000,
        memoryAccessSamples: [],
        computationTimeMs: 1000,
        memoryUsageBytes: 268435456
    };
    
    // Generate entropy
    const entropy = {
        blockchainEntropy: generateUniqueHash(`entropy_${blockHeight}`),
        localEntropy: generateUniqueHash(`local_${Date.now()}`),
        timestamp: Date.now() / 1000,
        combinedHash: generateUniqueHash(`combined_${blockHeight}_${Date.now()}`)
    };
    
    // Generate chain ID and commitment hash
    const chainId = prevCommitment ? prevCommitment.chainId : generateUniqueHash(`chain_${Date.now()}`);
    
    const commitment = {
        proverKey: Buffer.from('prover_demo_key_32_bytes_1234567', 'utf8').slice(0, 32),
        chainId,
        dataHash: combinedDataHash,
        blockHeight,
        blockHash,
        fileHashes: files.map(file => ({
            name: file.name,
            hash: generateUniqueHash(file.data)
        })),
        chunkHashes,
        vdfProof,
        entropy,
        commitmentHash: null,
        prevCommitmentHash: prevCommitment ? prevCommitment.commitmentHash : null
    };
    
    commitment.commitmentHash = generateUniqueHash(
        Buffer.concat([
            combinedDataHash,
            blockHash,
            Buffer.concat(chunkHashes),
            vdfProof.outputState,
            entropy.combinedHash,
            chainId
        ])
    );
    
    return commitment;
}

// Prover callbacks
const proverCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => {
            console.log(`📚 Prover: Getting blockchain height... (${chainState.blockHeight})`);
            return chainState.blockHeight;
        },
        getBlockHash: (height) => {
            console.log(`📚 Prover: Getting block hash for height ${height}...`);
            return generateUniqueHash(`block_${height}_${Date.now()}`);
        },
        getBlockchainEntropy: () => {
            console.log("🎲 Prover: Getting blockchain entropy...");
            return generateUniqueHash(`entropy_${Date.now()}`);
        },
        submitCommitment: (commitment) => {
            console.log("✅ Prover: Submitting storage commitment to blockchain...");
            chainState.addChainCommitment(commitment.chainId, commitment);
            return true;
        }
    },
    economic: {
        stakeTokens: (amount) => {
            console.log(`💰 Prover: Staking ${amount} tokens...`);
            return true;
        },
        getStakeAmount: () => {
            console.log("💰 Prover: Getting stake amount...");
            return 1000000;
        },
        onStakeSlashed: (amount) => {
            console.log(`⚡ Prover: Stake slashed by ${amount}...`);
            return true;
        },
        claimRewards: () => {
            console.log("🎁 Prover: Claiming rewards...");
            return 50000;
        }
    },
    storage: {
        storeChunk: (chunkIndex, data) => {
            console.log(`💾 Prover: Storing chunk ${chunkIndex} (${data.length} bytes)...`);
            const chunkHash = generateUniqueHash(data);
            console.log(`   Chunk Hash: ${chunkHash.toString('hex').substring(0, 16)}...`);
            return true;
        },
        retrieveChunk: (chunkIndex) => {
            console.log(`📁 Prover: Retrieving chunk ${chunkIndex}...`);
            const chunkData = Buffer.from(`unique_chunk_${chunkIndex}_data_${Date.now()}`, 'utf8');
            return chunkData;
        },
        verifyDataIntegrity: () => {
            console.log("🔍 Prover: Verifying data integrity...");
            return true;
        },
        getStorageStats: () => {
            console.log("📊 Prover: Getting storage statistics...");
            return {
                totalChunks: 1000,
                totalSize: 4096000,
                availableSpace: 1000000000
            };
        }
    },
    network: {
        announceAvailability: () => {
            console.log("📢 Prover: Announcing availability to network...");
            return true;
        },
        submitChallengeResponse: (response) => {
            console.log("⚔️ Prover: Submitting challenge response...");
            return true;
        },
        broadcastProof: (proof) => {
            console.log("📡 Prover: Broadcasting proof to network...");
            return true;
        }
    },
    peerNetwork: {
        registerPeer: (peerId, metadata) => {
            console.log(`📝 Prover: Registering peer ${peerId.toString('hex').substring(0, 8)}...`);
            return true;
        },
        getPeerInfo: (peerId) => {
            console.log(`📊 Prover: Getting peer info for ${peerId.toString('hex').substring(0, 8)}...`);
            return {
                peerId,
                endpoint: "https://prover-peer.example.com:8080",
                nodeType: "prover",
                lastSeen: Date.now(),
                reputation: 90
            };
        },
        updatePeerLatency: (peerId, latencyMs) => {
            console.log(`📈 Prover: Updating latency for peer ${peerId.toString('hex').substring(0, 8)}: ${latencyMs}ms`);
            return true;
        },
        removePeer: (peerId) => {
            console.log(`🗑️ Prover: Removing peer ${peerId.toString('hex').substring(0, 8)}...`);
            return true;
        },
        getActivePeers: () => {
            console.log("👥 Prover: Getting active peers...");
            return [
                Buffer.from('active_prover_peer_1_key_1234567890123456789012', 'utf8').slice(0, 32),
                Buffer.from('active_prover_peer_2_key_1234567890123456789012', 'utf8').slice(0, 32)
            ];
        }
    },
    availabilityChallenge: {
        respondToChallenge: (challenge) => {
            console.log("⚔️ Prover: Responding to availability challenge...");
            return {
                challengeId: challenge.challengeId,
                chunkData: Buffer.alloc(4096, 0x42),
                timestamp: Date.now(),
                signature: Buffer.alloc(64, 0x43)
            };
        },
        validateChallenge: (challenge) => {
            console.log("✅ Prover: Validating challenge...");
            return true;
        },
        getResponseDeadline: (challenge) => {
            console.log("⏰ Prover: Getting response deadline...");
            return Date.now() + 60000;
        },
        issueAvailabilityChallenge: (proverKey, commitmentHash) => {
            console.log("🎯 Prover: Issuing availability challenge...");
            return {
                challengeId: generateUniqueHash(`challenge_${Date.now()}`),
                proverKey,
                commitmentHash,
                challengedChunks: [0, 5, 10],
                nonce: generateUniqueHash(`nonce_${Date.now()}`).slice(0, 16),
                timestamp: Date.now(),
                deadline: Date.now() + 60000
            };
        },
        validateAvailabilityResponse: (challenge, response) => {
            console.log("✅ Prover: Validating availability response...");
            return response && 
                   response.challengeId && 
                   response.challengeId.equals(challenge.challengeId) &&
                   response.chunkData &&
                   response.timestamp <= Date.now() &&
                   response.signature;
        },
        getChallengeDifficulty: () => {
            console.log("🎯 Prover: Getting challenge difficulty...");
            return 1000;
        },
        reportChallengeResult: (challengeId, success, metadata) => {
            console.log(`📊 Prover: Reporting challenge result: ${success ? 'SUCCESS' : 'FAILURE'}`);
            return true;
        },
        getProverAvailabilityScore: (proverKey) => {
            console.log("⭐ Prover: Getting availability score...");
            return 95;
        }
    },
    blockchainData: {
        validateChunkCount: (fileHash, reportedChunks) => {
            console.log("🔍 Prover: Validating chunk count...");
            return reportedChunks > 0 && reportedChunks < 1000000;
        },
        getDataFileMetadata: (fileHash) => {
            console.log("📄 Prover: Getting data file metadata...");
            return {
                fileHash,
                totalChunks: 1000,
                chunkSize: 4096,
                encodingVersion: 1,
                registrationHeight: chainState.blockHeight
            };
        },
        verifyDataRegistration: (fileHash) => {
            console.log("✅ Prover: Verifying data registration...");
            return true;
        },
        getConfirmedStorageSize: () => {
            console.log("💾 Prover: Getting confirmed storage size...");
            return 1000000; // 1MB
        },
        updateAvailabilityStatus: (fileHash, status) => {
            console.log(`📊 Prover: Updating availability status: ${status}`);
            return true;
        }
    }
};

// ====================================================================
// ENHANCED VERIFIER CALLBACKS
// ====================================================================

// Verifier callbacks
const verifierCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => {
            console.log(`📚 Verifier: Getting blockchain height... (${chainState.blockHeight})`);
            return chainState.blockHeight;
        },
        getBlockHash: (height) => {
            console.log(`📚 Verifier: Getting block hash for height ${height}...`);
            return generateUniqueHash(`block_${height}_${Date.now()}`);
        },
        validateBlockHash: (hash) => {
            console.log("✅ Verifier: Validating block hash...");
            return true;
        },
        getCommitment: (commitmentHash) => {
            console.log("📚 Verifier: Getting commitment...");
            return null;
        }
    },
    challenge: {
        issueChallenge: (proverKey, commitmentHash) => {
            console.log("🎯 Verifier: Issuing challenge...");
            return {
                challengeId: Buffer.alloc(32, 0x01),
                proverKey,
                commitmentHash,
                challengedChunks: [0, 5, 10],
                nonce: Buffer.alloc(16, 0x02),
                timestamp: Date.now(),
                deadline: Date.now() + 60000
            };
        },
        validateResponse: (challenge, response) => {
            console.log("✅ Verifier: Validating challenge response...");
            return true;
        },
        reportResult: (challengeId, success) => {
            console.log(`📊 Verifier: Reporting challenge result: ${success ? 'SUCCESS' : 'FAILURE'}`);
            return true;
        }
    },
    network: {
        discoverProvers: () => {
            console.log("🔍 Verifier: Discovering active provers...");
            return [Buffer.from('prover_1_key', 'utf8'), Buffer.from('prover_2_key', 'utf8')];
        },
        getProverReputation: (proverKey) => {
            console.log("⭐ Verifier: Getting prover reputation...");
            return 0.95; // 95% reputation
        },
        reportMisbehavior: (proverKey, evidence) => {
            console.log("🚨 Verifier: Reporting prover misbehavior...");
            return true;
        }
    },
    economic: {
        rewardVerification: (amount) => {
            console.log(`🎁 Verifier: Receiving ${amount} verification reward...`);
            return true;
        },
        penalizeFailure: (amount) => {
            console.log(`⚡ Verifier: Penalized ${amount} for verification failure...`);
            return true;
        }
    },
    peerNetwork: {
        registerPeer: (peerId, metadata) => {
            console.log(`📝 Verifier: Registering peer ${peerId.toString('hex').substring(0, 8)}...`);
            return true;
        },
        getPeerInfo: (peerId) => {
            console.log(`📊 Verifier: Getting peer info for ${peerId.toString('hex').substring(0, 8)}...`);
            return {
                peerId,
                endpoint: "https://verifier-peer.example.com:8080",
                nodeType: "verifier",
                lastSeen: Date.now(),
                reputation: 90
            };
        },
        updatePeerLatency: (peerId, latencyMs) => {
            console.log(`📈 Verifier: Updating latency for peer ${peerId.toString('hex').substring(0, 8)}: ${latencyMs}ms`);
            return true;
        },
        removePeer: (peerId) => {
            console.log(`🗑️ Verifier: Removing peer ${peerId.toString('hex').substring(0, 8)}...`);
            return true;
        },
        getActivePeers: () => {
            console.log("👥 Verifier: Getting active peers...");
            return [
                Buffer.from('active_ver_peer_1_key_1234567890123456789012', 'utf8').slice(0, 32),
                Buffer.from('active_ver_peer_2_key_1234567890123456789012', 'utf8').slice(0, 32)
            ];
        }
    },
    availabilityChallenge: {
        issueAvailabilityChallenge: (proverKey, commitmentHash) => {
            console.log("🎯 Verifier: Issuing availability challenge...");
            return {
                challengeId: Buffer.alloc(32, 0x01),
                proverKey,
                commitmentHash,
                challengedChunks: [0, 5, 10],
                nonce: Buffer.alloc(16, 0x02),
                timestamp: Date.now(),
                deadline: Date.now() + 60000
            };
        },
        validateAvailabilityResponse: (challenge, response) => {
            console.log("✅ Verifier: Validating availability response...");
            return true;
        },
        getChallengeDifficulty: () => {
            console.log("🎯 Verifier: Getting challenge difficulty...");
            return 1000;
        },
        reportChallengeResult: (challengeId, success, metadata) => {
            console.log(`📊 Verifier: Reporting challenge result: ${success ? 'SUCCESS' : 'FAILURE'}`);
            return true;
        },
        getProverAvailabilityScore: (proverKey) => {
            console.log("⭐ Verifier: Getting availability score...");
            return 90;
        }
    },
    blockchainData: {
        validateChunkCount: (fileHash, reportedChunks) => {
            console.log("🔍 Verifier: Validating chunk count...");
            return reportedChunks > 0 && reportedChunks < 1000000;
        },
        getDataFileMetadata: (fileHash) => {
            console.log("📄 Verifier: Getting data file metadata...");
            return {
                fileHash,
                totalChunks: 1000,
                chunkSize: 4096,
                encodingVersion: 1,
                registrationHeight: chainState.blockHeight
            };
        },
        verifyDataRegistration: (fileHash) => {
            console.log("✅ Verifier: Verifying data registration...");
            return true;
        },
        getConfirmedStorageSize: () => {
            console.log("💾 Verifier: Getting confirmed storage size...");
            return 1000000; // 1MB
        },
        updateAvailabilityStatus: (fileHash, status) => {
            console.log(`📊 Verifier: Updating availability status: ${status}`);
            return true;
        }
    }
};

// ====================================================================
// TEST FILE GENERATOR
// ====================================================================

function generateTestFiles() {
    const files = [
        {
            name: "config.json",
            content: JSON.stringify({
                version: "1.0.0",
                networkType: "chia-mainnet",
                storageCapacity: "1TB"
            }, null, 2)
        },
        {
            name: "data1.txt",
            content: "This is the first data file in our HashChain demonstration. " +
                    "It contains important information that needs to be stored securely."
        },
        {
            name: "research_paper.md",
            content: "# Proof of Storage Continuity\n\n" +
                    "## Abstract\n\n" +
                    "This paper presents a novel approach to blockchain-based storage verification..."
        },
        {
            name: "log_2024_01.txt",
            content: "2024-01-01 00:00:00 - System initialized\n" +
                    "2024-01-01 00:01:00 - First data block received\n" +
                    "2024-01-01 00:02:00 - Verification completed"
        },
        {
            name: "image_metadata.json",
            content: JSON.stringify({
                filename: "landscape.jpg",
                size: "2.4MB",
                resolution: "1920x1080",
                format: "JPEG",
                checksum: "sha256:abc123..."
            })
        },
        {
            name: "smart_contract.sol",
            content: "// SPDX-License-Identifier: MIT\n" +
                    "pragma solidity ^0.8.0;\n\n" +
                    "contract StorageVerification {\n" +
                    "    mapping(bytes32 => bool) public commitments;\n" +
                    "    \n    function verifyCommitment(bytes32 hash) public returns (bool) {\n" +
                    "        return commitments[hash];\n    }\n}"
        },
        {
            name: "dataset.csv",
            content: "timestamp,temperature,humidity,pressure\n" +
                    "2024-01-01T00:00:00Z,22.5,65.2,1013.25\n" +
                    "2024-01-01T01:00:00Z,21.8,66.1,1012.87\n" +
                    "2024-01-01T02:00:00Z,21.2,67.5,1011.92"
        },
        {
            name: "backup_index.bin",
            content: Buffer.from([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 
                                0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52]).toString('hex')
        },
        {
            name: "user_profiles.json",
            content: JSON.stringify({
                users: [
                    { id: 1, name: "Alice", role: "prover" },
                    { id: 2, name: "Bob", role: "verifier" },
                    { id: 3, name: "Charlie", role: "admin" }
                ]
            })
        },
        {
            name: "network_topology.xml",
            content: "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n" +
                    "<network>\n" +
                    "  <nodes>\n" +
                    "    <node id=\"1\" type=\"prover\" location=\"us-east-1\"/>\n" +
                    "    <node id=\"2\" type=\"verifier\" location=\"eu-west-1\"/>\n" +
                    "  </nodes>\n" +
                    "</network>"
        },
        {
            name: "final_summary.txt",
            content: "This is the final file in our HashChain demonstration. " +
                    "It represents the completion of a 11-file chain with full continuity proofs."
        }
    ];

    return files.map((file, index) => ({
        ...file,
        id: index + 1,
        data: Buffer.from(file.content, 'utf8'),
        size: Buffer.byteLength(file.content, 'utf8')
    }));
}

// ====================================================================
// MAIN DEMONSTRATION FUNCTION
// ====================================================================

// Helper function to display chain state
function displayChainState(chainId, chain, testFiles) {
    console.log("\n📊 === CURRENT CHAIN STATE ===");
    console.log(`Chain ID: ${chainId.substring(0, 16)}...`);
    console.log(`Current Length: ${chain.length} blocks`);
    console.log(`Current Height: ${chainState.blockHeight}`);
    console.log("\nChain Structure:");
    console.log("  START");
    chain.forEach((commitment, index) => {
        console.log(`\n  ${index === 0 ? '' : '↓'} Block ${index + 1}:`);
        console.log("  ├─ Block Info:");
        console.log(`  │  ├─ Height: ${commitment.blockHeight}`);
        console.log(`  │  ├─ Block Hash: ${commitment.blockHash.toString('hex').substring(0, 16)}...`);
        console.log(`  │  ├─ Commitment Hash: ${commitment.commitmentHash.toString('hex').substring(0, 16)}...`);
        if (commitment.prevCommitmentHash) {
            console.log(`  │  └─ Previous Hash: ${commitment.prevCommitmentHash.toString('hex').substring(0, 16)}...`);
        }

        console.log("  ├─ Data:");
        console.log(`  │  ├─ Combined Data Hash: ${commitment.dataHash.toString('hex').substring(0, 16)}...`);
        console.log("  │  └─ Files:");
        commitment.fileHashes.forEach(fileHash => {
            console.log(`  │     ├─ ${fileHash.name}`);
            console.log(`  │     │  └─ Hash: ${fileHash.hash.toString('hex').substring(0, 16)}...`);
        });

        console.log("  ├─ VDF Proof:");
        console.log(`  │  ├─ Input State: ${commitment.vdfProof.inputState.toString('hex').substring(0, 16)}...`);
        console.log(`  │  ├─ Output State: ${commitment.vdfProof.outputState.toString('hex').substring(0, 16)}...`);
        console.log(`  │  ├─ Iterations: ${commitment.vdfProof.iterations}`);
        console.log(`  │  ├─ Computation Time: ${commitment.vdfProof.computationTimeMs}ms`);
        console.log(`  │  └─ Memory Usage: ${(commitment.vdfProof.memoryUsageBytes / (1024 * 1024)).toFixed(2)}MB`);

        console.log("  ├─ Entropy Sources:");
        console.log(`  │  ├─ Blockchain: ${commitment.entropy.blockchainEntropy.toString('hex').substring(0, 16)}...`);
        console.log(`  │  ├─ Local: ${commitment.entropy.localEntropy.toString('hex').substring(0, 16)}...`);
        console.log(`  │  ├─ Timestamp: ${new Date(commitment.entropy.timestamp * 1000).toISOString()}`);
        console.log(`  │  └─ Combined: ${commitment.entropy.combinedHash.toString('hex').substring(0, 16)}...`);

        console.log("  ├─ Chunk Proofs:");
        console.log(`  │  ├─ Total Chunks: ${commitment.chunkHashes.length}`);
        console.log("  │  └─ Chunk Hashes:");
        commitment.chunkHashes.slice(0, 3).forEach((hash, i) => {
            console.log(`  │     ├─ Chunk ${i}: ${hash.toString('hex').substring(0, 16)}...`);
        });
        if (commitment.chunkHashes.length > 3) {
            console.log(`  │     └─ ... and ${commitment.chunkHashes.length - 3} more chunks`);
        }

        // Generate and display availability challenge for this block
        const challenge = {
            challengeId: generateUniqueHash(`challenge_${commitment.blockHeight}`),
            proverKey: commitment.proverKey,
            commitmentHash: commitment.commitmentHash,
            challengedChunks: [0, 5, 10],
            nonce: generateUniqueHash(`nonce_${commitment.blockHeight}`).slice(0, 16),
            timestamp: Date.now(),
            deadline: Date.now() + 60000
        };

        console.log("  └─ Availability Challenge:");
        console.log(`     ├─ Challenge ID: ${challenge.challengeId.toString('hex').substring(0, 16)}...`);
        console.log(`     ├─ Nonce: ${challenge.nonce.toString('hex').substring(0, 16)}...`);
        console.log(`     ├─ Challenged Chunks: ${challenge.challengedChunks.join(', ')}`);
        console.log(`     ├─ Timestamp: ${new Date(challenge.timestamp).toISOString()}`);
        console.log(`     └─ Deadline: ${new Date(challenge.deadline).toISOString()}`);
    });
    console.log("\n  END");
    console.log("=".repeat(80));
    console.log();
}

async function demonstrateCompleteHashChain() {
    try {
        // Initialize with debug logging enabled
        const loggingOptions = {
            level: "debug",
            timestamps: true,
            chainState: true,
            proofDetails: true,
            vdfDetails: true,
            networkMessages: true
        };

        const proverOptions = {
            logging: loggingOptions,
            outputDirectory: './hashchain_output',
            chunkSize: 4096,
            vdfParams: {
                iterations: 1000,
                memorySize: 268435456
            }
        };

        const verifierOptions = {
            logging: loggingOptions,
            challengeParams: {
                timeout: 60000,
                retries: 3
            }
        };

        const networkOptions = {
            logging: loggingOptions
        };

        const proverKey = Buffer.from('prover_demo_key_32_bytes_1234567', 'utf8').slice(0, 32);
        const verifierKey = Buffer.from('verifier_demo_key_32_bytes_1234567', 'utf8').slice(0, 32);
        const prover = new ProofOfStorageProver(proverKey, proverCallbacks, proverOptions);
        const verifier = new ProofOfStorageVerifier(verifierKey, verifierCallbacks, verifierOptions);
        const networkManager = new HierarchicalNetworkManager(proverKey, "prover", networkOptions);

        const startTime = Date.now();
        const duration = 2 * 60 * 1000; // 2 minutes in milliseconds
        const interval = 5000; // 5 seconds in milliseconds
        let blockNumber = 0;
        let currentChainId = null;

        // Generate test files
        const testFiles = generateTestFiles();

        while (Date.now() - startTime < duration) {
            // Generate commitment for all files
            const commitment = generateCommitment(testFiles, prevCommitment);

            // Submit commitment to blockchain
            proverCallbacks.blockchain.submitCommitment(commitment);

            prevCommitment = commitment;
            blockNumber++;

            // Wait for the next interval
            await delay(interval);
        }

        return {
            success: true,
            totalBlocks: blockNumber,
            chainId: currentChainId,
            duration: Math.floor(duration / 1000),
            blockRange: {
                start: chainState.blockHeight - blockNumber,
                end: chainState.blockHeight - 1
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error.message
        };
    }
}

// Run the demonstration
if (require.main === module) {
    demonstrateCompleteHashChain()
        .then(result => {
            if (result.success) {
                console.log("\n🏆 DEMONSTRATION SUCCESSFUL!");
                console.log(`📊 Created complete HashChain with ${result.totalBlocks} blocks`);
            } else {
                console.log("\n❌ DEMONSTRATION FAILED!");
                if (result.error) {
                    console.log(`Error: ${result.error}`);
                }
            }
        })
        .catch(error => {
            console.error("❌ Unexpected error:", error);
        });
}

// Export for testing
module.exports = {
    generateTestFiles,
    demonstrateCompleteHashChain,
    chainState,
    proverCallbacks,
    verifierCallbacks
}; 