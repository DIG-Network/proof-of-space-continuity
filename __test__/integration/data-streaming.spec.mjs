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
  verifyFileNaming,
  generateTestDataWithPattern
} from '../helpers/test-setup.mjs'
import { createHash } from 'crypto'
import { existsSync } from 'fs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Data Streaming Integration Tests ===

test('stream data creates files with correct naming', (t) => {
  const testDir = createTestDir('stream_data_naming')
  const testData = createTestData(8192) // 2 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Verify files were created
  const filePaths = hashchain.getFilePaths()
  t.truthy(filePaths)
  t.is(filePaths.length, 2)
  
  const [hashchainPath, dataPath] = filePaths
  t.true(existsSync(hashchainPath))
  t.true(existsSync(dataPath))
  
  // Verify SHA256-based naming
  t.true(verifyFileNaming(filePaths, testData))
  
  // Verify file extensions
  t.true(hashchainPath.endsWith('.hashchain'))
  t.true(dataPath.endsWith('.data'))
  
  cleanupTestDir(testDir)
})

test('stream data processes chunks correctly', (t) => {
  const testDir = createTestDir('stream_data_chunks')
  const testData = createTestData(12000) // ~3 chunks with partial last chunk
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Verify chunk count
  const totalChunks = hashchain.getTotalChunks()
  t.is(totalChunks, 3) // Should be 3 chunks (4096 + 4096 + 3808 padded to 4096)
  
  // Verify we can read chunks
  for (let i = 0; i < totalChunks; i++) {
    const chunk = hashchain.readChunk(i)
    t.is(chunk.length, 4096) // All chunks should be 4KB
  }
  
  cleanupTestDir(testDir)
})

test('stream data creates anchored commitment', (t) => {
  const testDir = createTestDir('stream_data_commitment')
  const testData = createTestData(4096)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const anchoredCommitment = hashchain.getAnchoredCommitment()
  const currentCommitment = hashchain.getCurrentCommitment()
  
  t.truthy(anchoredCommitment)
  t.truthy(currentCommitment)
  t.is(anchoredCommitment.length, 32) // 32-byte hash
  t.is(currentCommitment.length, 32) // 32-byte hash
  
  // Initially, current commitment should equal anchored commitment
  t.deepEqual(anchoredCommitment, currentCommitment)
  
  cleanupTestDir(testDir)
})

test('stream data fails if already has data', (t) => {
  const testDir = createTestDir('stream_data_duplicate')
  const testData = createTestData(4096)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Try to stream data again
  t.throws(() => {
    hashchain.streamData(testData, testDir)
  }, { message: /HashChain already has data/ })
  
  cleanupTestDir(testDir)
})

test('stream data with different patterns produces different files', (t) => {
  const testDir = createTestDir('stream_data_patterns')
  
  // Test different data patterns
  const patterns = ['sequential', 'deterministic']
  const hashchains = []
  const filePaths = []
  
  for (const pattern of patterns) {
    const testData = generateTestDataWithPattern(4, pattern) // 4 chunks
    const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
    
    hashchain.streamData(testData, testDir)
    hashchains.push(hashchain)
    filePaths.push(hashchain.getFilePaths())
    
    // Verify basic properties
    t.is(hashchain.getTotalChunks(), 4)
    t.truthy(hashchain.getAnchoredCommitment())
  }
  
  // Different patterns should create different files
  t.notDeepEqual(filePaths[0], filePaths[1])
  
  // Different data should create different commitments
  t.notDeepEqual(
    hashchains[0].getAnchoredCommitment(),
    hashchains[1].getAnchoredCommitment()
  )
  
  cleanupTestDir(testDir)
})

test('stream data handles various file sizes', (t) => {
  const testDir = createTestDir('stream_data_sizes')
  
  const testSizes = [
    { bytes: 1, expectedChunks: 1 },      // Tiny file
    { bytes: 4096, expectedChunks: 1 },   // Exactly 1 chunk
    { bytes: 4097, expectedChunks: 2 },   // Just over 1 chunk
    { bytes: 8192, expectedChunks: 2 },   // Exactly 2 chunks
    { bytes: 16383, expectedChunks: 4 },  // Just under 4 chunks
    { bytes: 16384, expectedChunks: 4 }   // Exactly 4 chunks
  ]
  
  for (const { bytes, expectedChunks } of testSizes) {
    const testData = createTestData(bytes)
    const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
    
    hashchain.streamData(testData, testDir)
    
    t.is(hashchain.getTotalChunks(), expectedChunks, 
         `${bytes} bytes should create ${expectedChunks} chunks`)
    
    // Verify all chunks can be read
    for (let i = 0; i < expectedChunks; i++) {
      const chunk = hashchain.readChunk(i)
      t.is(chunk.length, 4096)
    }
  }
  
  cleanupTestDir(testDir)
})

test('stream data preserves data integrity across chunk boundaries', (t) => {
  const testDir = createTestDir('stream_data_integrity')
  
  // Create test data with known pattern
  const testData = Buffer.alloc(10000) // ~2.5 chunks
  for (let i = 0; i < testData.length; i++) {
    testData[i] = i % 256
  }
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  t.is(hashchain.getTotalChunks(), 3) // Should be 3 chunks with padding
  
  // Read all chunks and reconstruct data
  const reconstructed = Buffer.alloc(testData.length)
  let offset = 0
  
  for (let i = 0; i < hashchain.getTotalChunks(); i++) {
    const chunk = hashchain.readChunk(i)
    const copyLength = Math.min(chunk.length, testData.length - offset)
    chunk.copy(reconstructed, offset, 0, copyLength)
    offset += copyLength
  }
  
  // Original data should be preserved exactly
  t.deepEqual(reconstructed, testData)
  
  cleanupTestDir(testDir)
})

