import test from 'ava'
import { HashChain, verifyProofOfStorageContinuity } from '../../index.js'
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
  generateVariedBlockHash,
  generateTestDataWithPattern
} from '../helpers/test-setup.mjs'
import { existsSync } from 'fs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === End-to-End Integration Tests ===

test('complete workflow: stream to proof generation', (t) => {
  const testDir = createTestDir('complete_workflow')
  const testData = createTestData(20480) // 5 chunks
  
  // 1. Initialize HashChain
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Verify initial state
  t.is(hashchain.getTotalChunks(), 0)
  t.is(hashchain.getChainLength(), 0)
  
  // 2. Stream data
  hashchain.streamData(testData, testDir)
  
  // Verify post-streaming state
  t.is(hashchain.getTotalChunks(), 5)
  t.is(hashchain.getChainLength(), 0)
  t.truthy(hashchain.getFilePaths())
  
  // 3. Add blocks to build chain
  for (let i = 0; i < 10; i++) {
    const blockHash = generateBlockHash(1000 + i)
    const commitment = hashchain.addBlock(blockHash)
    t.truthy(commitment)
    t.is(commitment.selectedChunks.length, 4)
  }
  
  // Verify chain state
  t.is(hashchain.getChainLength(), 10)
  t.true(hashchain.verifyChain())
  
  // 4. Generate proof window
  const proofWindow = hashchain.getProofWindow()
  t.truthy(proofWindow)
  t.is(proofWindow.commitments.length, 8)
  
  cleanupTestDir(testDir)
})

test('state transitions throughout lifecycle', (t) => {
  const testDir = createTestDir('state_transitions')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // State 1: Uninitialized
  let info = hashchain.getChainInfo()
  t.is(info.status, 'uninitialized')
  t.is(info.totalChunks, 0)
  t.false(info.proofWindowReady)
  
  // State 2: Initialized (after streaming)
  hashchain.streamData(testData, testDir)
  info = hashchain.getChainInfo()
  t.is(info.status, 'initialized')
  t.is(info.totalChunks, 5)
  t.is(info.chainLength, 0)
  
  // State 3: Building (1-7 blocks)
  for (let i = 1; i <= 7; i++) {
    hashchain.addBlock(generateBlockHash(4000 + i))
    info = hashchain.getChainInfo()
    t.is(info.status, 'building')
    t.is(info.chainLength, i)
    t.false(info.proofWindowReady)
  }
  
  // State 4: Active (8+ blocks)
  hashchain.addBlock(generateBlockHash(4008))
  info = hashchain.getChainInfo()
  
  t.is(info.status, 'active')
  t.is(info.chainLength, 8)
  t.true(info.proofWindowReady)
  
  cleanupTestDir(testDir)
})

test('error recovery and consistency', (t) => {
  const testDir = createTestDir('error_recovery')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add valid blocks
  for (let i = 0; i < 3; i++) {
    hashchain.addBlock(generateBlockHash(3000 + i))
  }
  
  const validLength = hashchain.getChainLength()
  const validCommitment = hashchain.getCurrentCommitment()
  
  // Invalid operations should not affect state
  t.throws(() => hashchain.addBlock(Buffer.alloc(16))) // Invalid hash
  t.throws(() => hashchain.readChunk(-1)) // Invalid index
  
  // State should be unchanged
  t.is(hashchain.getChainLength(), validLength)
  t.deepEqual(hashchain.getCurrentCommitment(), validCommitment)
  t.true(hashchain.verifyChain())
  
  cleanupTestDir(testDir)
})

