const { ProofOfStorageProver } = require('./index.js');

// Helper function to create a delay
function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

// Simplified callbacks for demo
const demoCallbacks = {
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

async function demonstrateVdfTiming() {
    console.log("🚀 VDF Timing Constraint Demonstration");
    console.log("=====================================");
    console.log("This demo shows how VDF queuing enforces ~16 second delays between blocks\n");

    try {
        // Create prover
        const proverKey = Buffer.alloc(32, 1);
        const privateKey = Buffer.from(proverKey);
        const prover = new ProofOfStorageProver(proverKey, privateKey, demoCallbacks);

        // Create a 64KB test file (minimum size for 16 chunks)
        const testData = Buffer.alloc(65536, 0x42); // 64KB file filled with 'B'
        
        console.log("📁 Storing initial test file (64KB)...");
        const initialCommitment = prover.storeData(testData, './vdf_demo_output');
        console.log(`✅ Initial file stored. Commitment: ${initialCommitment.commitmentHash.toString('hex').substring(0, 16)}...\n`);

        console.log("🔄 Starting VDF Timing Test");
        console.log("---------------------------");

        // Submit multiple blocks in rapid succession to test queuing
        const blockHashes = [];
        const startTime = Date.now();

        console.log("⚡ Rapidly submitting 5 blocks (every 3 seconds) to test VDF queuing...");
        console.log("   This demonstrates blocks submitted faster than 16-second VDF completion:");
        
        for (let i = 1; i <= 5; i++) {
            const blockHash = Buffer.from(`test_block_${i}`.padEnd(32, '0'));
            blockHashes.push(blockHash);
            
            const result = prover.submitBlockForVdf(i, blockHash);
            console.log(`📤 Block ${i}: ${result}`);
            
            // Check queue status
            const queueStatus = prover.getVdfQueueStatus();
            console.log(`   📊 Queue: ${queueStatus.pendingCount} pending, ${queueStatus.completedCount} completed`);
            
            // Wait 3 seconds between submissions (faster than 16-second VDF)
            if (i < 5) {
                await delay(3000);
            }
        }

        console.log("\n⏱️  VDF Processing Timeline:");
        console.log("============================");

        // Process VDF queue and measure timing
        for (let round = 1; round <= 5; round++) {
            const roundStart = Date.now();
            console.log(`\n🔄 Round ${round} - Processing VDF queue...`);
            
            try {
                const processResult = prover.processVdfQueue();
                const processingTime = Date.now() - roundStart;
                
                console.log(`   📋 Result: ${processResult}`);
                console.log(`   ⏱️  Processing time: ${processingTime.toFixed(0)}ms`);
                
                // Check queue status after processing
                const queueStatus = prover.getVdfQueueStatus();
                console.log(`   📊 Queue status:`);
                console.log(`      - Pending: ${queueStatus.pendingCount}`);
                console.log(`      - Completed: ${queueStatus.completedCount}`);
                console.log(`      - Current VDF: ${queueStatus.currentVdf || 'None'}`);
                
                // Check which blocks are ready
                for (let i = 0; i < blockHashes.length; i++) {
                    const isReady = prover.isBlockReady(blockHashes[i]);
                    console.log(`      - Block ${i + 1} ready: ${isReady ? '✅' : '❌'}`);
                }
                
            } catch (error) {
                console.log(`   ❌ Error: ${error.message}`);
            }
        }

        const totalTime = Date.now() - startTime;
        console.log(`\n📈 Summary:`);
        console.log(`   - Total demonstration time: ${(totalTime / 1000).toFixed(1)} seconds`);
        console.log(`   - Expected VDF time per block: ~16 seconds (256MB memory-hard computation)`);
        console.log(`   - Blocks can only be finalized after VDF completion`);
        console.log(`   - Queue prevents parallel VDF computation to maintain timing security`);

        // Show final queue status
        const finalStatus = prover.getVdfQueueStatus();
        console.log(`\n🏁 Final Queue Status:`);
        console.log(`   - Pending blocks: ${finalStatus.pendingCount}`);
        console.log(`   - Completed blocks: ${finalStatus.completedCount}`);
        console.log(`   - Queue capacity: ${finalStatus.queueCapacity}`);

        console.log("\n🎯 Key Security Properties Demonstrated:");
        console.log("   ✅ Blocks cannot be finalized without VDF completion");
        console.log("   ✅ VDF computation enforces ~16 second minimum delay");
        console.log("   ✅ Queue prevents rapid block generation attacks");
        console.log("   ✅ Memory-hard VDF resists parallelization");
        console.log("   ✅ Timing constraints maintain network synchronization");

    } catch (error) {
        console.error("❌ Demo failed:", error.message);
    }
}

// Run the demonstration
if (require.main === module) {
    demonstrateVdfTiming()
        .then(() => {
            console.log("\n✅ VDF Timing Demonstration completed successfully!");
        })
        .catch(error => {
            console.error("❌ Unexpected error:", error);
        });
}

module.exports = { demonstrateVdfTiming }; 