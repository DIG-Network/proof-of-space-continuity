import test from 'ava'
import { HashChain, verifyChunkSelection } from '../../index.js'
import { 
  TEST_PUBLIC_KEY, 
  TEST_BLOCK_HASH, 
  TEST_BLOCK_HEIGHT,
  createTestData,
  createTestDir,
  cleanupTestDir,
  setupTest,
  teardownTest,
  generateBlockHash,
  generateVariedBlockHash
} from '../helpers/test-setup.mjs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Chain Management Integration Tests ===

test('addBlock creates physical access commitment', (t) => {
  const testDir = createTestDir('add_block_test')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const newBlockHash = generateBlockHash(1)
  const commitment = hashchain.addBlock(newBlockHash)
  
  t.truthy(commitment)
  t.is(commitment.selectedChunks.length, 4) // Should select 4 chunks
  t.is(commitment.chunkHashes.length, 4) // Should have 4 chunk hashes
  t.is(commitment.commitmentHash.length, 32) // 32-byte commitment hash
  t.deepEqual(commitment.blockHash, newBlockHash)
  
  // Chain length should increment
  t.is(hashchain.getChainLength(), 1)
  
  // Current commitment should change
  t.deepEqual(hashchain.getCurrentCommitment(), commitment.commitmentHash)
  
  cleanupTestDir(testDir)
})

test('addBlock validates block hash size', (t) => {
  const testDir = createTestDir('add_block_validation')
  const testData = createTestData(4096)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const invalidBlockHash = Buffer.from('invalid')
  
  t.throws(() => {
    hashchain.addBlock(invalidBlockHash)
  }, { message: /Block hash must be 32 bytes/ })
  
  cleanupTestDir(testDir)
})

test('addBlock fails without streamed data', (t) => {
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const newBlockHash = generateBlockHash(1)
  
  t.throws(() => {
    hashchain.addBlock(newBlockHash)
  }, { message: /No data has been streamed/ })
})

test('multiple addBlock calls create chain', (t) => {
  const testDir = createTestDir('multiple_blocks')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const blockHashes = [
    generateBlockHash(1),
    generateBlockHash(2),
    generateBlockHash(3)
  ]
  
  let previousCommitment = hashchain.getCurrentCommitment()
  
  for (let i = 0; i < blockHashes.length; i++) {
    const commitment = hashchain.addBlock(blockHashes[i])
    
    // Each commitment should reference the previous one
    t.deepEqual(commitment.previousCommitment, previousCommitment)
    
    // Chain length should increment
    t.is(hashchain.getChainLength(), i + 1)
    
    previousCommitment = commitment.commitmentHash
  }
  
  cleanupTestDir(testDir)
})

test('chain state transitions through status phases', (t) => {
  const testDir = createTestDir('status_transitions')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // 1. Uninitialized state
  let info = hashchain.getChainInfo()
  t.is(info.status, 'uninitialized')
  t.is(info.chainLength, 0)
  t.false(info.proofWindowReady)
  t.is(info.blocksUntilProofReady, 8)
  
  // 2. Initialized state (after streaming)
  hashchain.streamData(testData, testDir)
  info = hashchain.getChainInfo()
  t.is(info.status, 'initialized')
  t.is(info.chainLength, 0)
  t.false(info.proofWindowReady)
  t.is(info.blocksUntilProofReady, 8)
  
  // 3. Building state (1-7 blocks)
  for (let i = 1; i <= 7; i++) {
    hashchain.addBlock(generateBlockHash(i))
    info = hashchain.getChainInfo()
    
    t.is(info.status, 'building')
    t.is(info.chainLength, i)
    t.false(info.proofWindowReady)
    t.is(info.blocksUntilProofReady, 8 - i)
  }
  
  // 4. Active state (8+ blocks)
  hashchain.addBlock(generateBlockHash(8))
  info = hashchain.getChainInfo()
  
  t.is(info.status, 'active')
  t.is(info.chainLength, 8)
  t.true(info.proofWindowReady)
  t.is(info.blocksUntilProofReady, undefined)
  
  cleanupTestDir(testDir)
})