test('multiple instance isolation', (t) => {
  const testDir = createTestDir('multiple_instances')
  
  // Create multiple independent instances
  const instances = []
  const testDataSets = [
    generateTestDataWithPattern(4, 'sequential'),
    generateTestDataWithPattern(6, 'deterministic'),
    generateTestDataWithPattern(8, 'sequential')
  ]
  
  // Initialize all instances
  for (let i = 0; i < testDataSets.length; i++) {
    const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT + i, TEST_BLOCK_HASH)
    hashchain.streamData(testDataSets[i], testDir)
    instances.push(hashchain)
  }
  
  // Verify instances are independent
  for (let i = 0; i < instances.length; i++) {
    t.is(instances[i].getTotalChunks(), 4 + (i * 2)) // 4, 6, 8 chunks
    
    // File paths should be different
    for (let j = i + 1; j < instances.length; j++) {
      t.notDeepEqual(instances[i].getFilePaths(), instances[j].getFilePaths())
    }
  }
  
  // Add blocks to all instances
  for (let block = 0; block < 5; block++) {
    for (let inst = 0; inst < instances.length; inst++) {
      const blockHash = generateVariedBlockHash(block, inst * 1000)
      instances[inst].addBlock(blockHash)
    }
  }
  
  // Verify all instances remain independent
  for (let i = 0; i < instances.length; i++) {
    t.is(instances[i].getChainLength(), 5)
    t.true(instances[i].verifyChain())
    
    // Current commitments should be different
    for (let j = i + 1; j < instances.length; j++) {
      t.notDeepEqual(
        instances[i].getCurrentCommitment(),
        instances[j].getCurrentCommitment()
      )
    }
  }
  
  cleanupTestDir(testDir)
})

test('file persistence and reload workflow', (t) => {
  const testDir = createTestDir('file_persistence')
  const testData = createTestData(16384) // 4 chunks
  
  // 1. Create and populate initial hashchain
  const hashchain1 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain1.streamData(testData, testDir)
  
  // Add some blocks
  for (let i = 0; i < 5; i++) {
    hashchain1.addBlock(generateBlockHash(2000 + i))
  }
  
  const originalFilePaths = hashchain1.getFilePaths()
  const originalChainLength = hashchain1.getChainLength()
  const originalCurrentCommitment = hashchain1.getCurrentCommitment()
  const originalAnchoredCommitment = hashchain1.getAnchoredCommitment()
  
  // Verify files exist
  t.true(existsSync(originalFilePaths[0])) // .hashchain file
  t.true(existsSync(originalFilePaths[1])) // .data file
  
  // 2. Load from files (simulating application restart)
  const hashchain2 = HashChain.loadFromFile(originalFilePaths[0])
  
  // Verify loaded state matches original
  t.is(hashchain2.getTotalChunks(), hashchain1.getTotalChunks())
  t.is(hashchain2.getChainLength(), originalChainLength)
  t.deepEqual(hashchain2.getCurrentCommitment(), originalCurrentCommitment)
  t.deepEqual(hashchain2.getAnchoredCommitment(), originalAnchoredCommitment)
  t.deepEqual(hashchain2.getFilePaths(), originalFilePaths)
  
  // 3. Continue operations on loaded instance
  hashchain2.addBlock(generateBlockHash(2005))
  t.is(hashchain2.getChainLength(), originalChainLength + 1)
  t.true(hashchain2.verifyChain())
  
  cleanupTestDir(testDir)
})

test('production-scale end-to-end workflow', (t) => {
  const testDir = createTestDir('production_scale')
  
  // Use larger dataset for production-like testing
  const chunkCount = 50 // 200KB
  const testData = generateTestDataWithPattern(chunkCount, 'deterministic')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Measure streaming performance
  const streamStart = Date.now()
  hashchain.streamData(testData, testDir)
  const streamTime = Date.now() - streamStart
  
  t.true(streamTime < 5000, `Streaming took ${streamTime}ms`)
  t.is(hashchain.getTotalChunks(), chunkCount)
  
  // Add blocks with performance tracking
  const blockTimes = []
  for (let i = 0; i < 20; i++) {
    const blockStart = Date.now()
    hashchain.addBlock(generateVariedBlockHash(i, i * 13))
    const blockTime = Date.now() - blockStart
    blockTimes.push(blockTime)
  }
  
  const avgBlockTime = blockTimes.reduce((a, b) => a + b, 0) / blockTimes.length
  t.true(avgBlockTime < 1000, `Average block time ${avgBlockTime}ms`)
  
  // Generate proof with performance tracking
  const proofStart = Date.now()
  const proofWindow = hashchain.getProofWindow()
  const proofTime = Date.now() - proofStart
  
  t.true(proofTime < 2000, `Proof generation took ${proofTime}ms`)
  t.is(proofWindow.commitments.length, 8)
  
  // Verify chain integrity
  const verifyStart = Date.now()
  const isValid = hashchain.verifyChain()
  const verifyTime = Date.now() - verifyStart
  
  t.true(isValid)
  t.true(verifyTime < 3000, `Chain verification took ${verifyTime}ms`)
  
  cleanupTestDir(testDir)
})

