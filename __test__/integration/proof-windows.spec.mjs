import test from 'ava'
import { HashChain, verifyProof } from '../../index.js'
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

// === Proof Window Integration Tests ===

test('proof window generation workflow', (t) => {
  const testDir = createTestDir('proof_window_workflow')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add exactly 8 blocks to create minimum proof window
  for (let i = 0; i < 8; i++) {
    const blockHash = generateBlockHash(1000 + i)
    hashchain.addBlock(blockHash)
  }
  
  // Should now be able to generate proof window
  const proofWindow = hashchain.getProofWindow()
  
  t.truthy(proofWindow)
  t.is(proofWindow.commitments.length, 8)
  t.is(proofWindow.merkleProofs.length, 32) // 8 blocks * 4 chunks per block
  t.truthy(proofWindow.startCommitment)
  t.truthy(proofWindow.endCommitment)
  
  // Start commitment should be anchored commitment (first 8 blocks)
  t.deepEqual(proofWindow.startCommitment, hashchain.getAnchoredCommitment())
  
  // End commitment should be current commitment
  t.deepEqual(proofWindow.endCommitment, hashchain.getCurrentCommitment())
  
  cleanupTestDir(testDir)
})

test('proof window with more than 8 blocks', (t) => {
  const testDir = createTestDir('proof_window_extended')
  const testData = createTestData(24576) // 6 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 12 blocks (more than proof window size)
  const blockHashes = []
  for (let i = 0; i < 12; i++) {
    const blockHash = generateVariedBlockHash(i, i * 17)
    blockHashes.push(blockHash)
    hashchain.addBlock(blockHash)
  }
  
  const proofWindow = hashchain.getProofWindow()
  
  // Should still contain exactly 8 commitments (the last 8)
  t.is(proofWindow.commitments.length, 8)
  t.is(proofWindow.merkleProofs.length, 32)
  
  // Start commitment should NOT be anchored commitment
  t.notDeepEqual(proofWindow.startCommitment, hashchain.getAnchoredCommitment())
  
  // End commitment should be current commitment
  t.deepEqual(proofWindow.endCommitment, hashchain.getCurrentCommitment())
  
  // Verify the last 8 block hashes are in the proof window
  const proofBlockHashes = proofWindow.commitments.map(c => c.blockHash)
  const expectedBlockHashes = blockHashes.slice(-8) // Last 8 blocks
  
  for (let i = 0; i < 8; i++) {
    t.deepEqual(proofBlockHashes[i], expectedBlockHashes[i])
  }
  
  cleanupTestDir(testDir)
})

test('proof window commitment chain linkage', (t) => {
  const testDir = createTestDir('proof_window_linkage')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 10 blocks
  for (let i = 0; i < 10; i++) {
    hashchain.addBlock(generateBlockHash(2000 + i))
  }
  
  const proofWindow = hashchain.getProofWindow()
  
  // Verify chain linkage within proof window
  let expectedPrevious = proofWindow.startCommitment
  
  for (let i = 0; i < proofWindow.commitments.length; i++) {
    const commitment = proofWindow.commitments[i]
    
    t.deepEqual(commitment.previousCommitment, expectedPrevious)
    expectedPrevious = commitment.commitmentHash
  }
  
  // Final commitment should match end commitment
  t.deepEqual(expectedPrevious, proofWindow.endCommitment)
  
  cleanupTestDir(testDir)
})

test('proof window merkle proof structure', (t) => {
  const testDir = createTestDir('proof_window_merkle')
  const testData = createTestData(32768) // 8 chunks for variety
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 8 blocks
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(generateVariedBlockHash(i, i * 23))
  }
  
  const proofWindow = hashchain.getProofWindow()
  
  // Should have exactly one merkle proof per selected chunk
  t.is(proofWindow.merkleProofs.length, 32) // 8 blocks * 4 chunks
  
  // Each merkle proof should be properly formatted
  for (const proof of proofWindow.merkleProofs) {
    t.truthy(proof)
    t.is(proof.length % 33, 0) // Each proof element is 32 bytes + 1 direction byte
    t.true(proof.length > 0)
  }
  
  // Each commitment should have exactly 4 chunk selections
  for (const commitment of proofWindow.commitments) {
    t.is(commitment.selectedChunks.length, 4)
    t.is(commitment.chunkHashes.length, 4)
    
    // All selected chunks should be within valid range
    for (const chunkIdx of commitment.selectedChunks) {
      t.true(chunkIdx >= 0 && chunkIdx < 8)
    }
  }
  
  cleanupTestDir(testDir)
})

test('proof window verification workflow', (t) => {
  const testDir = createTestDir('proof_verification_workflow')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 8 blocks
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(generateBlockHash(3000 + i))
  }
  
  const proofWindow = hashchain.getProofWindow()
  const anchoredCommitment = hashchain.getAnchoredCommitment()
  
  // Create mock merkle root (in real implementation this would be calculated)
  const merkleRoot = Buffer.alloc(32, 0xaa)
  
  // Basic verification should handle the structure correctly
  t.notThrows(() => {
    verifyProof(proofWindow, anchoredCommitment, merkleRoot, 5)
  })
  
  cleanupTestDir(testDir)
})

