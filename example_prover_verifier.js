const { 
    ProofOfStorageProver, 
    ProofOfStorageVerifier, 
    HierarchicalNetworkManager 
} = require('./index.js');

/**
 * Example: Clean Prover/Verifier Interface Separation
 * 
 * This demonstrates the new blockchain-agnostic design with clear separation:
 * - ProofOfStorageProver: Handles data storage and proof generation
 * - ProofOfStorageVerifier: Handles proof verification and challenge generation  
 * - HierarchicalNetworkManager: Manages the network of provers/verifiers
 */

// ====================================================================
// PROVER CALLBACKS (Blockchain-Agnostic)
// ====================================================================

const proverCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => {
            console.log("📚 Prover: Getting blockchain height...");
            return 12345; // Could be any blockchain
        },
        getBlockHash: (height) => {
            console.log(`📚 Prover: Getting block hash for height ${height}...`);
            return Buffer.from('a'.repeat(64), 'hex'); // 32 bytes
        },
        getBlockchainEntropy: () => {
            console.log("🎲 Prover: Getting blockchain entropy...");
            return Buffer.from('b'.repeat(64), 'hex');
        },
        submitCommitment: (commitment) => {
            console.log("✅ Prover: Submitting storage commitment to blockchain...");
            return true;
        }
    },
    economic: {
        stakeTokens: (amount) => {
            console.log(`💰 Prover: Staking ${amount} tokens...`);
            return Buffer.from('stake_id', 'utf8');
        },
        getStakeAmount: () => {
            console.log("💰 Prover: Checking stake amount...");
            return 1000;
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
            return Buffer.from(`chunk_${chunkIndex}_data`, 'utf8');
        },
        verifyDataIntegrity: () => {
            console.log("🔍 Prover: Verifying data integrity...");
            return true;
        },
        getStorageStats: () => {
            console.log("📊 Prover: Getting storage statistics...");
            return JSON.stringify({ totalChunks: 1000, usedSpace: "500MB" });
        }
    },
    network: {
        announceAvailability: (commitment) => {
            console.log("📢 Prover: Announcing availability to network...");
        },
        submitChallengeResponse: (response) => {
            console.log("📤 Prover: Submitting challenge response...");
        },
        broadcastProof: (proof) => {
            console.log("📡 Prover: Broadcasting proof to network...");
        }
    }
};

// ====================================================================
// VERIFIER CALLBACKS (Blockchain-Agnostic)
// ====================================================================

const verifierCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => {
            console.log("📚 Verifier: Getting blockchain height...");
            return 12345;
        },
        getBlockHash: (height) => {
            console.log(`📚 Verifier: Getting block hash for height ${height}...`);
            return Buffer.from('a'.repeat(64), 'hex');
        },
        validateBlockHash: (height, hash) => {
            console.log(`✅ Verifier: Validating block hash for height ${height}...`);
            return true;
        },
        getCommitment: (commitmentHash) => {
            console.log("🔍 Verifier: Retrieving commitment from blockchain...");
            return null; // No commitment found
        }
    },
    challenge: {
        issueChallenge: (challenge) => {
            console.log("⚔️ Verifier: Issuing challenge to prover...");
        },
        validateResponse: (response) => {
            console.log("✅ Verifier: Validating challenge response...");
            return true;
        },
        reportResult: (proverKey, passed) => {
            console.log(`📝 Verifier: Reporting result for prover: ${passed ? 'PASSED' : 'FAILED'}`);
        }
    },
    network: {
        discoverProvers: () => {
            console.log("🔍 Verifier: Discovering active provers...");
            return [Buffer.from('prover1', 'utf8'), Buffer.from('prover2', 'utf8')];
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
// DEMONSTRATION
// ====================================================================

async function demonstrateProverVerifierSeparation() {
    console.log("🚀 Starting Prover/Verifier Interface Demonstration\n");

    // Create prover and verifier keys
    const proverKey = Buffer.from('prover_public_key_32_bytes_123456', 'utf8');
    const verifierKey = Buffer.from('verifier_public_key_32_bytes_1234', 'utf8');

    try {
        // ================================================================
        // PROVER OPERATIONS
        // ================================================================
        console.log("👨‍💻 === PROVER OPERATIONS ===");
        const prover = new ProofOfStorageProver(proverKey, proverCallbacks);
        
        // Store data
        const testData = Buffer.from("Hello, this is test data for proof of storage!", 'utf8');
        console.log("\n1. Storing data and generating commitment...");
        const commitment = prover.storeData(testData, "./temp");
        console.log("✅ Storage commitment generated");

        // Create proofs
        console.log("\n2. Creating compact proof...");
        const compactProof = prover.createCompactProof();
        console.log("✅ Compact proof created");

        console.log("\n3. Creating full proof...");
        const fullProof = prover.createFullProof();
        console.log("✅ Full proof created");

        // Get prover stats
        console.log("\n4. Getting prover statistics...");
        const proverStats = prover.getProverStats();
        console.log("📊 Prover stats:", proverStats);

        // ================================================================
        // VERIFIER OPERATIONS
        // ================================================================
        console.log("\n\n🔍 === VERIFIER OPERATIONS ===");
        const verifier = new ProofOfStorageVerifier(verifierKey, verifierCallbacks);

        // Verify proofs
        console.log("\n1. Verifying compact proof...");
        const compactValid = verifier.verifyCompactProof(compactProof);
        console.log(`✅ Compact proof verification: ${compactValid ? 'VALID' : 'INVALID'}`);

        console.log("\n2. Verifying full proof...");
        const fullValid = verifier.verifyFullProof(fullProof);
        console.log(`✅ Full proof verification: ${fullValid ? 'VALID' : 'INVALID'}`);

        // Generate challenge
        console.log("\n3. Generating challenge for prover...");
        const challenge = verifier.generateChallenge(proverKey, commitment.commitmentHash);
        console.log("⚔️ Challenge generated");

        // Prover responds to challenge
        console.log("\n4. Prover responding to challenge...");
        const response = prover.respondToChallenge(challenge);
        console.log("📤 Challenge response generated");

        // Verifier validates response
        console.log("\n5. Verifier validating response...");
        const responseValid = verifier.verifyChallengeResponse(response, challenge);
        console.log(`✅ Challenge response: ${responseValid ? 'VALID' : 'INVALID'}`);

        // Get verifier stats
        console.log("\n6. Getting verifier statistics...");
        const verifierStats = verifier.getVerifierStats();
        console.log("📊 Verifier stats:", verifierStats);

        // ================================================================
        // NETWORK MANAGEMENT
        // ================================================================
        console.log("\n\n🌐 === NETWORK MANAGEMENT ===");
        const networkManager = new HierarchicalNetworkManager(proverKey, "both");

        // Register nodes
        console.log("\n1. Registering prover and verifier...");
        const proverRegistered = networkManager.registerProver(prover);
        const verifierRegistered = networkManager.registerVerifier(verifier);
        console.log(`✅ Prover registered: ${proverRegistered}`);
        console.log(`✅ Verifier registered: ${verifierRegistered}`);

        // Get network stats
        console.log("\n2. Getting network statistics...");
        const networkStats = networkManager.getNetworkStats();
        console.log("📊 Network stats:", {
            totalProvers: networkStats.totalProvers,
            totalVerifiers: networkStats.totalVerifiers,
            healthScore: networkStats.healthScore,
            totalStorage: networkStats.totalStorage,
            challengeSuccessRate: networkStats.challengeSuccessRate
        });

        console.log("\n🎉 === DEMONSTRATION COMPLETE ===");
        console.log("\n✨ Key Benefits of the New Interface:");
        console.log("• Clear separation between prover and verifier responsibilities");
        console.log("• Blockchain-agnostic design through callback interfaces");
        console.log("• Support for any blockchain (Chia, Ethereum, custom, etc.)");
        console.log("• Hierarchical network management for scalability");
        console.log("• Enhanced security with memory-hard VDF and challenge-response");

    } catch (error) {
        console.error("❌ Error during demonstration:", error.message);
    }
}

// Run the demonstration
if (require.main === module) {
    demonstrateProverVerifierSeparation();
}

module.exports = {
    demonstrateProverVerifierSeparation,
    proverCallbacks,
    verifierCallbacks
}; 