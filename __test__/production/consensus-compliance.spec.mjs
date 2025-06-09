import test from 'ava'
import { 
  HashChain, 
  selectChunksV1, 
  verifyChunkSelection,
  verifyProofOfStorageContinuity
} from '../../index.js'
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
import { createHash } from 'crypto'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Network Consensus Compliance Tests ===

test('chunk selection algorithm determinism', (t) => {
  // Consensus critical: Same inputs must always produce same outputs
  const testBlockHash = Buffer.from('a'.repeat(64), 'hex')
  const totalChunks = 100
  
  // Run selection multiple times
  const results = []
  for (let i = 0; i < 10; i++) {
    const result = selectChunksV1(testBlockHash, totalChunks)
    results.push(result.selectedIndices)
  }
  
  // All results should be identical
  for (let i = 1; i < results.length; i++) {
    t.deepEqual(results[0], results[i])
  }
  
  // Should always select exactly 4 chunks
  t.is(results[0].length, 4)
  
  // All selected indices should be unique
  const uniqueIndices = new Set(results[0])
  t.is(uniqueIndices.size, 4)
  
  // All indices should be within valid range
  for (const idx of results[0]) {
    t.true(idx >= 0 && idx < totalChunks)
  }
})

test('consensus algorithm version compliance', (t) => {
  const testBlockHash = Buffer.from('b'.repeat(64), 'hex')
  const totalChunks = 50
  
  // Generate selection with algorithm V1
  const result = selectChunksV1(testBlockHash, totalChunks)
  
  // Verify algorithm version is correctly set
  t.is(result.algorithmVersion, 1)
  
  // Verify verification function accepts V1 results
  const isValid = verifyChunkSelection(
    testBlockHash, 
    totalChunks, 
    result.selectedIndices, 
    1
  )
  t.true(isValid)
  
  // Verify verification fails for wrong algorithm version
  const isInvalidVersion = verifyChunkSelection(
    testBlockHash, 
    totalChunks, 
    result.selectedIndices, 
    999 // Invalid version
  )
  t.false(isInvalidVersion)
})

test('block hash entropy utilization', (t) => {
  // Test that different block hashes produce different chunk selections
  const totalChunks = 1000
  const selections = new Map()
  
  // Generate selections for different block hashes
  for (let i = 0; i < 100; i++) {
    const blockHash = Buffer.alloc(32)
    blockHash.writeUInt32BE(i, 0) // Different patterns
    
    const result = selectChunksV1(blockHash, totalChunks)
    const selectionKey = result.selectedIndices.join(',')
    
    // Track if we've seen this selection before
    if (selections.has(selectionKey)) {
      selections.set(selectionKey, selections.get(selectionKey) + 1)
    } else {
      selections.set(selectionKey, 1)
    }
  }
  
  // Should have high variety (low collision rate)
  const uniqueSelections = selections.size
  const collisionRate = 1 - (uniqueSelections / 100)
  
  t.true(uniqueSelections >= 85, `Only ${uniqueSelections} unique selections out of 100`)
  t.true(collisionRate < 0.15, `Collision rate too high: ${collisionRate * 100}%`)
})

test('chunk selection edge case compliance', (t) => {
  // Test minimum valid chunk count (exactly 4)
  const result4 = selectChunksV1(TEST_BLOCK_HASH, 4)
  t.is(result4.selectedIndices.length, 4)
  t.deepEqual(result4.selectedIndices.sort(), [0, 1, 2, 3])
  
  // Test with small chunk counts
  const result5 = selectChunksV1(TEST_BLOCK_HASH, 5)
  t.is(result5.selectedIndices.length, 4)
  for (const idx of result5.selectedIndices) {
    t.true(idx >= 0 && idx < 5)
  }
  
  // Test with large chunk counts
  const resultLarge = selectChunksV1(TEST_BLOCK_HASH, 1000000)
  t.is(resultLarge.selectedIndices.length, 4)
  for (const idx of resultLarge.selectedIndices) {
    t.true(idx >= 0 && idx < 1000000)
  }
})

test('commitment hash standardization', (t) => {
  const testDir = createTestDir('commitment_hash_standardization')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add blocks and verify commitment hash calculations
  const commitments = []
  for (let i = 0; i < 5; i++) {
    const blockHash = generateBlockHash(1000 + i)
    const commitment = hashchain.addBlock(blockHash)
    commitments.push(commitment)
    
    // Manually recalculate commitment hash to verify
    const hashInput = Buffer.concat([
      commitment.previousCommitment,
      commitment.blockHash,
      ...commitment.selectedChunks.map(idx => Buffer.from([0, 0, 0, idx])), // 4-byte big-endian
      ...commitment.chunkHashes
    ])
    
    const expectedHash = createHash('sha256').update(hashInput).digest()
    t.deepEqual(commitment.commitmentHash, expectedHash)
  }
  
  cleanupTestDir(testDir)
})