test('proof window handles different chunk patterns', (t) => {
  const testDir = createTestDir('proof_window_patterns')
  const testData = createTestData(40960) // 10 chunks for variety
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add blocks with varied patterns to ensure different chunk selections
  const selectedChunksPerBlock = []
  for (let i = 0; i < 8; i++) {
    const blockHash = generateVariedBlockHash(i, i * 37 + 7) // Varied pattern
    const commitment = hashchain.addBlock(blockHash)
    selectedChunksPerBlock.push(new Set(commitment.selectedChunks))
  }
  
  const proofWindow = hashchain.getProofWindow()
  
  // Verify we got good variety in chunk selection
  const allSelectedChunks = new Set()
  for (const commitment of proofWindow.commitments) {
    for (const chunkIdx of commitment.selectedChunks) {
      allSelectedChunks.add(chunkIdx)
    }
  }
  
  // Should have selected chunks from across the range
  t.true(allSelectedChunks.size >= 8, `Only ${allSelectedChunks.size} unique chunks selected`)
  
  // Each block should have different selections (with high probability)
  let uniqueSelections = 0
  for (let i = 0; i < selectedChunksPerBlock.length; i++) {
    let isUnique = true
    for (let j = i + 1; j < selectedChunksPerBlock.length; j++) {
      // Check if selections are identical
      const set1 = selectedChunksPerBlock[i]
      const set2 = selectedChunksPerBlock[j]
      if (set1.size === set2.size && [...set1].every(x => set2.has(x))) {
        isUnique = false
        break
      }
    }
    if (isUnique) uniqueSelections++
  }
  
  t.true(uniqueSelections >= 6, `Only ${uniqueSelections} unique selections out of 8`)
  
  cleanupTestDir(testDir)
})

test('proof window generation performance', (t) => {
  const testDir = createTestDir('proof_window_performance')
  const testData = createTestData(81920) // 20 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 8 blocks
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(generateBlockHash(4000 + i))
  }
  
  // Measure proof generation performance
  const startTime = Date.now()
  const proofWindow = hashchain.getProofWindow()
  const elapsedTime = Date.now() - startTime
  
  // Should generate proof window reasonably quickly
  t.true(elapsedTime < 1000, `Proof generation took ${elapsedTime}ms, should be < 1000ms`)
  
  // Verify structure is correct
  t.is(proofWindow.commitments.length, 8)
  t.is(proofWindow.merkleProofs.length, 32)
  
  cleanupTestDir(testDir)
})

test('proof window with edge case block sequences', (t) => {
  const testDir = createTestDir('proof_window_edge_cases')
  const testData = createTestData(28672) // 7 chunks (prime number)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add exactly 8 blocks (minimum for proof window)
  const blockHashes = []
  for (let i = 0; i < 8; i++) {
    // Use edge case patterns
    const pattern = i % 3 === 0 ? 0x00 : i % 3 === 1 ? 0xFF : 0xAA
    const blockHash = Buffer.alloc(32, pattern)
    blockHashes.push(blockHash)
    hashchain.addBlock(blockHash)
  }
  
  const proofWindow = hashchain.getProofWindow()
  
  // Should handle edge case block hashes correctly
  t.is(proofWindow.commitments.length, 8)
  
  // Verify each commitment references the correct block hash
  for (let i = 0; i < 8; i++) {
    t.deepEqual(proofWindow.commitments[i].blockHash, blockHashes[i])
  }
  
  // Should still generate valid chunk selections despite edge case hashes
  for (const commitment of proofWindow.commitments) {
    t.is(commitment.selectedChunks.length, 4)
    for (const chunkIdx of commitment.selectedChunks) {
      t.true(chunkIdx >= 0 && chunkIdx < 7)
    }
  }
  
  cleanupTestDir(testDir)
})

test('proof window state consistency across operations', (t) => {
  const testDir = createTestDir('proof_window_consistency')
  const testData = createTestData(24576) // 6 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 8 blocks
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(generateBlockHash(5000 + i))
  }
  
  // Generate proof window multiple times
  const proofWindow1 = hashchain.getProofWindow()
  const proofWindow2 = hashchain.getProofWindow()
  
  // Should be identical
  t.deepEqual(proofWindow1.startCommitment, proofWindow2.startCommitment)
  t.deepEqual(proofWindow1.endCommitment, proofWindow2.endCommitment)
  t.is(proofWindow1.commitments.length, proofWindow2.commitments.length)
  
  // Add another block
  hashchain.addBlock(generateBlockHash(5008))
  
  const proofWindow3 = hashchain.getProofWindow()
  
  // Should be different from previous proof windows
  t.notDeepEqual(proofWindow1.endCommitment, proofWindow3.endCommitment)
  t.deepEqual(proofWindow3.endCommitment, hashchain.getCurrentCommitment())
  
  cleanupTestDir(testDir)
})

test('proof window integration with chain verification', (t) => {
  const testDir = createTestDir('proof_window_chain_verification')
  const testData = createTestData(32768) // 8 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add blocks and verify chain remains valid
  for (let i = 0; i < 10; i++) {
    hashchain.addBlock(generateVariedBlockHash(i, i * 19))
    
    // Chain should remain valid after each block
    t.true(hashchain.verifyChain())
    
    // Once we have enough blocks, proof window should be available
    if (i >= 7) {
      t.notThrows(() => {
        const proofWindow = hashchain.getProofWindow()
        t.is(proofWindow.commitments.length, 8)
      })
    }
  }
  
  cleanupTestDir(testDir)
}) 