test('addBlock maintains consensus compliance', (t) => {
  const testDir = createTestDir('consensus_compliance')
  const testData = createTestData(32768) // 8 chunks for variety
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add multiple blocks with different patterns
  for (let i = 0; i < 10; i++) {
    const blockHash = generateVariedBlockHash(i, i * 13)
    const commitment = hashchain.addBlock(blockHash)
    
    // Verify consensus compliance
    t.is(commitment.selectedChunks.length, 4) // Exactly 4 chunks
    t.is(commitment.chunkHashes.length, 4) // Exactly 4 hashes
    
    // Verify chunk selection follows consensus algorithm
    t.true(verifyChunkSelection(
      blockHash,
      hashchain.getTotalChunks(),
      commitment.selectedChunks,
      1 // Algorithm version 1
    ))
    
    // All selected chunks should be in valid range
    for (const chunkIdx of commitment.selectedChunks) {
      t.true(chunkIdx >= 0 && chunkIdx < hashchain.getTotalChunks())
    }
    
    // No duplicate chunks
    const uniqueChunks = new Set(commitment.selectedChunks)
    t.is(uniqueChunks.size, 4)
  }
  
  cleanupTestDir(testDir)
})

test('addBlock handles block height progression', (t) => {
  const testDir = createTestDir('block_height_progression')
  const testData = createTestData(16384) // 4 chunks
  
  const initialBlockHeight = 50000
  const hashchain = new HashChain(TEST_PUBLIC_KEY, initialBlockHeight, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add several blocks and verify height progression
  for (let i = 0; i < 5; i++) {
    const blockHash = generateBlockHash(1000 + i)
    const commitment = hashchain.addBlock(blockHash)
    
    // Block height should increment from initial height
    t.is(commitment.blockHeight, initialBlockHeight + i + 1)
  }
  
  cleanupTestDir(testDir)
})

test('chain verification throughout block addition', (t) => {
  const testDir = createTestDir('chain_verification')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Verify chain after streaming
  t.true(hashchain.verifyChain())
  
  // Add blocks and verify after each addition
  for (let i = 0; i < 12; i++) {
    const blockHash = generateVariedBlockHash(i, i * 7)
    hashchain.addBlock(blockHash)
    
    // Chain should remain valid after each block
    t.true(hashchain.verifyChain())
  }
  
  cleanupTestDir(testDir)
})

test('commitment chain linkage integrity', (t) => {
  const testDir = createTestDir('commitment_linkage')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const commitments = []
  let expectedPrevious = hashchain.getCurrentCommitment() // Anchored commitment
  
  // Add 10 blocks and track linkage
  for (let i = 0; i < 10; i++) {
    const blockHash = generateBlockHash(100 + i)
    const commitment = hashchain.addBlock(blockHash)
    
    // Verify linkage
    t.deepEqual(commitment.previousCommitment, expectedPrevious)
    
    commitments.push(commitment)
    expectedPrevious = commitment.commitmentHash
  }
  
  // Final current commitment should match last commitment
  t.deepEqual(hashchain.getCurrentCommitment(), expectedPrevious)
  
  // Verify complete chain
  for (let i = 1; i < commitments.length; i++) {
    t.deepEqual(
      commitments[i].previousCommitment,
      commitments[i - 1].commitmentHash
    )
  }
  
  cleanupTestDir(testDir)
})

test('addBlock with diverse chunk patterns', (t) => {
  const testDir = createTestDir('diverse_chunks')
  const testData = createTestData(102400) // 25 chunks for variety
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const allSelectedChunks = new Set()
  
  // Add many blocks to see chunk diversity
  for (let i = 0; i < 20; i++) {
    const blockHash = generateVariedBlockHash(i, i * 17)
    const commitment = hashchain.addBlock(blockHash)
    
    // Track which chunks get selected
    for (const chunkIdx of commitment.selectedChunks) {
      allSelectedChunks.add(chunkIdx)
    }
    
    // Verify each commitment
    t.is(commitment.selectedChunks.length, 4)
    t.is(commitment.chunkHashes.length, 4)
    
    // All chunks should be in valid range
    for (const chunkIdx of commitment.selectedChunks) {
      t.true(chunkIdx >= 0 && chunkIdx < 25)
    }
  }
  
  // Should have selected a variety of chunks across the dataset
  // With 20 blocks * 4 chunks = 80 selections from 25 chunks,
  // we should see good coverage
  t.true(allSelectedChunks.size >= 15, `Only ${allSelectedChunks.size} unique chunks selected`)
  
  cleanupTestDir(testDir)
})

test('concurrent chain management operations', (t) => {
  const testDir = createTestDir('concurrent_chains')
  
  // Create multiple independent chains
  const chains = []
  const testDataSets = [
    createTestData(16384), // 4 chunks
    createTestData(20480), // 5 chunks
    createTestData(24576)  // 6 chunks
  ]
  
  // Initialize all chains
  for (let i = 0; i < testDataSets.length; i++) {
    const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT + i, TEST_BLOCK_HASH)
    hashchain.streamData(testDataSets[i], testDir)
    chains.push(hashchain)
  }
  
  // Add blocks to all chains simultaneously
  for (let block = 0; block < 5; block++) {
    for (let chain = 0; chain < chains.length; chain++) {
      const blockHash = generateVariedBlockHash(block, chain * 100)
      const commitment = chains[chain].addBlock(blockHash)
      
      // Verify independent operation
      t.is(commitment.selectedChunks.length, 4)
      t.is(chains[chain].getChainLength(), block + 1)
    }
  }
  
  // All chains should be independent and valid
  for (let i = 0; i < chains.length; i++) {
    t.is(chains[i].getChainLength(), 5)
    t.is(chains[i].getTotalChunks(), 4 + i) // 4, 5, 6 chunks respectively
    t.true(chains[i].verifyChain())
    
    // Each should have different current commitments
    for (let j = i + 1; j < chains.length; j++) {
      t.notDeepEqual(
        chains[i].getCurrentCommitment(),
        chains[j].getCurrentCommitment()
      )
    }
  }
  
  cleanupTestDir(testDir)
})

