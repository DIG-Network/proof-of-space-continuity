/**
 * Mock Callback System for Blockchain-Agnostic Proof-of-Storage Testing
 * 
 * This file provides mock implementations of all callback interfaces needed for testing
 * the new ProofOfStorageProver and ProofOfStorageVerifier classes without requiring
 * actual blockchain connections.
 */

const crypto = require('crypto');

/**
 * Mock blockchain state for testing
 */
class MockBlockchainState {
  constructor() {
    this.currentHeight = 12345;
    this.blocks = new Map();
    this.commitments = new Map();
    this.entropy = Buffer.alloc(32, 0xAB);
    
    // Initialize some mock blocks
    for (let i = 12340; i <= 12350; i++) {
      const blockHash = Buffer.alloc(32, i % 256);
      this.blocks.set(i, blockHash);
    }
  }
  
  getCurrentBlockHeight() {
    return this.currentHeight;
  }
  
  getBlockHash(height) {
    if (this.blocks.has(height)) {
      return this.blocks.get(height);
    }
    // Generate deterministic hash for the height
    const hash = Buffer.alloc(32);
    hash.writeUInt32BE(height, 0);
    return hash;
  }
  
  validateBlockHash(height, hash) {
    const expectedHash = this.getBlockHash(height);
    return expectedHash.equals(hash);
  }
  
  getBlockchainEntropy() {
    return this.entropy;
  }
  
  submitCommitment(commitment) {
    const key = commitment.commitmentHash.toString('hex');
    this.commitments.set(key, commitment);
    return true;
  }
  
  getCommitment(commitmentHash) {
    const key = commitmentHash.toString('hex');
    return this.commitments.get(key) || null;
  }
}

/**
 * Mock economic state for testing
 */
class MockEconomicState {
  constructor() {
    this.stakes = new Map();
    this.rewards = new Map();
    this.totalStaked = 0;
  }
  
  stakeTokens(amount, publicKey) {
    const stakeId = Buffer.alloc(32);
    crypto.randomFillSync(stakeId);
    const key = publicKey.toString('hex');
    
    this.stakes.set(key, (this.stakes.get(key) || 0) + amount);
    this.totalStaked += amount;
    
    return stakeId;
  }
  
  getStakeAmount(publicKey) {
    const key = publicKey.toString('hex');
    return this.stakes.get(key) || 0;
  }
  
  slashStake(publicKey, amount, reason) {
    const key = publicKey.toString('hex');
    const currentStake = this.stakes.get(key) || 0;
    const slashed = Math.min(amount, currentStake);
    this.stakes.set(key, currentStake - slashed);
    this.totalStaked -= slashed;
  }
  
  claimRewards(publicKey, amount) {
    const key = publicKey.toString('hex');
    this.rewards.set(key, (this.rewards.get(key) || 0) + amount);
    return true;
  }
  
  rewardVerification(amount) {
    return { rewarded: true, amount };
  }
  
  penalizeFailure(amount) {
    return { penalized: true, amount };
  }
}

/**
 * Mock storage state for testing
 */
class MockStorageState {
  constructor() {
    this.chunks = new Map();
    this.storageStats = {
      totalChunks: 0,
      totalSize: 0,
      integrityChecks: 0,
      corruptedChunks: 0
    };
  }
  
  storeChunk(chunkIndex, data) {
    const key = chunkIndex.toString();
    this.chunks.set(key, Buffer.from(data));
    this.storageStats.totalChunks++;
    this.storageStats.totalSize += data.length;
    return true;
  }
  
  retrieveChunk(chunkIndex) {
    const key = chunkIndex.toString();
    const chunk = this.chunks.get(key);
    if (!chunk) {
      throw new Error(`Chunk ${chunkIndex} not found`);
    }
    return chunk;
  }
  
  verifyDataIntegrity() {
    this.storageStats.integrityChecks++;
    // Simulate 99% success rate
    return Math.random() > 0.01;
  }
  
  getStorageStats() {
    return JSON.stringify(this.storageStats);
  }
}