test('data integrity throughout complete lifecycle', (t) => {
  const testDir = createTestDir('data_integrity_lifecycle')
  
  // Create data with known pattern
  const testData = Buffer.alloc(24576) // 6 chunks
  for (let i = 0; i < testData.length; i++) {
    testData[i] = (i * 17 + 23) % 256
  }
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Verify data integrity after streaming
  for (let chunkIdx = 0; chunkIdx < 6; chunkIdx++) {
    const chunk = hashchain.readChunk(chunkIdx)
    
    for (let i = 0; i < 1000; i++) { // Check first 1000 bytes
      const originalIndex = chunkIdx * 4096 + i
      if (originalIndex < testData.length) {
        t.is(chunk[i], testData[originalIndex])
      }
    }
  }
  
  // Add blocks and verify data integrity is preserved
  for (let i = 0; i < 10; i++) {
    hashchain.addBlock(generateVariedBlockHash(i, i * 31))
    
    // Verify data can still be read correctly
    const randomChunk = Math.floor(Math.random() * 6)
    const chunk = hashchain.readChunk(randomChunk)
    
    // Check first few bytes still match
    const originalIndex = randomChunk * 4096
    for (let j = 0; j < 10; j++) {
      if (originalIndex + j < testData.length) {
        t.is(chunk[j], testData[originalIndex + j])
      }
    }
  }
  
  // Verify chain integrity
  t.true(hashchain.verifyChain())
  
  cleanupTestDir(testDir)
})

test('concurrent operations integration', (t) => {
  const testDir = createTestDir('concurrent_operations')
  
  // Create multiple instances with different configurations
  const configs = [
    { chunks: 4, publicKey: Buffer.from('a'.repeat(64), 'hex') },
    { chunks: 6, publicKey: Buffer.from('b'.repeat(64), 'hex') },
    { chunks: 8, publicKey: Buffer.from('c'.repeat(64), 'hex') }
  ]
  
  const instances = []
  
  // Initialize all instances
  for (let i = 0; i < configs.length; i++) {
    const config = configs[i]
    const testData = createTestData(config.chunks * 4096)
    
    const hashchain = new HashChain(
      config.publicKey, 
      TEST_BLOCK_HEIGHT + i, 
      TEST_BLOCK_HASH
    )
    
    hashchain.streamData(testData, testDir)
    instances.push(hashchain)
  }
  
  // Perform operations on all instances concurrently
  for (let blockNum = 0; blockNum < 12; blockNum++) {
    for (let instIdx = 0; instIdx < instances.length; instIdx++) {
      const blockHash = generateVariedBlockHash(blockNum, instIdx * 100)
      instances[instIdx].addBlock(blockHash)
    }
  }
  
  // Verify all instances are valid and independent
  for (let i = 0; i < instances.length; i++) {
    t.is(instances[i].getChainLength(), 12)
    t.is(instances[i].getTotalChunks(), configs[i].chunks)
    t.true(instances[i].verifyChain())
    
    // Should be able to generate proof windows
    const proofWindow = instances[i].getProofWindow()
    t.is(proofWindow.commitments.length, 8)
    
    // Verify independence from other instances
    for (let j = i + 1; j < instances.length; j++) {
      t.notDeepEqual(
        instances[i].getCurrentCommitment(),
        instances[j].getCurrentCommitment()
      )
      t.notDeepEqual(
        instances[i].getFilePaths(),
        instances[j].getFilePaths()
      )
    }
  }
  
  cleanupTestDir(testDir)
}) 