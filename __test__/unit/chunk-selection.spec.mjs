import test from 'ava'
import { 
  selectChunksV1,
  verifyChunkSelection
} from '../../index.js'
import { 
  TEST_BLOCK_HASH,
  setupTest,
  teardownTest,
  generateBlockHash,
  generateVariedBlockHash
} from '../helpers/test-setup.mjs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Chunk Selection Algorithm Tests ===

test('selectChunksV1 produces deterministic results', (t) => {
  const blockHash = TEST_BLOCK_HASH
  const totalChunks = 100
  
  const result1 = selectChunksV1(blockHash, totalChunks)
  const result2 = selectChunksV1(blockHash, totalChunks)
  
  // Results should be identical
  t.deepEqual(result1.selectedIndices, result2.selectedIndices)
  t.is(result1.algorithmVersion, result2.algorithmVersion)
  t.deepEqual(result1.verificationHash, result2.verificationHash)
})

test('selectChunksV1 returns correct structure', (t) => {
  const blockHash = TEST_BLOCK_HASH
  const totalChunks = 100
  
  const result = selectChunksV1(blockHash, totalChunks)
  
  t.truthy(result)
  t.true(Array.isArray(result.selectedIndices))
  t.is(result.selectedIndices.length, 4) // CHUNKS_PER_BLOCK = 4
  t.is(result.algorithmVersion, 1)
  t.is(result.totalChunks, totalChunks)
  t.deepEqual(result.blockHash, blockHash)
  t.is(result.verificationHash.length, 32)
  
  // All indices should be unique and within range
  const uniqueIndices = new Set(result.selectedIndices)
  t.is(uniqueIndices.size, 4) // All indices should be unique
  
  for (const idx of result.selectedIndices) {
    t.true(idx >= 0 && idx < totalChunks)
  }
})

test('selectChunksV1 with different block hashes produces different results', (t) => {
  const blockHash1 = TEST_BLOCK_HASH
  const blockHash2 = Buffer.from('c'.repeat(64), 'hex')
  const totalChunks = 100
  
  const result1 = selectChunksV1(blockHash1, totalChunks)
  const result2 = selectChunksV1(blockHash2, totalChunks)
  
  // Results should be different
  t.notDeepEqual(result1.selectedIndices, result2.selectedIndices)
  t.notDeepEqual(result1.verificationHash, result2.verificationHash)
})

test('selectChunksV1 validates input parameters', (t) => {
  const validBlockHash = TEST_BLOCK_HASH
  const validTotalChunks = 100
  
  // Invalid block hash size
  t.throws(() => {
    selectChunksV1(Buffer.from('invalid'), validTotalChunks)
  }, { message: /Block hash must be exactly 32 bytes/ })
  
  // Zero chunks
  t.throws(() => {
    selectChunksV1(validBlockHash, 0)
  }, { message: /Total chunks must be positive/ })
  
  // Too few chunks for selection
  t.throws(() => {
    selectChunksV1(validBlockHash, 2)
  }, { message: /Total chunks \(2\) must be >= CHUNKS_PER_BLOCK \(4\)/ })
})

test('selectChunksV1 handles edge case chunk counts', (t) => {
  const blockHash = TEST_BLOCK_HASH
  
  // Test with exactly 4 chunks (minimum for selection)
  let result = selectChunksV1(blockHash, 4)
  t.is(result.selectedIndices.length, 4)
  const sorted = [...result.selectedIndices].sort()
  t.deepEqual(sorted, [0, 1, 2, 3]) // All chunks must be selected
  
  // Test with 5 chunks
  result = selectChunksV1(blockHash, 5)
  t.is(result.selectedIndices.length, 4)
  for (const idx of result.selectedIndices) {
    t.true(idx >= 0 && idx < 5)
  }
  
  // Test with large number of chunks
  result = selectChunksV1(blockHash, 10000)
  t.is(result.selectedIndices.length, 4)
  for (const idx of result.selectedIndices) {
    t.true(idx >= 0 && idx < 10000)
  }
  
  // All indices should be unique
  const uniqueIndices = new Set(result.selectedIndices)
  t.is(uniqueIndices.size, 4)
})

test('selectChunksV1 algorithm consistency across different scenarios', (t) => {
  // Test determinism across different total chunk counts with same block hash
  const blockHash = generateBlockHash(12345)
  const chunkCounts = [10, 50, 100, 500, 1000]
  
  for (const chunks of chunkCounts) {
    const result1 = selectChunksV1(blockHash, chunks)
    const result2 = selectChunksV1(blockHash, chunks)
    
    t.deepEqual(result1.selectedIndices, result2.selectedIndices)
    t.deepEqual(result1.verificationHash, result2.verificationHash)
  }
})

test('selectChunksV1 verification hash properties', (t) => {
  const blockHash = TEST_BLOCK_HASH
  const totalChunks = 100
  
  const result = selectChunksV1(blockHash, totalChunks)
  
  // Verification hash should be 32 bytes
  t.is(result.verificationHash.length, 32)
  
  // Different inputs should produce different verification hashes
  const differentResult = selectChunksV1(generateBlockHash(999), totalChunks)
  t.notDeepEqual(result.verificationHash, differentResult.verificationHash)
  
  // Same inputs should produce same verification hash
  const sameResult = selectChunksV1(blockHash, totalChunks)
  t.deepEqual(result.verificationHash, sameResult.verificationHash)
})

