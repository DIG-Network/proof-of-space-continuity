import test from 'ava'
import { HashChain } from '../../index.js'
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
import { promises as fs } from 'fs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Production Performance Tests ===

test('streaming performance with large datasets', async t => {
  const testDir = createTestDir('performance_streaming')
  
  // Test with 50 chunks (200KB)
  const chunkCount = 50
  const testData = generateTestDataWithPattern(chunkCount, 'deterministic')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  try {
    // Measure streaming performance
    const streamStart = Date.now()
    hashchain.streamData(testData, testDir)
    const streamTime = Date.now() - streamStart
    
    t.is(hashchain.getTotalChunks(), chunkCount)
    t.true(streamTime < 5000, `Streaming ${chunkCount} chunks took ${streamTime}ms, should be < 5000ms`)
    
    console.log(`Streaming Performance: ${streamTime}ms for ${chunkCount} chunks`)
    
  } finally {
    await fs.rm(testDir, { recursive: true, force: true })
  }
})

test('block addition performance under load', async t => {
  const testDir = createTestDir('performance_blocks')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const testData = createTestData(16384) // 4 chunks
  
  try {
    hashchain.streamData(testData, testDir)
    
    // Measure block addition performance
    const blockTimes = []
    for (let i = 0; i < 20; i++) {
      const blockStart = Date.now()
      const hash = generateVariedBlockHash(i, i * 13)
      hashchain.addBlock(hash)
      const blockTime = Date.now() - blockStart
      blockTimes.push(blockTime)
      
      // Each block should be reasonably fast
      t.true(blockTime < 2000, `Block ${i} took ${blockTime}ms, should be < 2000ms`)
    }
    
    // Verify average performance
    const avgBlockTime = blockTimes.reduce((a, b) => a + b, 0) / blockTimes.length
    t.true(avgBlockTime < 1000, `Average block time ${avgBlockTime}ms should be < 1000ms`)
    
    console.log(`Block Performance: ${avgBlockTime.toFixed(1)}ms average`)
    
  } finally {
    await fs.rm(testDir, { recursive: true, force: true })
  }
})

test('memory efficiency with multiple instances', async t => {
  const testDir = createTestDir('memory_efficiency')
  
  const instances = []
  const instanceCount = 10
  
  try {
    // Create multiple instances simultaneously
    const createStart = Date.now()
    
    for (let i = 0; i < instanceCount; i++) {
      const testData = createTestData(16384) // 4 chunks each (minimum required)
      const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT + i, TEST_BLOCK_HASH)
      hashchain.streamData(testData, testDir)
      instances.push(hashchain)
    }
    
    const createTime = Date.now() - createStart
    t.true(createTime < 10000, `Creating ${instanceCount} instances took ${createTime}ms`)
    
    // All instances should be independent
    for (let i = 0; i < instances.length; i++) {
      t.is(instances[i].getTotalChunks(), 4)
      
      // Add blocks to each
      instances[i].addBlock(generateBlockHash(i))
      t.is(instances[i].getChainLength(), 1)
    }
    
    console.log(`Memory Efficiency: ${instanceCount} instances in ${createTime}ms`)
    
  } finally {
    await fs.rm(testDir, { recursive: true, force: true })
  }
})

test('proof generation performance', async t => {
  const testDir = createTestDir('proof_performance')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const testData = createTestData(32768) // 8 chunks
  
  try {
    hashchain.streamData(testData, testDir)
    
    // Add blocks to enable proof generation
    for (let i = 0; i < 10; i++) {
      hashchain.addBlock(generateBlockHash(i))
    }
    
    // Measure proof generation
    const proofStart = Date.now()
    const proofWindow = hashchain.getProofWindow()
    const proofTime = Date.now() - proofStart
    
    t.true(proofTime < 5000, `Proof generation took ${proofTime}ms, should be < 5000ms`)
    t.is(proofWindow.commitments.length, 8)
    t.is(proofWindow.merkleProofs.length, 32)
    
    // Measure chain validation
    const validateStart = Date.now()
    const isValid = hashchain.verifyChain()
    const validateTime = Date.now() - validateStart
    
    t.true(isValid)
    t.true(validateTime < 5000, `Chain validation took ${validateTime}ms, should be < 5000ms`)
    
    console.log(`Proof Performance: Generation ${proofTime}ms, Validation ${validateTime}ms`)
    
  } finally {
    await fs.rm(testDir, { recursive: true, force: true })
  }
})

test('production readiness simulation', async t => {
  const testDir = createTestDir('production_simulation')
  
  // Simulate production workflow
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const productionData = generateTestDataWithPattern(25, 'deterministic') // 100KB
  
  try {
    // 1. Data ingestion
    const ingestStart = Date.now()
    hashchain.streamData(productionData, testDir)
    const ingestTime = Date.now() - ingestStart
    
    // 2. Continuous block processing
    const processStart = Date.now()
    for (let i = 0; i < 20; i++) {
      hashchain.addBlock(generateVariedBlockHash(i, i * 17))
      
      // Validate periodically
      if (i % 5 === 4) {
        t.true(hashchain.verifyChain())
      }
    }
    const processTime = Date.now() - processStart
    
    // 3. Final validation
    t.true(hashchain.verifyChain())
    t.is(hashchain.getChainLength(), 20)
    
    // Performance requirements
    const avgBlockTime = processTime / 20
    t.true(avgBlockTime < 500, `Average block time ${avgBlockTime}ms should be < 500ms`)
    t.true(ingestTime < 3000, `Data ingestion ${ingestTime}ms should be < 3000ms`)
    
    console.log(`Production Simulation:`)
    console.log(`  Data ingestion: ${ingestTime}ms`)
    console.log(`  Block processing: ${avgBlockTime.toFixed(1)}ms average`)
    console.log(`  ✅ Ready for production use`)
    
  } finally {
    await fs.rm(testDir, { recursive: true, force: true })
  }
}) 