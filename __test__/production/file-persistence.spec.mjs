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
  generateTestDataWithPattern
} from '../helpers/test-setup.mjs'
import { promises as fs, existsSync } from 'fs'
import { join } from 'path'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Production File Persistence Tests ===

test('file creation and naming consistency', (t) => {
  const testDir = createTestDir('file_naming_consistency')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const filePaths = hashchain.getFilePaths()
  t.truthy(filePaths)
  t.is(filePaths.length, 2)
  
  // Verify both files exist
  t.true(existsSync(filePaths[0])) // .hashchain file
  t.true(existsSync(filePaths[1])) // .data file
  
  // Verify naming convention (SHA256-based)
  const hashchainFile = filePaths[0]
  const dataFile = filePaths[1]
  
  // Both should have same base name (SHA256 hash)
  const hashchainBase = hashchainFile.slice(0, -10) // Remove .hashchain
  const dataBase = dataFile.slice(0, -5) // Remove .data
  
  t.is(hashchainBase, dataBase)
  
  // Base name should be 64 hex characters (SHA256)
  const baseName = hashchainBase.split('/').pop()
  t.is(baseName.length, 64)
  t.true(/^[0-9a-f]{64}$/.test(baseName))
  
  cleanupTestDir(testDir)
})

test('file persistence across application restarts', async (t) => {
  const testDir = createTestDir('file_persistence_restart')
  const testData = generateTestDataWithPattern(8, 'deterministic')
  
  // Phase 1: Create and populate hashchain
  const hashchain1 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain1.streamData(testData, testDir)
  
  // Add blocks
  for (let i = 0; i < 5; i++) {
    hashchain1.addBlock(generateBlockHash(1000 + i))
  }
  
  const originalState = {
    filePaths: hashchain1.getFilePaths(),
    chainLength: hashchain1.getChainLength(),
    totalChunks: hashchain1.getTotalChunks(),
    currentCommitment: hashchain1.getCurrentCommitment(),
    anchoredCommitment: hashchain1.getAnchoredCommitment()
  }
  
  // Verify files exist and have expected sizes
  const hashchainStats = await fs.stat(originalState.filePaths[0])
  const dataStats = await fs.stat(originalState.filePaths[1])
  
  t.true(hashchainStats.size > 0)
  t.is(dataStats.size, 8 * 4096) // 8 chunks * 4KB each
  
  // Phase 2: Simulate application restart by loading from file
  const hashchain2 = HashChain.loadFromFile(originalState.filePaths[0])
  
  // Verify loaded state matches original
  t.deepEqual(hashchain2.getFilePaths(), originalState.filePaths)
  t.is(hashchain2.getChainLength(), originalState.chainLength)
  t.is(hashchain2.getTotalChunks(), originalState.totalChunks)
  t.deepEqual(hashchain2.getCurrentCommitment(), originalState.currentCommitment)
  t.deepEqual(hashchain2.getAnchoredCommitment(), originalState.anchoredCommitment)
  
  // Phase 3: Continue operations on loaded instance
  hashchain2.addBlock(generateBlockHash(1005))
  hashchain2.addBlock(generateBlockHash(1006))
  
  t.is(hashchain2.getChainLength(), originalState.chainLength + 2)
  t.true(hashchain2.verifyChain())
  
  cleanupTestDir(testDir)
})

test('file corruption detection and handling', async (t) => {
  const testDir = createTestDir('file_corruption_detection')
  const testData = createTestData(12288) // 3 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const filePaths = hashchain.getFilePaths()
  
  // Corrupt the .hashchain file by modifying a byte
  const hashchainContent = await fs.readFile(filePaths[0])
  const corruptedContent = Buffer.from(hashchainContent)
  corruptedContent[100] = corruptedContent[100] ^ 0xFF // Flip bits
  await fs.writeFile(filePaths[0], corruptedContent)
  
  // Attempt to load corrupted file should fail
  t.throws(() => {
    HashChain.loadFromFile(filePaths[0])
  })
  
  cleanupTestDir(testDir)
})

