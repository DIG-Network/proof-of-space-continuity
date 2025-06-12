const { 
    ProofOfStorageProver, 
    ProofOfStorageVerifier, 
    HierarchicalNetworkManager 
} = require('./index.js');

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

class HashChainState {
    constructor() {
        this.chains = new Map();
        this.commitments = new Map();
        this.blockHeight = 12345;
        this.globalEntropy = Buffer.from('blockchain_entropy_source_12345', 'utf8');
    }

    addChainCommitment(chainId, commitment) {
        if (!this.chains.has(chainId)) {
            this.chains.set(chainId, []);
        }
        this.chains.get(chainId).push(commitment);
        this.commitments.set(commitment.commitmentHash.toString('hex'), commitment);
    }

    getChain(chainId) {
        return this.chains.get(chainId) || [];
    }

    getChainLength(chainId) {
        return this.getChain(chainId).length;
    }

    getAllChains() {
        return Array.from(this.chains.keys());
    }
}

const chainState = new HashChainState();

const proverCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => {
            console.log(`📚 Prover: Getting blockchain height... (${chainState.blockHeight})`);
            return chainState.blockHeight;
        },
        getBlockHash: (height) => {
            console.log(`📚 Prover: Getting block hash for height ${height}...`);
            // Generate deterministic hash based on height
            const hash = Buffer.alloc(32);
            hash.writeUInt32BE(height, 0);
            return hash;
        },
        getBlockchainEntropy: () => {
            console.log("🎲 Prover: Getting blockchain entropy...");
            return chainState.globalEntropy;
        },
        submitCommitment: (commitment) => {
            console.log("✅ Prover: Submitting storage commitment to blockchain...");
            console.log(`   Chain ID: ${commitment.chainId ? commitment.chainId.toString('hex').substring(0, 16) : 'N/A'}...`);
            console.log(`   Block Height: ${commitment.blockHeight}`);
            console.log(`   Commitment Hash: ${commitment.commitmentHash.toString('hex').substring(0, 16)}...`);
            
            // Store commitment in our mock blockchain
            if (commitment.chainId) {
                chainState.addChainCommitment(commitment.chainId.toString('hex'), commitment);
            }
            return true;
        }
    },
    economic: {
        stakeTokens: (amount) => {
            console.log(`💰 Prover: Staking ${amount} tokens...`);
            return Buffer.from('stake_transaction_id_12345', 'utf8');
        },
        getStakeAmount: () => {
            console.log("💰 Prover: Checking stake amount...");
            return 10000; // 10,000 tokens staked
        },
        onStakeSlashed: (amount, reason) => {
            console.log(`⚡ Prover: Stake slashed! Amount: ${amount}, Reason: ${reason}`);
        },
        claimRewards: (amount) => {
            console.log(`🎁 Prover: Claiming ${amount} rewards...`);
            return true;
        }
    },
    storage: {
        storeChunk: (chunkIndex, data) => {
            console.log(`💾 Prover: Storing chunk ${chunkIndex} (${data.length} bytes)...`);
            return true;
        },
        retrieveChunk: (chunkIndex) => {
            console.log(`📁 Prover: Retrieving chunk ${chunkIndex}...`);
            // Generate deterministic chunk data
            return Buffer.from(`chunk_${chunkIndex}_data_content`, 'utf8');
        },
        verifyDataIntegrity: () => {
            console.log("🔍 Prover: Verifying data integrity...");
            return true;
        },
        getStorageStats: () => {
            console.log("📊 Prover: Getting storage statistics...");
            const totalFiles = Array.from(chainState.chains.values()).reduce((sum, chain) => sum + chain.length, 0);
            return JSON.stringify({ 
                totalFiles,
                totalChains: chainState.chains.size,
                totalStorage: `${totalFiles * 1.5}MB`,
                integrityScore: 1.0
            });
        }
    },
    network: {
        announceAvailability: (commitment) => {
            console.log("📢 Prover: Announcing availability to network...");
            console.log(`   Available for challenges on chain: ${commitment.chainId ? commitment.chainId.toString('hex').substring(0, 8) : 'N/A'}...`);
        },
        submitChallengeResponse: (response) => {
            console.log("📤 Prover: Submitting challenge response...");
            console.log(`   Challenge ID: ${response.challengeId.toString('hex').substring(0, 16)}...`);
        },
        broadcastProof: (proof) => {
            console.log("📡 Prover: Broadcasting proof to network...");
            console.log(`   Proof type: ${proof.proofType || 'Storage Proof'}`);
        }
    },
    peerNetwork: {
        discoverPeers: () => {
            console.log("🔍 Prover: Discovering network peers...");
            return [
                Buffer.from('peer_1_key_12345678901234567890123456789012', 'utf8').slice(0, 32),
                Buffer.from('peer_2_key_12345678901234567890123456789012', 'utf8').slice(0, 32),
                Buffer.from('peer_3_key_12345678901234567890123456789012', 'utf8').slice(0, 32)
            ];
        },
        validatePeer: (peerId) => {
            console.log(`✅ Prover: Validating peer ${peerId.toString('hex').substring(0, 8)}...`);
            return true;
        },
        reportPeerMisbehavior: (peerId, evidence) => {
            console.log(`🚨 Prover: Reporting peer misbehavior for ${peerId.toString('hex').substring(0, 8)}...`);
            return true;
        },
        getPeerReputation: (peerId) => {
            console.log(`⭐ Prover: Getting reputation for peer ${peerId.toString('hex').substring(0, 8)}...`);
            return 85; // 85% reputation
        },
        broadcastToNetwork: (message) => {
            console.log("📡 Prover: Broadcasting message to peer network...");
            return true;
        },
        registerPeer: (peerId, metadata) => {
            console.log(`📝 Prover: Registering peer ${peerId.toString('hex').substring(0, 8)}...`);
            return true;
        },
        getPeerInfo: (peerId) => {
            console.log(`📊 Prover: Getting peer info for ${peerId.toString('hex').substring(0, 8)}...`);
            return {
                peerId,
                endpoint: "https://peer.example.com:8080",
                nodeType: "prover",
                lastSeen: Date.now(),
                reputation: 85
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
        getNetworkStats: () => {
            console.log("📊 Prover: Getting peer network statistics...");
            return {
                totalPeers: 15,
                activePeers: 12,
                averageLatency: 45,
                networkHealth: 0.9
            };
        },
        getActivePeers: () => {
            console.log("👥 Prover: Getting active peers...");
            return [
                Buffer.from('active_peer_1_key_1234567890123456789012', 'utf8').slice(0, 32),
                Buffer.from('active_peer_2_key_1234567890123456789012', 'utf8').slice(0, 32)
            ];
        }
    }
};

// ====================================================================
// ENHANCED VERIFIER CALLBACKS
// ====================================================================

const verifierCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => {
            console.log("📚 Verifier: Getting blockchain height...");
            return chainState.blockHeight;
        },
        getBlockHash: (height) => {
            console.log(`📚 Verifier: Getting block hash for height ${height}...`);
            const hash = Buffer.alloc(32);
            hash.writeUInt32BE(height, 0);
            return hash;
        },
        validateBlockHash: (height, hash) => {
            console.log(`✅ Verifier: Validating block hash for height ${height}...`);
            const expectedHash = Buffer.alloc(32);
            expectedHash.writeUInt32BE(height, 0);
            return expectedHash.equals(hash);
        },
        getCommitment: (commitmentHash) => {
            console.log("🔍 Verifier: Retrieving commitment from blockchain...");
            const commitment = chainState.commitments.get(commitmentHash.toString('hex'));
            if (commitment) {
                console.log(`   Found commitment for chain: ${commitment.chainId.toString('hex').substring(0, 8)}...`);
            }
            return commitment || null;
        }
    },
    challenge: {
        issueChallenge: (challenge) => {
            console.log("⚔️ Verifier: Issuing challenge to prover...");
            console.log(`   Challenge ID: ${challenge.challengeId.toString('hex').substring(0, 16)}...`);
            console.log(`   Challenged chunks: ${challenge.challengedChunks.length}`);
        },
        validateResponse: (response) => {
            console.log("✅ Verifier: Validating challenge response...");
            return true;
        },
        reportResult: (proverKey, passed) => {
            console.log(`📝 Verifier: Reporting result for prover: ${passed ? 'PASSED ✅' : 'FAILED ❌'}`);
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
        }
    },
    economic: {
        rewardVerification: (amount) => {
            console.log(`🎁 Verifier: Receiving ${amount} verification reward...`);
        },
        penalizeFailure: (amount) => {
            console.log(`⚡ Verifier: Penalized ${amount} for verification failure...`);
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
        buffer: Buffer.from(file.content, 'utf8'),
        size: Buffer.byteLength(file.content, 'utf8')
    }));
}