// === Chunk Selection Verification Tests ===

test('verifyChunkSelection validates correct selections', (t) => {
  const blockHash = TEST_BLOCK_HASH
  const totalChunks = 100
  
  const result = selectChunksV1(blockHash, totalChunks)
  
  // Should verify correctly
  t.true(verifyChunkSelection(blockHash, totalChunks, result.selectedIndices, 1))
  
  // Should fail with wrong indices
  const wrongIndices = [0, 1, 2, 3]
  t.false(verifyChunkSelection(blockHash, totalChunks, wrongIndices, 1))
  
  // Should fail with wrong algorithm version
  t.false(verifyChunkSelection(blockHash, totalChunks, result.selectedIndices, 2))
})

test('verifyChunkSelection handles edge cases', (t) => {
  const blockHash = TEST_BLOCK_HASH
  const totalChunks = 50
  const result = selectChunksV1(blockHash, totalChunks)
  
  // Test with correct data
  t.true(verifyChunkSelection(blockHash, totalChunks, result.selectedIndices, 1))
  
  // Test with wrong number of chunks in selection
  const tooFewIndices = result.selectedIndices.slice(0, 3)
  t.false(verifyChunkSelection(blockHash, totalChunks, tooFewIndices, 1))
  
  const tooManyIndices = [...result.selectedIndices, 99]
  t.false(verifyChunkSelection(blockHash, totalChunks, tooManyIndices, 1))
  
  // Test with out-of-range indices
  const outOfRangeIndices = [0, 1, 2, totalChunks]
  t.false(verifyChunkSelection(blockHash, totalChunks, outOfRangeIndices, 1))
  
  // Test with duplicate indices
  const duplicateIndices = [0, 1, 2, 2]
  t.false(verifyChunkSelection(blockHash, totalChunks, duplicateIndices, 1))
})

test('verifyChunkSelection order preservation', (t) => {
  const blockHash = TEST_BLOCK_HASH
  const totalChunks = 100
  const result = selectChunksV1(blockHash, totalChunks)
  
  // Original order should verify
  t.true(verifyChunkSelection(blockHash, totalChunks, result.selectedIndices, 1))
  
  // Shuffled order should fail (consensus requirement: order preservation)
  const shuffledIndices = [...result.selectedIndices].reverse()
  if (JSON.stringify(shuffledIndices) !== JSON.stringify(result.selectedIndices)) {
    t.false(verifyChunkSelection(blockHash, totalChunks, shuffledIndices, 1))
  }
})

test('consensus compliance across different data sizes', (t) => {
  const testSizes = [4, 8, 16, 64, 256] // Different chunk counts
  const blockHash = TEST_BLOCK_HASH
  
  for (const chunks of testSizes) {
    const result = selectChunksV1(blockHash, chunks)
    
    // Should always select exactly 4 chunks
    t.is(result.selectedIndices.length, 4)
    
    // All indices should be unique and in range
    const uniqueIndices = new Set(result.selectedIndices)
    t.is(uniqueIndices.size, 4)
    
    for (const idx of result.selectedIndices) {
      t.true(idx >= 0 && idx < chunks)
    }
    
    // Verification should pass
    t.true(verifyChunkSelection(blockHash, chunks, result.selectedIndices, 1))
  }
})

test('selectChunksV1 stress test with varied block hashes', (t) => {
  const totalChunks = 200
  const numTests = 50
  
  for (let i = 0; i < numTests; i++) {
    const blockHash = generateVariedBlockHash(i, i * 7)
    const result = selectChunksV1(blockHash, totalChunks)
    
    // Basic structure validation
    t.is(result.selectedIndices.length, 4)
    t.is(result.algorithmVersion, 1)
    t.is(result.totalChunks, totalChunks)
    
    // Range validation
    for (const idx of result.selectedIndices) {
      t.true(idx >= 0 && idx < totalChunks)
    }
    
    // Uniqueness validation
    const uniqueIndices = new Set(result.selectedIndices)
    t.is(uniqueIndices.size, 4)
    
    // Verification should pass
    t.true(verifyChunkSelection(blockHash, totalChunks, result.selectedIndices, 1))
  }
})

test('selectChunksV1 algorithm version compliance', (t) => {
  const blockHash = TEST_BLOCK_HASH
  const totalChunks = 100
  
  const result = selectChunksV1(blockHash, totalChunks)
  
  // Should always return version 1
  t.is(result.algorithmVersion, 1)
  
  // Verification with correct version should pass
  t.true(verifyChunkSelection(blockHash, totalChunks, result.selectedIndices, 1))
  
  // Verification with wrong version should fail
  t.false(verifyChunkSelection(blockHash, totalChunks, result.selectedIndices, 0))
  t.false(verifyChunkSelection(blockHash, totalChunks, result.selectedIndices, 2))
}) 