/**
 * Mock network state for testing
 */
class MockNetworkState {
  constructor() {
    this.announcements = [];
    this.responses = [];
    this.broadcasts = [];
    this.provers = new Map();
    this.verifiers = new Map();
    this.challenges = new Map();
    this.reputations = new Map();
    this.misbehaviorReports = [];
  }
  
  announceAvailability(commitment) {
    this.announcements.push({
      commitment,
      timestamp: Date.now()
    });
  }
  
  submitChallengeResponse(response) {
    const key = response.challengeId.toString('hex');
    this.responses.set(key, response);
  }
  
  broadcastProof(proof) {
    this.broadcasts.push({
      proof,
      timestamp: Date.now()
    });
  }
  
  discoverProvers() {
    return Array.from(this.provers.keys()).map(key => Buffer.from(key, 'hex'));
  }
  
  getProverReputation(proverKey) {
    const key = proverKey.toString('hex');
    return this.reputations.get(key) || 50; // Default neutral reputation
  }
  
  reportMisbehavior(proverKey, evidence) {
    this.misbehaviorReports.push({
      proverKey: proverKey.toString('hex'),
      evidence,
      timestamp: Date.now()
    });
  }
  
  issueChallenge(challenge) {
    const key = challenge.challengeId.toString('hex');
    this.challenges.set(key, challenge);
  }
  
  validateResponse(response) {
    const key = response.challengeId.toString('hex');
    const challenge = this.challenges.get(key);
    return challenge !== undefined;
  }
  
  reportResult(proverKey, passed) {
    const key = proverKey.toString('hex');
    const currentRep = this.reputations.get(key) || 50;
    this.reputations.set(key, passed ? currentRep + 1 : currentRep - 5);
  }
}

/**
 * Create mock prover callbacks
 */
function createMockProverCallbacks() {
  const blockchainState = new MockBlockchainState();
  const economicState = new MockEconomicState();
  const storageState = new MockStorageState();
  const networkState = new MockNetworkState();
  
  return {
    blockchain: {
      getCurrentBlockHeight: () => blockchainState.getCurrentBlockHeight(),
      getBlockHash: (height) => blockchainState.getBlockHash(height),
      getBlockchainEntropy: () => blockchainState.getBlockchainEntropy(),
      submitCommitment: (commitment) => blockchainState.submitCommitment(commitment)
    },
    
    economic: {
      stakeTokens: (amount) => economicState.stakeTokens(amount, Buffer.alloc(32, 1)),
      getStakeAmount: () => economicState.getStakeAmount(Buffer.alloc(32, 1)),
      onStakeSlashed: (amount, reason) => economicState.slashStake(Buffer.alloc(32, 1), amount, reason),
      claimRewards: (amount) => economicState.claimRewards(Buffer.alloc(32, 1), amount)
    },
    
    storage: {
      storeChunk: (chunkIndex, data) => storageState.storeChunk(chunkIndex, data),
      retrieveChunk: (chunkIndex) => storageState.retrieveChunk(chunkIndex),
      verifyDataIntegrity: () => storageState.verifyDataIntegrity(),
      getStorageStats: () => storageState.getStorageStats()
    },
    
    network: {
      announceAvailability: (commitment) => networkState.announceAvailability(commitment),
      submitChallengeResponse: (response) => networkState.submitChallengeResponse(response),
      broadcastProof: (proof) => networkState.broadcastProof(proof)
    },
    
    // Add peer network callbacks
    peerNetwork: {
      discoverPeers: () => Array.from({ length: 5 }, (_, i) => Buffer.alloc(32, i + 1)),
      validatePeer: (peerId) => true,
      reportPeerMisbehavior: (peerId, evidence) => true,
      getPeerReputation: (peerId) => 75,
      broadcastToNetwork: (message) => true,
      registerPeer: (peerId, metadata) => true,
      getPeerInfo: (peerId) => ({
        peerId,
        endpoint: "https://peer.example.com:8080",
        nodeType: "prover",
        lastSeen: Date.now(),
        reputation: 75
      }),
      updatePeerLatency: (peerId, latencyMs) => true,
      removePeer: (peerId) => true,
      getNetworkStats: () => ({
        totalPeers: 10,
        activePeers: 8,
        averageLatency: 45,
        networkHealth: 0.9
      }),
      getActivePeers: () => Array.from({ length: 8 }, (_, i) => Buffer.alloc(32, i + 1))
    },
    
    // Add availability challenge callbacks
    availabilityChallenge: {
      respondToChallenge: (challenge) => ({
        challengeId: challenge.challengeId,
        chunkData: Buffer.alloc(4096, 0x42),
        timestamp: Date.now(),
        signature: Buffer.alloc(64, 0x43)
      }),
      validateChallenge: (challenge) => true,
      getResponseDeadline: (challenge) => Date.now() + 60000,
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
      getProverAvailabilityScore: (proverKey) => 85
    },
    
    // Add blockchain data callbacks
    blockchainData: {
      getChainInfo: () => ({
        chainId: Buffer.alloc(32, 0x01),
        networkId: 'chia-mainnet',
        currentEpoch: 100
      }),
      getBlockData: (height) => ({
        height,
        hash: blockchainState.getBlockHash(height),
        timestamp: Date.now(),
        difficulty: 1000
      }),
      subscribeToBlocks: (callback) => true,
      validateChunkCount: (totalChunks, selectedCount) => selectedCount <= Math.min(totalChunks, 16),
      getDataFileMetadata: (dataHash) => ({
        size: 100 * 1024 * 1024, // 100MB
        chunks: 25600, // 100MB / 4KB chunks
        created: Date.now() - 86400000, // 1 day ago
        modified: Date.now() - 3600000, // 1 hour ago
        contentType: 'application/octet-stream',
        checksum: dataHash
      }),
      verifyDataRegistration: (dataHash) => true,
      blockchainEntropy: () => blockchainState.getBlockchainEntropy(),
      getConfirmedStorageSize: (dataHash) => 100 * 1024 * 1024, // 100MB confirmed storage
      updateAvailabilityStatus: (chainId, status) => true
    }
  };
}