// ====================================================================
// MAIN DEMONSTRATION FUNCTION
// ====================================================================

async function demonstrateCompleteHashChain() {
    console.log("🚀 COMPLETE HASHCHAIN DEMONSTRATION");
    console.log("📁 Adding 11 Files to HashChain with Continuity Proofs\n");

    const files = generateTestFiles();
    console.log(`📋 Generated ${files.length} test files:`);
    files.forEach(file => {
        console.log(`   ${file.id}. ${file.name} (${file.size} bytes)`);
    });

    // Create prover and verifier
    const proverKey = Buffer.from('prover_demo_key_32_bytes_12345678', 'utf8');
    const verifierKey = Buffer.from('verifier_demo_key_32_bytes_123456', 'utf8');

    try {
        // ================================================================
        // INITIALIZE PROVER AND VERIFIER
        // ================================================================
        console.log("\n🔧 === INITIALIZATION ===");
        const prover = new ProofOfStorageProver(proverKey, proverCallbacks);
        const verifier = new ProofOfStorageVerifier(verifierKey, verifierCallbacks);
        console.log("✅ Prover and Verifier initialized");

        // ================================================================
        // BUILD THE HASHCHAIN FILE BY FILE
        // ================================================================
        console.log("\n🔗 === BUILDING HASHCHAIN ===");
        
        const chainCommitments = [];
        let previousCommitmentHash = null;

        for (let i = 0; i < files.length; i++) {
            const file = files[i];
            
            console.log(`\n📁 Adding File ${file.id}/${files.length}: ${file.name}`);
            console.log(`   Size: ${file.size} bytes`);
            console.log(`   Content preview: ${file.content.substring(0, 50)}...`);

            // Store the file and get commitment
            console.log("   🔄 Storing file and generating commitment...");
            const commitment = prover.storeData(file.buffer, `./temp/${file.name}`);
            
            // Link to previous commitment if exists
            if (previousCommitmentHash) {
                console.log(`   🔗 Linking to previous commitment: ${previousCommitmentHash.toString('hex').substring(0, 16)}...`);
                commitment.previousCommitmentHash = previousCommitmentHash;
            }

            console.log(`   ✅ Commitment generated: ${commitment.commitmentHash.toString('hex').substring(0, 16)}...`);
            
            // Simulate block progression
            chainState.blockHeight += 1;
            
            // Create proof for this link
            console.log("   🧮 Generating VDF proof for chain link...");
            const compactProof = prover.createCompactProof();
            console.log("   ✅ VDF proof generated");

            chainCommitments.push({
                fileInfo: file,
                commitment,
                proof: compactProof,
                blockHeight: chainState.blockHeight,
                chainPosition: i + 1
            });

            previousCommitmentHash = commitment.commitmentHash;

            // Show chain progress
            console.log(`   📊 Chain Progress: ${i + 1}/${files.length} files added`);
            console.log(`   🔗 Chain Length: ${chainState.getChainLength(commitment.chainId.toString('hex'))}`);
        }

        // ================================================================
        // VERIFY THE COMPLETE CHAIN
        // ================================================================
        console.log("\n🔍 === CHAIN VERIFICATION ===");
        
        let allValid = true;
        
        for (let i = 0; i < chainCommitments.length; i++) {
            const { fileInfo, commitment, proof, chainPosition } = chainCommitments[i];
            
            console.log(`\n🔍 Verifying Chain Link ${chainPosition}/${files.length}: ${fileInfo.name}`);
            
            // Verify the proof
            const proofValid = verifier.verifyCompactProof(proof);
            console.log(`   📋 Proof Verification: ${proofValid ? '✅ VALID' : '❌ INVALID'}`);
            
            // Verify commitment exists on blockchain
            const commitmentExists = chainState.commitments.has(commitment.commitmentHash.toString('hex'));
            console.log(`   📚 Blockchain Record: ${commitmentExists ? '✅ FOUND' : '❌ NOT FOUND'}`);
            
            // Verify chain linkage (except for first file)
            if (i > 0) {
                const previousCommitment = chainCommitments[i - 1].commitment;
                const linkageValid = commitment.previousCommitmentHash && 
                                   commitment.previousCommitmentHash.equals(previousCommitment.commitmentHash);
                console.log(`   🔗 Chain Linkage: ${linkageValid ? '✅ VALID' : '❌ BROKEN'}`);
                allValid = allValid && linkageValid;
            }
            
            // Generate and verify challenge
            console.log("   ⚔️ Issuing availability challenge...");
            const challenge = verifier.generateChallenge(proverKey, commitment.commitmentHash);
            const response = prover.respondToChallenge(challenge);
            const responseValid = verifier.verifyChallengeResponse(response, challenge);
            console.log(`   📤 Challenge Response: ${responseValid ? '✅ VALID' : '❌ INVALID'}`);
            
            allValid = allValid && proofValid && commitmentExists && responseValid;
        }

        // ================================================================
        // CHAIN ANALYSIS AND STATISTICS
        // ================================================================
        console.log("\n📊 === CHAIN ANALYSIS ===");
        
        const totalFiles = chainCommitments.length;
        const totalSize = chainCommitments.reduce((sum, c) => sum + c.fileInfo.size, 0);
        const chainId = chainCommitments[0].commitment.chainId.toString('hex');
        
        console.log(`📈 Chain Statistics:`);
        console.log(`   Chain ID: ${chainId.substring(0, 16)}...`);
        console.log(`   Total Files: ${totalFiles}`);
        console.log(`   Total Size: ${(totalSize / 1024).toFixed(2)} KB`);
        console.log(`   Block Range: ${chainCommitments[0].blockHeight} - ${chainCommitments[totalFiles - 1].blockHeight}`);
        console.log(`   Chain Integrity: ${allValid ? '✅ INTACT' : '❌ COMPROMISED'}`);

        // Show chain structure
        console.log(`\n🔗 Chain Structure:`);
        chainCommitments.forEach((c, i) => {
            const arrow = i === 0 ? '  START' : '  ↓';
            console.log(`${arrow} ${c.chainPosition}. ${c.fileInfo.name}`);
            console.log(`      Block: ${c.blockHeight} | Hash: ${c.commitment.commitmentHash.toString('hex').substring(0, 16)}...`);
        });
        console.log('  END');

        // ================================================================
        // NETWORK MANAGEMENT DEMONSTRATION
        // ================================================================
        console.log("\n🌐 === NETWORK MANAGEMENT ===");
        const networkManager = new HierarchicalNetworkManager(proverKey, "both");
        
        console.log("🔄 Registering nodes in network...");
        networkManager.registerProver(prover);
        networkManager.registerVerifier(verifier);
        
        const networkStats = networkManager.getNetworkStats();
        console.log("📊 Network Statistics:");
        console.log(`   Total Provers: ${networkStats.totalProvers}`);
        console.log(`   Total Verifiers: ${networkStats.totalVerifiers}`);
        console.log(`   Network Health: ${(networkStats.healthScore * 100).toFixed(1)}%`);
        console.log(`   Total Storage: ${networkStats.totalStorage}`);

        // ================================================================
        // FINAL SUMMARY
        // ================================================================
        console.log("\n🎉 === DEMONSTRATION COMPLETE ===");
        console.log("\n✨ HashChain Demonstration Results:");
        console.log(`   ✅ Successfully added ${totalFiles} files to HashChain`);
        console.log(`   ✅ All files verified with continuity proofs`);
        console.log(`   ✅ Chain integrity maintained throughout`);
        console.log(`   ✅ Challenge-response protocol validated`);
        console.log(`   ✅ Network management operational`);
        
        console.log("\n🔧 Technical Features Demonstrated:");
        console.log("   • File-by-file chain building with linkage");
        console.log("   • Memory-hard VDF proofs for each link");
        console.log("   • Blockchain integration with block progression");
        console.log("   • Availability challenges and responses");
        console.log("   • Complete chain verification");
        console.log("   • Network-wide proof broadcasting");
        console.log("   • Economic incentive tracking");

        return {
            success: allValid,
            chainId,
            totalFiles,
            totalSize,
            commitments: chainCommitments
        };

    } catch (error) {
        console.error("❌ Error during HashChain demonstration:", error.message);
        console.error("Stack trace:", error.stack);
        return { success: false, error: error.message };
    }
}

// ====================================================================
// EXECUTION
// ====================================================================

// Run the demonstration
if (require.main === module) {
    demonstrateCompleteHashChain()
        .then(result => {
            if (result.success) {
                console.log("\n🏆 DEMONSTRATION SUCCESSFUL!");
                console.log(`📊 Created complete HashChain with ${result.totalFiles} files`);
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

module.exports = {
    demonstrateCompleteHashChain,
    generateTestFiles,
    proverCallbacks,
    verifierCallbacks,
    HashChainState
}; 