test('stream data creates valid hashchain info', (t) => {
  const testDir = createTestDir('stream_data_info')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Before streaming
  let info = hashchain.getChainInfo()
  t.is(info.status, 'uninitialized')
  t.is(info.totalChunks, 0)
  
  // After streaming
  hashchain.streamData(testData, testDir)
  info = hashchain.getChainInfo()
  
  t.is(info.status, 'initialized')
  t.is(info.totalChunks, 5)
  t.is(info.chainLength, 0)
  t.is(info.chunkSizeBytes, 4096)
  t.truthy(info.hashchainFilePath)
  t.truthy(info.dataFilePath)
  t.truthy(info.anchoredCommitment)
  t.truthy(info.currentCommitment)
  t.is(info.anchoredCommitment, info.currentCommitment) // Should be equal initially
  t.false(info.proofWindowReady)
  t.is(info.blocksUntilProofReady, 8)
  
  cleanupTestDir(testDir)
})

test('stream data enforces consensus chunk limits', (t) => {
  const testDir = createTestDir('stream_data_limits')
  
  // Test minimum chunks (empty data should fail)
  const emptyData = Buffer.alloc(0)
  const hashchain1 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  t.throws(() => {
    hashchain1.streamData(emptyData, testDir)
  }, { message: /Too few chunks: 0 < 1/ })
  
  // Test valid minimum (1 byte should work)
  const minData = Buffer.alloc(1)
  const hashchain2 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  t.notThrows(() => {
    hashchain2.streamData(minData, testDir)
  })
  
  t.is(hashchain2.getTotalChunks(), 1)
  
  cleanupTestDir(testDir)
})

test('stream data handles concurrent instances', (t) => {
  const testDir = createTestDir('stream_data_concurrent')
  
  // Create multiple instances with different data
  const instances = []
  const testDataSets = [
    createTestData(4096),
    createTestData(8192),
    createTestData(12288)
  ]
  
  for (let i = 0; i < testDataSets.length; i++) {
    const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT + i, TEST_BLOCK_HASH)
    hashchain.streamData(testDataSets[i], testDir)
    instances.push(hashchain)
  }
  
  // All instances should be independent
  for (let i = 0; i < instances.length; i++) {
    t.is(instances[i].getTotalChunks(), i + 1) // 1, 2, 3 chunks respectively
    t.truthy(instances[i].getFilePaths())
    
    // Each should have different file paths
    for (let j = i + 1; j < instances.length; j++) {
      t.notDeepEqual(instances[i].getFilePaths(), instances[j].getFilePaths())
    }
  }
  
  cleanupTestDir(testDir)
})

test('stream data creates deterministic results', (t) => {
  const testDir = createTestDir('stream_data_deterministic')
  
  const testData = Buffer.from('Deterministic test data for streaming', 'utf-8')
  
  // Stream same data multiple times
  const hashchain1 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const hashchain2 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  hashchain1.streamData(testData, testDir)
  hashchain2.streamData(testData, testDir)
  
  // Should produce identical results
  t.is(hashchain1.getTotalChunks(), hashchain2.getTotalChunks())
  t.deepEqual(hashchain1.getAnchoredCommitment(), hashchain2.getAnchoredCommitment())
  t.deepEqual(hashchain1.getCurrentCommitment(), hashchain2.getCurrentCommitment())
  
  // File paths should be identical (both based on SHA256 of data)
  const paths1 = hashchain1.getFilePaths()
  const paths2 = hashchain2.getFilePaths()
  
  // Extract just the filenames (without full paths)
  const filename1 = paths1[0].split('/').pop()
  const filename2 = paths2[0].split('/').pop()
  
  t.is(filename1, filename2) // Same filename based on data hash
  
  cleanupTestDir(testDir)
})

test('stream data validation and error handling', (t) => {
  const testDir = createTestDir('stream_data_validation')
  
  const testData = createTestData(4096)
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Should work normally
  t.notThrows(() => {
    hashchain.streamData(testData, testDir)
  })
  
  // Directory creation should handle existing directories
  t.notThrows(() => {
    const hashchain2 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT + 1, TEST_BLOCK_HASH)
    const differentData = createTestData(8192)
    hashchain2.streamData(differentData, testDir)
  })
  
  cleanupTestDir(testDir)
})

test('file paths contain correct SHA256 hash', (t) => {
  const testDir = createTestDir('sha256_naming')
  const testData = Buffer.from('Hello, HashChain World!', 'utf-8')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const filePaths = hashchain.getFilePaths()
  const expectedHash = createHash('sha256').update(testData).digest('hex')
  
  t.true(filePaths[0].includes(expectedHash)) // .hashchain file
  t.true(filePaths[1].includes(expectedHash)) // .data file
  
  cleanupTestDir(testDir)
})

test('handles extremely small data files', (t) => {
  const testDir = createTestDir('tiny_file')
  const tinyData = Buffer.from('X') // Just 1 byte
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(tinyData, testDir)
  
  t.is(hashchain.getTotalChunks(), 1) // Should create 1 chunk (padded to 4KB)
  
  const chunk = hashchain.readChunk(0)
  t.is(chunk.length, 4096) // Should be padded to full chunk size
  t.is(chunk[0], 88) // 'X' = 88
  t.is(chunk[1], 0) // Remaining should be zero-padded
  
  cleanupTestDir(testDir)
}) 