/**
 * Create mock verifier callbacks
 */
function createMockVerifierCallbacks() {
  const blockchainState = new MockBlockchainState();
  const economicState = new MockEconomicState();
  const networkState = new MockNetworkState();
  
  return {
    blockchain: {
      getCurrentBlockHeight: () => blockchainState.getCurrentBlockHeight(),
      getBlockHash: (height) => blockchainState.getBlockHash(height),
      validateBlockHash: (height, hash) => blockchainState.validateBlockHash(height, hash),
      getCommitment: (commitmentHash) => blockchainState.getCommitment(commitmentHash)
    },
    
    challenge: {
      issueChallenge: (challenge) => networkState.issueChallenge(challenge),
      validateResponse: (response) => networkState.validateResponse(response),
      reportResult: (proverKey, passed) => networkState.reportResult(proverKey, passed)
    },
    
    network: {
      discoverProvers: () => networkState.discoverProvers(),
      getProverReputation: (proverKey) => networkState.getProverReputation(proverKey),
      reportMisbehavior: (proverKey, evidence) => networkState.reportMisbehavior(proverKey, evidence)
    },
    
    economic: {
      rewardVerification: (amount) => economicState.rewardVerification(amount),
      penalizeFailure: (amount) => economicState.penalizeFailure(amount)
    },
    
    // Add peer network callbacks
    peerNetwork: {
      discoverPeers: () => Array.from({ length: 5 }, (_, i) => Buffer.alloc(32, i + 1)),
      validatePeer: (peerId) => true,
      reportPeerMisbehavior: (peerId, evidence) => true,
      getPeerReputation: (peerId) => 75,
      broadcastToNetwork: (message) => true,
      registerPeer: (peerId, metadata) => true,
      getPeerInfo: (peerId) => ({
        peerId,
        endpoint: "https://peer.example.com:8080",
        nodeType: "verifier",
        lastSeen: Date.now(),
        reputation: 75
      }),
      updatePeerLatency: (peerId, latencyMs) => true,
      removePeer: (peerId) => true,
      getNetworkStats: () => ({
        totalPeers: 10,
        activePeers: 8,
        averageLatency: 45,
        networkHealth: 0.9
      }),
      getActivePeers: () => Array.from({ length: 8 }, (_, i) => Buffer.alloc(32, i + 1))
    },
    
    // Add availability challenge callbacks
    availabilityChallenge: {
      respondToChallenge: (challenge) => ({
        challengeId: challenge.challengeId,
        chunkData: Buffer.alloc(4096, 0x42),
        timestamp: Date.now(),
        signature: Buffer.alloc(64, 0x43)
      }),
      validateChallenge: (challenge) => true,
      getResponseDeadline: (challenge) => Date.now() + 60000,
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
      getProverAvailabilityScore: (proverKey) => 85
    },
    
    // Add blockchain data callbacks
    blockchainData: {
      getChainInfo: () => ({
        chainId: Buffer.alloc(32, 0x01),
        networkId: 'chia-mainnet',
        currentEpoch: 100
      }),
      getBlockData: (height) => ({
        height,
        hash: blockchainState.getBlockHash(height),
        timestamp: Date.now(),
        difficulty: 1000
      }),
      subscribeToBlocks: (callback) => true,
      validateChunkCount: (totalChunks, selectedCount) => selectedCount <= Math.min(totalChunks, 16),
      getDataFileMetadata: (dataHash) => ({
        size: 100 * 1024 * 1024, // 100MB
        chunks: 25600, // 100MB / 4KB chunks
        created: Date.now() - 86400000, // 1 day ago
        modified: Date.now() - 3600000, // 1 hour ago
        contentType: 'application/octet-stream',
        checksum: dataHash
      }),
      verifyDataRegistration: (dataHash) => true,
      blockchainEntropy: () => blockchainState.getBlockchainEntropy(),
      getConfirmedStorageSize: (dataHash) => 100 * 1024 * 1024, // 100MB confirmed storage
      updateAvailabilityStatus: (chainId, status) => true
    }
  };
}

