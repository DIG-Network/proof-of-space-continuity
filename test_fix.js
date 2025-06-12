const { ProofOfStorageProver } = require('./index.js');

const testCallbacks = {
    blockchain: {
        getCurrentBlockHeight: () => 12345,
        getBlockHash: (height) => Buffer.alloc(32),
        getBlockchainEntropy: () => Buffer.alloc(32),
        submitCommitment: (commitment) => true
    },
    economic: {
        stakeTokens: (amount) => Buffer.alloc(32),
        getStakeAmount: () => 1000,
        onStakeSlashed: (amount, reason) => {},
        claimRewards: (amount) => true
    },
    storage: {
        storeChunk: (chunkIndex, data) => true,
        retrieveChunk: (chunkIndex) => Buffer.alloc(100),
        verifyDataIntegrity: () => true,
        getStorageStats: () => "{}"
    },
    network: {
        announceAvailability: (commitment) => {},
        submitChallengeResponse: (response) => {},
        broadcastProof: (proof) => {}
    },
    peerNetwork: {
        discoverPeers: () => [Buffer.alloc(32)],
        validatePeer: (peerId) => true,
        reportPeerMisbehavior: (peerId, evidence) => true,
        getPeerReputation: (peerId) => 85,
        broadcastToNetwork: (message) => true,
        registerPeer: (peerId, metadata) => true,
        getPeerInfo: (peerId) => ({
            peerId,
            endpoint: "https://peer.example.com:8080",
            nodeType: "prover",
            lastSeen: Date.now(),
            reputation: 85
        }),
        updatePeerLatency: (peerId, latencyMs) => true,
        removePeer: (peerId) => true,
        getNetworkStats: () => ({
            totalPeers: 15,
            activePeers: 12,
            averageLatency: 45,
            networkHealth: 0.9
        }),
        getActivePeers: () => [Buffer.alloc(32)]
    }
};

console.log("Testing ProofOfStorageProver with peerNetwork callbacks...");

try {
    const prover = new ProofOfStorageProver(Buffer.alloc(32), testCallbacks);
    console.log("✅ Success! ProofOfStorageProver created with peerNetwork callbacks");
} catch (error) {
    console.log("❌ Error:", error.message);
} 