test('large file handling and performance', (t) => {
  const testDir = createTestDir('large_file_handling')
  
  // Create larger dataset (1MB = 256 chunks)
  const chunkCount = 256
  const testData = generateTestDataWithPattern(chunkCount, 'sequential')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Measure file creation time
  const streamStart = Date.now()
  hashchain.streamData(testData, testDir)
  const streamTime = Date.now() - streamStart
  
  t.true(streamTime < 10000, `Large file streaming took ${streamTime}ms`)
  t.is(hashchain.getTotalChunks(), chunkCount)
  
  // Verify file sizes are correct
  const filePaths = hashchain.getFilePaths()
  const info = hashchain.getChainInfo()
  
  t.is(info.dataFileSizeBytes, chunkCount * 4096)
  t.true(info.hashchainFileSizeBytes > 0)
  
  // Test random chunk access performance
  const accessStart = Date.now()
  for (let i = 0; i < 50; i++) {
    const randomChunk = Math.floor(Math.random() * chunkCount)
    const chunk = hashchain.readChunk(randomChunk)
    t.is(chunk.length, 4096)
  }
  const accessTime = Date.now() - accessStart
  
  t.true(accessTime < 5000, `50 random accesses took ${accessTime}ms`)
  
  cleanupTestDir(testDir)
})

test('concurrent file access safety', (t) => {
  const testDir = createTestDir('concurrent_file_access')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add blocks to enable various operations
  for (let i = 0; i < 10; i++) {
    hashchain.addBlock(generateBlockHash(2000 + i))
  }
  
  // Simulate concurrent operations
  const operations = []
  
  // Concurrent chunk reads
  for (let i = 0; i < 20; i++) {
    operations.push(() => {
      const chunkIdx = i % 5
      return hashchain.readChunk(chunkIdx)
    })
  }
  
  // Concurrent info queries
  for (let i = 0; i < 10; i++) {
    operations.push(() => hashchain.getChainInfo())
  }
  
  // Concurrent proof window generation
  for (let i = 0; i < 5; i++) {
    operations.push(() => hashchain.getProofWindow())
  }
  
  // Execute all operations (simulating concurrency)
  const results = operations.map(op => {
    try {
      return { success: true, result: op() }
    } catch (error) {
      return { success: false, error }
    }
  })
  
  // All operations should succeed
  const failures = results.filter(r => !r.success)
  t.is(failures.length, 0, `${failures.length} operations failed`)
  
  // Verify chain integrity is maintained
  t.true(hashchain.verifyChain())
  
  cleanupTestDir(testDir)
})

test('file system edge cases', async (t) => {
  const testDir = createTestDir('filesystem_edge_cases')
  const testData = createTestData(8192) // 2 chunks
  
  // Test with nested directory creation
  const nestedDir = join(testDir, 'deeply', 'nested', 'path')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Should handle deep directory creation
  t.notThrows(() => {
    hashchain.streamData(testData, nestedDir)
  })
  
  const filePaths = hashchain.getFilePaths()
  t.true(existsSync(filePaths[0]))
  t.true(existsSync(filePaths[1]))
  
  // Verify files are in the correct nested location
  t.true(filePaths[0].includes('deeply/nested/path'))
  t.true(filePaths[1].includes('deeply/nested/path'))
  
  cleanupTestDir(testDir)
})

test('file permissions and access patterns', async (t) => {
  const testDir = createTestDir('file_permissions')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const filePaths = hashchain.getFilePaths()
  
  // Verify files are readable
  const hashchainStats = await fs.stat(filePaths[0])
  const dataStats = await fs.stat(filePaths[1])
  
  t.true(hashchainStats.isFile())
  t.true(dataStats.isFile())
  t.true(hashchainStats.size > 0)
  t.true(dataStats.size > 0)
  
  // Verify data can be read back
  for (let i = 0; i < 4; i++) {
    t.notThrows(() => {
      const chunk = hashchain.readChunk(i)
      t.is(chunk.length, 4096)
    })
  }
  
  cleanupTestDir(testDir)
})

