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
  generateTestDataWithPattern
} from '../helpers/test-setup.mjs'
import { createHash } from 'crypto'
import { promises as fs } from 'fs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === File I/O Unit Tests ===

test('readChunk reads correct data from file', (t) => {
  const testDir = createTestDir('read_chunk_test')
  const testData = createTestData(16384) // 4 chunks with known pattern
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Read each chunk and verify pattern
  for (let i = 0; i < 4; i++) {
    const chunk = hashchain.readChunk(i)
    
    t.is(chunk.length, 4096)
    
    // Verify the pattern matches expected data
    const expectedStart = i * 4096
    for (let j = 0; j < 100; j++) { // Check first 100 bytes
      t.is(chunk[j], testData[expectedStart + j])
    }
  }
  
  cleanupTestDir(testDir)
})

test('readChunk validates chunk index bounds', (t) => {
  const testDir = createTestDir('read_chunk_bounds')
  const testData = createTestData(8192) // 2 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Valid reads should work
  t.notThrows(() => {
    hashchain.readChunk(0)
    hashchain.readChunk(1)
  })
  
  // Invalid reads should throw
  t.throws(() => {
    hashchain.readChunk(2) // Beyond range
  }, { message: /out of range/ })
  
  t.throws(() => {
    hashchain.readChunk(-1) // Negative index
  }, { message: /out of range/ })
  
  t.throws(() => {
    hashchain.readChunk(999) // Way beyond range
  }, { message: /out of range/ })
  
  cleanupTestDir(testDir)
})

test('readChunk handles last chunk padding correctly', (t) => {
  const testDir = createTestDir('read_chunk_padding')
  
  // Create data that doesn't align perfectly with chunk boundaries
  const oddSize = 4096 + 100 // 1 full chunk + 100 bytes
  const testData = Buffer.alloc(oddSize)
  for (let i = 0; i < oddSize; i++) {
    testData[i] = i % 256
  }
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  t.is(hashchain.getTotalChunks(), 2) // Should be 2 chunks
  
  // First chunk should be full
  const chunk0 = hashchain.readChunk(0)
  t.is(chunk0.length, 4096)
  
  // Last chunk should be padded to full size
  const chunk1 = hashchain.readChunk(1)
  t.is(chunk1.length, 4096)
  
  // First 100 bytes should match original data
  for (let i = 0; i < 100; i++) {
    t.is(chunk1[i], testData[4096 + i])
  }
  
  // Remaining bytes should be zero padding
  for (let i = 100; i < 4096; i++) {
    t.is(chunk1[i], 0)
  }
  
  cleanupTestDir(testDir)
})

test('readChunk fails without data file', (t) => {
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Should fail when no data has been streamed
  t.throws(() => {
    hashchain.readChunk(0)
  }, { message: /No data file available/ })
})

test('file path operations return correct values', (t) => {
  const testDir = createTestDir('file_paths')
  const testData = createTestData(4096)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Before streaming
  t.is(hashchain.getFilePaths(), null)
  t.is(hashchain.getDataFilePath(), null)
  
  // After streaming
  hashchain.streamData(testData, testDir)
  
  const filePaths = hashchain.getFilePaths()
  const dataFilePath = hashchain.getDataFilePath()
  
  t.truthy(filePaths)
  t.is(filePaths.length, 2)
  t.truthy(dataFilePath)
  
  // Should return .hashchain and .data files
  t.true(filePaths[0].endsWith('.hashchain'))
  t.true(filePaths[1].endsWith('.data'))
  t.true(dataFilePath.endsWith('.data'))
  
  // Data file path should match
  t.is(filePaths[1], dataFilePath)
  
  cleanupTestDir(testDir)
})

test('file operations handle concurrent access', (t) => {
  const testDir = createTestDir('concurrent_file_access')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Read multiple chunks concurrently (simulate concurrent access)
  const reads = []
  for (let i = 0; i < 5; i++) {
    reads.push(() => hashchain.readChunk(i))
  }
  
  // All reads should succeed
  const results = reads.map(readFn => readFn())
  
  for (let i = 0; i < results.length; i++) {
    t.is(results[i].length, 4096)
    
    // Verify data integrity
    const expectedByte = (i * 4096) % 256
    t.is(results[i][0], expectedByte)
  }
  
  cleanupTestDir(testDir)
})