/**
 * Create custom callbacks for specific test scenarios
 */
function createCustomMockCallbacks(type, options = {}) {
  const base = type === 'prover' ? createMockProverCallbacks() : createMockVerifierCallbacks();
  
  // Allow customization for specific test scenarios
  if (options.blockHeight !== undefined) {
    base.blockchain.getCurrentBlockHeight = () => options.blockHeight;
  }
  
  if (options.failStorage && type === 'prover') {
    base.storage.storeChunk = () => false;
    base.storage.retrieveChunk = () => { throw new Error('Storage failure'); };
    base.storage.verifyDataIntegrity = () => false;
  }
  
  if (options.lowReputation && type === 'verifier') {
    base.network.getProverReputation = () => 10; // Low reputation
  }
  
  if (options.networkFailure) {
    if (type === 'prover') {
      base.network.announceAvailability = () => { throw new Error('Network failure'); };
      base.network.broadcastProof = () => { throw new Error('Network failure'); };
    } else {
      base.network.discoverProvers = () => [];
      base.network.reportMisbehavior = () => { throw new Error('Network failure'); };
    }
  }
  
  return base;
}

/**
 * Generate mock data structures
 */
function generateMockMultiSourceEntropy() {
  return {
    blockchainEntropy: Buffer.alloc(32, 0xAA),
    beaconEntropy: Buffer.alloc(32, 0xBB),
    localEntropy: Buffer.alloc(32, 0xCC),
    timestamp: Date.now(),
    combinedHash: Buffer.alloc(32, 0xDD)
  };
}

function generateMockMemoryHardVdfProof() {
  return {
    inputState: Buffer.alloc(32, 0x11),
    outputState: Buffer.alloc(32, 0x22),
    iterations: 1000,
    memoryAccessSamples: [
      {
        iteration: 100,
        readAddress: 1024,
        writeAddress: 2048,
        memoryContentHash: Buffer.alloc(32, 0x33)
      },
      {
        iteration: 500,
        readAddress: 4096,
        writeAddress: 8192,
        memoryContentHash: Buffer.alloc(32, 0x44)
      }
    ],
    computationTimeMs: 500,
    memoryUsageBytes: 1048576,
    memorySize: 1048576
  };
}