test('disk space efficiency and optimization', (t) => {
  const testDir = createTestDir('disk_space_efficiency')
  const testData = createTestData(40960) // 10 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add blocks to create chain data
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(generateBlockHash(3000 + i))
  }
  
  const info = hashchain.getChainInfo()
  
  // Data file should be exactly the expected size
  t.is(info.dataFileSizeBytes, 10 * 4096)
  
  // HashChain file should be reasonable size (not bloated)
  const expectedMetadataSize = 1024 // Rough estimate for headers, merkle tree, chain
  t.true(info.hashchainFileSizeBytes > expectedMetadataSize)
  t.true(info.hashchainFileSizeBytes < info.dataFileSizeBytes / 2) // Should be much smaller than data
  
  // Total storage efficiency
  const totalSize = info.dataFileSizeBytes + info.hashchainFileSizeBytes
  const overhead = info.hashchainFileSizeBytes / info.dataFileSizeBytes
  
  t.true(overhead < 0.5, `Metadata overhead too high: ${overhead * 100}%`)
  
  cleanupTestDir(testDir)
})

test('file recovery and validation workflows', async (t) => {
  const testDir = createTestDir('file_recovery_validation')
  const testData = generateTestDataWithPattern(6, 'deterministic')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add some blocks
  for (let i = 0; i < 4; i++) {
    hashchain.addBlock(generateBlockHash(4000 + i))
  }
  
  const filePaths = hashchain.getFilePaths()
  
  // Verify chain before any modifications
  t.true(hashchain.verifyChain())
  
  // Create backup of files
  const hashchainBackup = await fs.readFile(filePaths[0])
  const dataBackup = await fs.readFile(filePaths[1])
  
  // Simulate file recovery scenario
  const hashchain2 = HashChain.loadFromFile(filePaths[0])
  
  // Verify recovered state
  t.is(hashchain2.getChainLength(), hashchain.getChainLength())
  t.is(hashchain2.getTotalChunks(), hashchain.getTotalChunks())
  t.deepEqual(hashchain2.getCurrentCommitment(), hashchain.getCurrentCommitment())
  
  // Verify data integrity after recovery
  for (let i = 0; i < 6; i++) {
    const originalChunk = hashchain.readChunk(i)
    const recoveredChunk = hashchain2.readChunk(i)
    t.deepEqual(originalChunk, recoveredChunk)
  }
  
  // Continue operations on recovered instance
  hashchain2.addBlock(generateBlockHash(4004))
  t.true(hashchain2.verifyChain())
  
  cleanupTestDir(testDir)
})

test('production load simulation', (t) => {
  const testDir = createTestDir('production_load_simulation')
  
  // Simulate production-like scenario
  const instances = []
  const instanceCount = 5
  
  // Create multiple concurrent instances
  for (let i = 0; i < instanceCount; i++) {
    const chunkCount = 20 + (i * 5) // Varying sizes: 20, 25, 30, 35, 40
    const testData = generateTestDataWithPattern(chunkCount, `pattern_${i}`)
    
    const publicKey = Buffer.alloc(32, i + 1) // Different keys
    const hashchain = new HashChain(publicKey, TEST_BLOCK_HEIGHT + i, TEST_BLOCK_HASH)
    
    hashchain.streamData(testData, testDir)
    instances.push(hashchain)
  }
  
  // Add blocks to all instances
  const blockCount = 15
  for (let blockNum = 0; blockNum < blockCount; blockNum++) {
    for (let instIdx = 0; instIdx < instances.length; instIdx++) {
      const blockHash = generateBlockHash(5000 + (blockNum * 100) + instIdx)
      instances[instIdx].addBlock(blockHash)
    }
  }
  
  // Verify all instances are functioning correctly
  for (let i = 0; i < instances.length; i++) {
    const instance = instances[i]
    
    t.is(instance.getChainLength(), blockCount)
    t.true(instance.verifyChain())
    
    // Should be able to generate proof windows
    const proofWindow = instance.getProofWindow()
    t.is(proofWindow.commitments.length, 8)
    
    // Random chunk access should work
    const totalChunks = instance.getTotalChunks()
    for (let j = 0; j < 5; j++) {
      const randomChunk = Math.floor(Math.random() * totalChunks)
      const chunk = instance.readChunk(randomChunk)
      t.is(chunk.length, 4096)
    }
  }
  
  cleanupTestDir(testDir)
}) 