test('chain state persistence through operations', (t) => {
  const testDir = createTestDir('state_persistence')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Store initial state
  const initialAnchoredCommitment = hashchain.getAnchoredCommitment()
  const initialFilePaths = hashchain.getFilePaths()
  
  // Add blocks and verify state consistency
  for (let i = 0; i < 8; i++) {
    const blockHash = generateBlockHash(200 + i)
    hashchain.addBlock(blockHash)
    
    // Anchored commitment should never change
    t.deepEqual(hashchain.getAnchoredCommitment(), initialAnchoredCommitment)
    
    // File paths should never change
    t.deepEqual(hashchain.getFilePaths(), initialFilePaths)
    
    // Total chunks should never change
    t.is(hashchain.getTotalChunks(), 4)
    
    // Chain info should show correct progression
    const info = hashchain.getChainInfo()
    t.is(info.chainLength, i + 1)
    t.is(info.totalChunks, 4)
    t.is(info.initialBlockHeight, TEST_BLOCK_HEIGHT)
  }
  
  cleanupTestDir(testDir)
})

test('error recovery and edge cases in chain management', (t) => {
  const testDir = createTestDir('error_recovery')
  const testData = createTestData(16384) // 4 chunks (minimum for selection)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Valid operations should work
  t.notThrows(() => {
    hashchain.addBlock(generateBlockHash(1))
  })
  
  t.is(hashchain.getChainLength(), 1)
  
  // Invalid operations should fail without corrupting state
  t.throws(() => {
    hashchain.addBlock(Buffer.from('invalid'))
  })
  
  // State should be unchanged after error
  t.is(hashchain.getChainLength(), 1)
  t.true(hashchain.verifyChain())
  
  // Should be able to continue normally
  t.notThrows(() => {
    hashchain.addBlock(generateBlockHash(2))
  })
  
  t.is(hashchain.getChainLength(), 2)
  t.true(hashchain.verifyChain())
  
  cleanupTestDir(testDir)
}) 