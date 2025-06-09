import test from 'ava'
import { HashChain, selectChunksV1 } from '../../index.js'
import { 
  TEST_PUBLIC_KEY, 
  TEST_BLOCK_HASH, 
  TEST_BLOCK_HEIGHT,
  createTestData,
  createTestDir,
  cleanupTestDir,
  setupTest,
  teardownTest,
  generateBlockHash
} from '../helpers/test-setup.mjs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Boundary Conditions and Edge Cases ===

test('HashChain handles minimum viable file size', (t) => {
  const testDir = createTestDir('minimum_size')
  const testData = createTestData(16384) // Exactly 4 chunks (minimum for selection)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  t.is(hashchain.getTotalChunks(), 4)
  
  // Should be able to add blocks
  const blockHash = generateBlockHash(1)
  const commitment = hashchain.addBlock(blockHash)
  
  t.truthy(commitment)
  t.is(commitment.selectedChunks.length, 4)
  
  // All chunks should be selected (since we only have 4)
  const sortedIndices = [...commitment.selectedChunks].sort()
  t.deepEqual(sortedIndices, [0, 1, 2, 3])
  
  cleanupTestDir(testDir)
})

test('HashChain chunk selection with prime number chunks', (t) => {
  const testDir = createTestDir('prime_chunks')
  const primeChunks = 17 // Prime number for interesting selection patterns
  const testData = createTestData(primeChunks * 4096)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  t.is(hashchain.getTotalChunks(), primeChunks)
  
  // Add multiple blocks and verify selection patterns
  const allSelectedChunks = new Set()
  
  for (let i = 0; i < 10; i++) {
    const blockHash = generateBlockHash(100 + i)
    const commitment = hashchain.addBlock(blockHash)
    
    t.is(commitment.selectedChunks.length, 4)
    
    // All chunks should be in valid range
    for (const chunkIdx of commitment.selectedChunks) {
      t.true(chunkIdx >= 0 && chunkIdx < primeChunks)
      allSelectedChunks.add(chunkIdx)
    }
    
    // No duplicates within single selection
    const uniqueChunks = new Set(commitment.selectedChunks)
    t.is(uniqueChunks.size, 4)
  }
  
  // Should have good distribution across prime number of chunks
  t.true(allSelectedChunks.size >= 8, `Only ${allSelectedChunks.size} unique chunks selected`)
  
  cleanupTestDir(testDir)
})

test('HashChain handles maximum practical chunk count', (t) => {
  const testDir = createTestDir('max_chunks')
  
  // Test with a large number of chunks (but practical for testing)
  const largeChunkCount = 1000
  const testData = createTestData(largeChunkCount * 4096)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  t.is(hashchain.getTotalChunks(), largeChunkCount)
  
  // Should be able to add blocks efficiently
  const startTime = Date.now()
  for (let i = 0; i < 5; i++) {
    const blockHash = generateBlockHash(200 + i)
    const commitment = hashchain.addBlock(blockHash)
    
    t.is(commitment.selectedChunks.length, 4)
    
    // All chunks should be in valid range
    for (const chunkIdx of commitment.selectedChunks) {
      t.true(chunkIdx >= 0 && chunkIdx < largeChunkCount)
    }
  }
  const elapsedTime = Date.now() - startTime
  
  // Should be reasonably fast even with many chunks
  t.true(elapsedTime < 5000, `Processing with ${largeChunkCount} chunks took ${elapsedTime}ms`)
  
  cleanupTestDir(testDir)
})

test('HashChain file paths use correct SHA256 content addressing', (t) => {
  const testDir = createTestDir('content_addressing')
  
  // Same content should always produce same file paths
  const testContent = Buffer.from('Deterministic content for hashing test', 'utf-8')
  
  const hashchain1 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const hashchain2 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT + 1, TEST_BLOCK_HASH)
  
  hashchain1.streamData(testContent, testDir)
  hashchain2.streamData(testContent, testDir)
  
  const paths1 = hashchain1.getFilePaths()
  const paths2 = hashchain2.getFilePaths()
  
  // File names should be identical (based on content hash)
  const filename1 = paths1[0].split('/').pop()
  const filename2 = paths2[0].split('/').pop()
  
  t.is(filename1, filename2) // Same content = same filename
  
  // But different content should produce different names
  const differentContent = Buffer.from('Different content for testing', 'utf-8')
  const hashchain3 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain3.streamData(differentContent, testDir)
  
  const paths3 = hashchain3.getFilePaths()
  const filename3 = paths3[0].split('/').pop()
  
  t.not(filename1, filename3) // Different content = different filename
  
  cleanupTestDir(testDir)
})