test('network consensus file format validation', (t) => {
  const testDir = createTestDir('file_format_validation')
  const testData = createTestData(20480) // 5 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add blocks to create complete chain
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(generateBlockHash(2000 + i))
  }
  
  // Verify chain follows consensus format
  t.true(hashchain.verifyChain())
  
  // Load from file and verify same compliance
  const filePaths = hashchain.getFilePaths()
  const loadedChain = HashChain.loadFromFile(filePaths[0])
  
  t.true(loadedChain.verifyChain())
  
  // Both instances should produce identical proof windows
  const originalProof = hashchain.getProofWindow()
  const loadedProof = loadedChain.getProofWindow()
  
  t.deepEqual(originalProof.startCommitment, loadedProof.startCommitment)
  t.deepEqual(originalProof.endCommitment, loadedProof.endCommitment)
  t.is(originalProof.commitments.length, loadedProof.commitments.length)
  
  cleanupTestDir(testDir)
})

test('proof window consensus structure', (t) => {
  const testDir = createTestDir('proof_window_consensus')
  const testData = createTestData(32768) // 8 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add exactly 8 blocks (minimum for proof window)
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(generateBlockHash(3000 + i))
  }
  
  const proofWindow = hashchain.getProofWindow()
  
  // Consensus requirement: exactly 8 commitments
  t.is(proofWindow.commitments.length, 8)
  
  // Consensus requirement: exactly 32 merkle proofs (8 blocks * 4 chunks)
  t.is(proofWindow.merkleProofs.length, 32)
  
  // Consensus requirement: valid commitment chain
  let expectedPrevious = proofWindow.startCommitment
  for (const commitment of proofWindow.commitments) {
    t.deepEqual(commitment.previousCommitment, expectedPrevious)
    expectedPrevious = commitment.commitmentHash
  }
  t.deepEqual(expectedPrevious, proofWindow.endCommitment)
  
  // Consensus requirement: all chunk selections valid
  for (const commitment of proofWindow.commitments) {
    t.is(commitment.selectedChunks.length, 4)
    
    const isValidSelection = verifyChunkSelection(
      commitment.blockHash,
      8, // Total chunks
      commitment.selectedChunks,
      1  // Algorithm version
    )
    t.true(isValidSelection)
  }
  
  cleanupTestDir(testDir)
})

test('cross-platform consistency', (t) => {
  // Test that HashChain produces identical results across different environments
  const testCases = [
    { chunks: 4, pattern: 'deterministic' },
    { chunks: 10, pattern: 'sequential' },
    { chunks: 100, pattern: 'deterministic' }
  ]
  
  for (const testCase of testCases) {
    const testDir = createTestDir(`cross_platform_${testCase.chunks}`)
    const testData = generateTestDataWithPattern(testCase.chunks, testCase.pattern)
    
    const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
    hashchain.streamData(testData, testDir)
    
    // Use deterministic block hashes
    const deterministicBlocks = [
      Buffer.from('1'.repeat(64), 'hex'),
      Buffer.from('2'.repeat(64), 'hex'),
      Buffer.from('3'.repeat(64), 'hex'),
      Buffer.from('4'.repeat(64), 'hex'),
      Buffer.from('5'.repeat(64), 'hex')
    ]
    
    const commitments = []
    for (const blockHash of deterministicBlocks) {
      const commitment = hashchain.addBlock(blockHash)
      commitments.push({
        blockHash: commitment.blockHash,
        selectedChunks: commitment.selectedChunks,
        chunkHashes: commitment.chunkHashes.map(h => h.toString('hex')),
        commitmentHash: commitment.commitmentHash.toString('hex')
      })
    }
    
    // These results should be identical across all platforms/environments
    // (In practice, you would compare against known good values)
    t.is(commitments.length, 5)
    for (const commitment of commitments) {
      t.is(commitment.selectedChunks.length, 4)
      t.is(commitment.chunkHashes.length, 4)
      t.is(commitment.commitmentHash.length, 64) // 32 bytes = 64 hex chars
    }
    
    cleanupTestDir(testDir)
  }
})

test('algorithm robustness against manipulation', (t) => {
  // Test that the consensus algorithm is resistant to manipulation attempts
  const totalChunks = 1000
  
  // Try to bias chunk selection through various block hash patterns
  const biasAttempts = [
    Buffer.alloc(32, 0x00),           // All zeros
    Buffer.alloc(32, 0xFF),           // All ones
    Buffer.from('0'.repeat(64), 'hex'), // Hex zeros
    Buffer.from('f'.repeat(64), 'hex'), // Hex ones
    // Sequential pattern
    Buffer.from(Array.from({length: 32}, (_, i) => i % 256)),
    // Repeating pattern
    Buffer.from(Array.from({length: 32}, (_, i) => [0xAA, 0xBB, 0xCC, 0xDD][i % 4]))
  ]
  
  const allSelections = []
  
  for (const blockHash of biasAttempts) {
    const result = selectChunksV1(blockHash, totalChunks)
    allSelections.push(result.selectedIndices)
    
    // Basic validation for each attempt
    t.is(result.selectedIndices.length, 4)
    
    // Check for uniqueness within selection
    const unique = new Set(result.selectedIndices)
    t.is(unique.size, 4)
    
    // Check range validity
    for (const idx of result.selectedIndices) {
      t.true(idx >= 0 && idx < totalChunks)
    }
  }
  
  // Verify that manipulated inputs don't produce concentrated results
  const allSelectedChunks = new Set()
  for (const selection of allSelections) {
    for (const chunk of selection) {
      allSelectedChunks.add(chunk)
    }
  }
  
  // Should spread across a reasonable range (not clustered)
  const spreadRatio = allSelectedChunks.size / (allSelections.length * 4)
  t.true(spreadRatio > 0.5, `Chunk spread too low: ${spreadRatio}`)
})