test('chunk reading performance is acceptable', (t) => {
  const testDir = createTestDir('chunk_read_performance')
  const testData = createTestData(40960) // 10 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Measure read performance
  const startTime = Date.now()
  
  // Read all chunks multiple times
  for (let round = 0; round < 5; round++) {
    for (let i = 0; i < 10; i++) {
      const chunk = hashchain.readChunk(i)
      t.is(chunk.length, 4096)
    }
  }
  
  const elapsedTime = Date.now() - startTime
  
  // 50 chunk reads should be reasonably fast
  t.true(elapsedTime < 1000, `Reading 50 chunks took ${elapsedTime}ms, should be < 1000ms`)
  
  cleanupTestDir(testDir)
})

test('data file integrity across operations', (t) => {
  const testDir = createTestDir('data_integrity')
  
  // Create data with distinctive pattern
  const testData = Buffer.alloc(12288) // 3 chunks
  for (let i = 0; i < testData.length; i++) {
    testData[i] = (i * 7 + 13) % 256 // Distinctive pattern
  }
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Read chunks and verify they match original pattern
  for (let chunkIdx = 0; chunkIdx < 3; chunkIdx++) {
    const chunk = hashchain.readChunk(chunkIdx)
    
    for (let i = 0; i < 4096; i++) {
      const originalIndex = chunkIdx * 4096 + i
      if (originalIndex < testData.length) {
        t.is(chunk[i], testData[originalIndex], 
             `Mismatch at chunk ${chunkIdx}, byte ${i}`)
      }
    }
  }
  
  cleanupTestDir(testDir)
})

test('random access patterns work correctly', (t) => {
  const testDir = createTestDir('random_access')
  const testData = createTestData(32768) // 8 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Random access pattern
  const accessPattern = [7, 0, 3, 5, 1, 6, 2, 4, 0, 7]
  
  for (const chunkIdx of accessPattern) {
    const chunk = hashchain.readChunk(chunkIdx)
    
    t.is(chunk.length, 4096)
    
    // Verify first few bytes match expected pattern
    const expectedStart = chunkIdx * 4096
    for (let i = 0; i < 10; i++) {
      t.is(chunk[i], testData[expectedStart + i])
    }
  }
  
  cleanupTestDir(testDir)
})

test('file size calculations are accurate', async (t) => {
  const testDir = createTestDir('file_sizes')
  const testData = createTestData(20480) // 5 chunks = 20KB
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const filePaths = hashchain.getFilePaths()
  const info = hashchain.getChainInfo()
  
  // Check actual file sizes
  const dataFileStats = await fs.stat(filePaths[1])
  const hashchainFileStats = await fs.stat(filePaths[0])
  
  // Data file should be exactly 5 * 4096 bytes
  t.is(dataFileStats.size, 5 * 4096)
  
  // Chain info should report correct sizes
  t.is(info.dataFileSizeBytes, dataFileStats.size)
  t.is(info.hashchainFileSizeBytes, hashchainFileStats.size)
  
  // Storage calculation should be correct
  const expectedStorageMB = (5 * 4096) / (1024 * 1024)
  t.is(info.totalStorageMb, expectedStorageMB)
  
  cleanupTestDir(testDir)
})

test('chunk data consistency across multiple reads', (t) => {
  const testDir = createTestDir('read_consistency')
  const testData = generateTestDataWithPattern(6, 'deterministic')
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Read the same chunk multiple times
  const chunk2_read1 = hashchain.readChunk(2)
  const chunk2_read2 = hashchain.readChunk(2)
  const chunk2_read3 = hashchain.readChunk(2)
  
  // All reads should be identical
  t.deepEqual(chunk2_read1, chunk2_read2)
  t.deepEqual(chunk2_read2, chunk2_read3)
  
  // Verify hash consistency
  const hash1 = createHash('sha256').update(chunk2_read1).digest()
  const hash2 = createHash('sha256').update(chunk2_read2).digest()
  const hash3 = createHash('sha256').update(chunk2_read3).digest()
  
  t.deepEqual(hash1, hash2)
  t.deepEqual(hash2, hash3)
  
  cleanupTestDir(testDir)
})

test('file operations after chain modifications', (t) => {
  const testDir = createTestDir('file_ops_after_mods')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Read chunks before adding blocks
  const beforeChunk = hashchain.readChunk(1)
  
  // Add some blocks
  hashchain.addBlock(Buffer.from('a'.repeat(64), 'hex'))
  hashchain.addBlock(Buffer.from('b'.repeat(64), 'hex'))
  
  // Read same chunk after modifications
  const afterChunk = hashchain.readChunk(1)
  
  // Chunk data should be unchanged
  t.deepEqual(beforeChunk, afterChunk)
  
  // File operations should still work normally
  for (let i = 0; i < 4; i++) {
    const chunk = hashchain.readChunk(i)
    t.is(chunk.length, 4096)
  }
  
  cleanupTestDir(testDir)
}) 