test('HashChain commitment chain maintains cryptographic integrity', (t) => {
  const testDir = createTestDir('crypto_integrity')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const commitments = []
  
  // Add multiple blocks and collect commitments
  for (let i = 0; i < 8; i++) {
    const blockHash = generateBlockHash(300 + i)
    const commitment = hashchain.addBlock(blockHash)
    commitments.push(commitment)
    
    // Each commitment hash should be exactly 32 bytes
    t.is(commitment.commitmentHash.length, 32)
    
    // Should not be all zeros (cryptographically unlikely)
    const allZeros = Buffer.alloc(32, 0)
    t.notDeepEqual(commitment.commitmentHash, allZeros)
  }
  
  // All commitment hashes should be unique
  const commitmentHashes = commitments.map(c => c.commitmentHash.toString('hex'))
  const uniqueHashes = new Set(commitmentHashes)
  t.is(uniqueHashes.size, commitments.length)
  
  // Chain linkage should be perfect
  for (let i = 1; i < commitments.length; i++) {
    t.deepEqual(
      commitments[i].previousCommitment,
      commitments[i - 1].commitmentHash
    )
  }
  
  cleanupTestDir(testDir)
})

test('chunk selection edge cases with different chunk counts', (t) => {
  // Test edge cases for chunk selection algorithm
  const edgeCases = [
    { chunks: 4, description: 'exact minimum' },
    { chunks: 5, description: 'just over minimum' },
    { chunks: 7, description: 'prime number less than 8' },
    { chunks: 8, description: 'power of 2' },
    { chunks: 15, description: 'odd number' },
    { chunks: 16, description: 'power of 2' },
    { chunks: 100, description: 'round number' },
    { chunks: 127, description: 'just under power of 2' },
    { chunks: 128, description: 'power of 2' }
  ]
  
  for (const { chunks, description } of edgeCases) {
    const result = selectChunksV1(TEST_BLOCK_HASH, chunks)
    
    t.is(result.selectedIndices.length, 4, `${description}: wrong selection count`)
    t.is(result.totalChunks, chunks, `${description}: wrong total chunks`)
    
    // All indices should be in valid range
    for (const idx of result.selectedIndices) {
      t.true(idx >= 0 && idx < chunks, `${description}: index ${idx} out of range`)
    }
    
    // All indices should be unique
    const uniqueIndices = new Set(result.selectedIndices)
    t.is(uniqueIndices.size, 4, `${description}: duplicate indices`)
    
    // Should be deterministic
    const result2 = selectChunksV1(TEST_BLOCK_HASH, chunks)
    t.deepEqual(result.selectedIndices, result2.selectedIndices, `${description}: not deterministic`)
  }
})

test('boundary conditions with exactly chunk size boundaries', (t) => {
  const testDir = createTestDir('chunk_boundaries')
  
  // Test data sizes that align exactly with chunk boundaries
  const boundarySizes = [
    4096,      // Exactly 1 chunk
    8192,      // Exactly 2 chunks
    12288,     // Exactly 3 chunks
    16384,     // Exactly 4 chunks (minimum)
    20480,     // Exactly 5 chunks
  ]
  
  for (const size of boundarySizes) {
    const expectedChunks = size / 4096
    const testData = createTestData(size)
    
    const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
    hashchain.streamData(testData, testDir)
    
    t.is(hashchain.getTotalChunks(), expectedChunks)
    
    // Should be able to read all chunks
    for (let i = 0; i < expectedChunks; i++) {
      const chunk = hashchain.readChunk(i)
      t.is(chunk.length, 4096)
    }
    
    // Reading beyond should fail
    t.throws(() => {
      hashchain.readChunk(expectedChunks)
    }, { message: /out of range/ })
    
    // If we have enough chunks, test block addition
    if (expectedChunks >= 4) {
      const commitment = hashchain.addBlock(generateBlockHash(1))
      t.is(commitment.selectedChunks.length, 4)
    }
  }
  
  cleanupTestDir(testDir)
})

test('error boundaries and recovery scenarios', (t) => {
  const testDir = createTestDir('error_boundaries')
  
  // Test various error conditions don't corrupt state
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const testData = createTestData(16384) // 4 chunks
  
  hashchain.streamData(testData, testDir)
  
  // Valid operation baseline
  hashchain.addBlock(generateBlockHash(1))
  t.is(hashchain.getChainLength(), 1)
  
  // Error conditions that should not affect state
  const errorConditions = [
    () => hashchain.addBlock(Buffer.from('invalid')), // Wrong size
    () => hashchain.addBlock(Buffer.alloc(31)), // Too short
    () => hashchain.addBlock(Buffer.alloc(33)), // Too long
    () => hashchain.readChunk(-1), // Negative index
    () => hashchain.readChunk(999), // Out of range
  ]
  
  for (const errorFn of errorConditions) {
    const beforeLength = hashchain.getChainLength()
    const beforeCommitment = hashchain.getCurrentCommitment()
    
    // Error should throw
    t.throws(() => errorFn())
    
    // State should be unchanged
    t.is(hashchain.getChainLength(), beforeLength)
    t.deepEqual(hashchain.getCurrentCommitment(), beforeCommitment)
    t.true(hashchain.verifyChain())
  }
  
  // Should still be able to add valid blocks after errors
  hashchain.addBlock(generateBlockHash(2))
  t.is(hashchain.getChainLength(), 2)
  t.true(hashchain.verifyChain())
  
  cleanupTestDir(testDir)
}) 