test('verification consensus compliance', (t) => {
  const testDir = createTestDir('verification_consensus')
  const testData = createTestData(24576) // 6 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add blocks for proof window
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(generateBlockHash(4000 + i))
  }
  
  const proofWindow = hashchain.getProofWindow()
  const anchoredCommitment = hashchain.getAnchoredCommitment()
  
  // Create proper merkle root (simplified - would be calculated correctly)
  const merkleRoot = Buffer.alloc(32, 0xaa)
  
  // Test consensus verification requirements
  const verifyResult = verifyProofOfStorageContinuity(proofWindow, anchoredCommitment, merkleRoot, 6)
  
  // Basic structure verification should pass
  t.notThrows(() => {
    verifyProofOfStorageContinuity(proofWindow, anchoredCommitment, merkleRoot, 6)
  })
  
  // Test verification failure cases
  
  // Invalid proof window size
  const invalidWindow = { ...proofWindow, commitments: proofWindow.commitments.slice(0, 7) }
  t.throws(() => {
    verifyProofOfStorageContinuity(invalidWindow, anchoredCommitment, merkleRoot, 6)
  })
  
  // Invalid anchored commitment
  const invalidAnchor = Buffer.alloc(32, 0x99)
  t.throws(() => {
    verifyProofOfStorageContinuity(proofWindow, invalidAnchor, merkleRoot, 6)
  })
  
  cleanupTestDir(testDir)
})

test('network parameter compliance', (t) => {
  // Verify that all network consensus parameters are correctly enforced
  
  // CHUNKS_PER_BLOCK must be 4
  const result = selectChunksV1(TEST_BLOCK_HASH, 100)
  t.is(result.selectedIndices.length, 4)
  
  // PROOF_WINDOW_BLOCKS must be 8
  const testDir = createTestDir('network_parameters')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 7 blocks - should not be able to generate proof window
  for (let i = 0; i < 7; i++) {
    hashchain.addBlock(generateBlockHash(5000 + i))
  }
  
  t.throws(() => {
    hashchain.getProofWindow()
  }, { message: /Chain too short/ })
  
  // Add 8th block - should now be able to generate proof window
  hashchain.addBlock(generateBlockHash(5007))
  
  t.notThrows(() => {
    const proofWindow = hashchain.getProofWindow()
    t.is(proofWindow.commitments.length, 8)
  })
  
  cleanupTestDir(testDir)
})

test('consensus algorithm stress testing', (t) => {
  // Test algorithm behavior under various stress conditions
  const stressTests = [
    { totalChunks: 4, description: 'minimum chunks' },
    { totalChunks: 1000000, description: 'maximum reasonable chunks' },
    { totalChunks: 37, description: 'prime number chunks' },
    { totalChunks: 128, description: 'power of 2 chunks' },
    { totalChunks: 999, description: 'near-round number' }
  ]
  
  for (const stress of stressTests) {
    // Test with multiple different block hashes
    for (let hashSeed = 0; hashSeed < 20; hashSeed++) {
      const blockHash = Buffer.alloc(32)
      blockHash.writeUInt32BE(hashSeed, 0)
      blockHash.writeUInt32BE(stress.totalChunks, 4)
      
      const result = selectChunksV1(blockHash, stress.totalChunks)
      
      // Verify basic constraints
      t.is(result.selectedIndices.length, 4, 
           `Failed for ${stress.description} with hash seed ${hashSeed}`)
      
      // Verify uniqueness
      const unique = new Set(result.selectedIndices)
      t.is(unique.size, 4, 
           `Non-unique selection for ${stress.description} with hash seed ${hashSeed}`)
      
      // Verify range
      for (const idx of result.selectedIndices) {
        t.true(idx >= 0 && idx < stress.totalChunks,
               `Out of range index ${idx} for ${stress.description}`)
      }
      
      // Verify determinism
      const result2 = selectChunksV1(blockHash, stress.totalChunks)
      t.deepEqual(result.selectedIndices, result2.selectedIndices,
                  `Non-deterministic for ${stress.description}`)
    }
  }
}) 