function generateMockStorageCommitment() {
  return {
    proverKey: Buffer.alloc(32, 0x01),
    dataHash: Buffer.alloc(32, 0x02),
    blockHeight: 12345,
    blockHash: Buffer.alloc(32, 0x03),
    selectedChunks: [0, 10, 20, 30, 40],
    chunkHashes: [
      Buffer.alloc(32, 0x04),
      Buffer.alloc(32, 0x05),
      Buffer.alloc(32, 0x06),
      Buffer.alloc(32, 0x07),
      Buffer.alloc(32, 0x08)
    ],
    vdfProof: generateMockMemoryHardVdfProof(),
    entropy: generateMockMultiSourceEntropy(),
    commitmentHash: Buffer.alloc(32, 0x09)
  };
}

function generateMockStorageChallenge() {
  return {
    challengeId: Buffer.alloc(32, 0x10),
    proverKey: Buffer.alloc(32, 0x01),
    commitmentHash: Buffer.alloc(32, 0x09),
    challengedChunks: [5, 15, 25],
    nonce: Buffer.alloc(16, 0x11),
    timestamp: Date.now(),
    deadline: Date.now() + 60000 // 1 minute from now
  };
}

function generateMockChallengeResponse() {
  return {
    challengeId: Buffer.alloc(32, 0x10),
    chunkData: [
      Buffer.alloc(4096, 0x12),
      Buffer.alloc(4096, 0x13),
      Buffer.alloc(4096, 0x14)
    ],
    merkleProofs: [
      Buffer.alloc(256, 0x15),
      Buffer.alloc(256, 0x16),
      Buffer.alloc(256, 0x17)
    ],
    timestamp: Date.now(),
    accessProof: generateMockMemoryHardVdfProof()
  };
}

function generateMockCompactStorageProof() {
  return {
    proverKey: Buffer.alloc(32, 0x01),
    commitmentHash: Buffer.alloc(32, 0x09),
    blockHeight: 12345,
    chunkProofs: [
      Buffer.alloc(64, 0x18),
      Buffer.alloc(64, 0x19),
      Buffer.alloc(64, 0x1A)
    ],
    vdfProof: generateMockMemoryHardVdfProof(),
    networkPosition: Buffer.alloc(16, 0x1B),
    timestamp: Date.now()
  };
}

function generateMockFullStorageProof() {
  return {
    proverKey: Buffer.alloc(32, 0x01),
    commitment: generateMockStorageCommitment(),
    allChunkHashes: Array.from({ length: 100 }, (_, i) => Buffer.alloc(32, i)),
    merkleTree: Array.from({ length: 200 }, (_, i) => Buffer.alloc(32, i + 100)),
    vdfChain: [
      generateMockMemoryHardVdfProof(),
      generateMockMemoryHardVdfProof(),
      generateMockMemoryHardVdfProof()
    ],
    networkProofs: [
      Buffer.alloc(128, 0x1C),
      Buffer.alloc(128, 0x1D)
    ],
    metadata: {
      timestamp: Date.now(),
      algorithmVersion: 1,
      securityLevel: "high",
      performanceMetrics: JSON.stringify({ cpu: 80, memory: 1024, io: 50 }),
      verificationGuide: "Standard verification procedure"
    }
  };
}

module.exports = {
  createMockProverCallbacks,
  createMockVerifierCallbacks,
  createCustomMockCallbacks,
  generateMockMultiSourceEntropy,
  generateMockMemoryHardVdfProof,
  generateMockStorageCommitment,
  generateMockStorageChallenge,
  generateMockChallengeResponse,
  generateMockCompactStorageProof,
  generateMockFullStorageProof,
  MockBlockchainState,
  MockEconomicState,
  MockStorageState,
  MockNetworkState
}; 