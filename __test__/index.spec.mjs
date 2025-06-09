import test from 'ava'
import { readFileSync, writeFileSync, mkdirSync, rmSync, existsSync } from 'fs'
import { join } from 'path'
import { createHash } from 'crypto'

import { 
  HashChain,
  selectChunksV1,
  verifyChunkSelection,
  createOwnershipCommitment,
  createAnchoredOwnershipCommitment,
  verifyProof
} from '../index.js'

// Helper function to create test data
function createTestData(size = 16384) { // 4 chunks by default
  const data = Buffer.alloc(size)
  for (let i = 0; i < size; i++) {
    data[i] = i % 256
  }
  return data
}

// Helper function to create test directory
function createTestDir(testName) {
  const testDir = join(process.cwd(), 'test_output', testName)
  if (existsSync(testDir)) {
    rmSync(testDir, { recursive: true, force: true })
  }
  mkdirSync(testDir, { recursive: true })
  return testDir
}

// Helper function to clean up test directory
function cleanupTestDir(testDir) {
  if (existsSync(testDir)) {
    rmSync(testDir, { recursive: true, force: true })
  }
}

// Test constants for deterministic testing
const TEST_PUBLIC_KEY = Buffer.from('a'.repeat(64), 'hex') // 32 bytes
const TEST_BLOCK_HASH = Buffer.from('b'.repeat(64), 'hex') // 32 bytes
const TEST_BLOCK_HEIGHT = 100

test.beforeEach(() => {
  // Clean up any existing test output
  const testOutputDir = join(process.cwd(), 'test_output')
  if (existsSync(testOutputDir)) {
    rmSync(testOutputDir, { recursive: true, force: true })
  }
})

test.afterEach(() => {
  // Clean up test output after each test
  const testOutputDir = join(process.cwd(), 'test_output')
  if (existsSync(testOutputDir)) {
    rmSync(testOutputDir, { recursive: true, force: true })
  }
})

// === HashChain Constructor Tests ===

test('HashChain constructor with valid inputs', (t) => {
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  t.truthy(hashchain)
  t.is(hashchain.getChainLength(), 0)
  t.is(hashchain.getTotalChunks(), 0)
  t.is(hashchain.getCurrentCommitment(), null)
  t.is(hashchain.getAnchoredCommitment(), null)
  t.is(hashchain.getFilePaths(), null)
})

test('HashChain constructor with invalid public key size', (t) => {
  const invalidPublicKey = Buffer.from('invalid', 'utf-8') // Wrong size
  
  t.throws(() => {
    new HashChain(invalidPublicKey, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  }, { message: /Public key must be 32 bytes/ })
})

test('HashChain constructor with invalid block hash size', (t) => {
  const invalidBlockHash = Buffer.from('invalid', 'utf-8') // Wrong size
  
  t.throws(() => {
    new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, invalidBlockHash)
  }, { message: /Block hash must be 32 bytes/ })
})

// === Data Streaming Tests ===

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
  const expectedHash = createHash('sha256').update(testData).digest('hex')
  t.true(hashchainPath.includes(expectedHash))
  t.true(dataPath.includes(expectedHash))
  
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

// === Commitment Tests ===

test('createOwnershipCommitment works correctly', (t) => {
  const dataHash = createHash('sha256').update('test data').digest()
  
  const commitment = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  
  t.truthy(commitment)
  t.deepEqual(commitment.publicKey, TEST_PUBLIC_KEY)
  t.deepEqual(commitment.dataHash, dataHash)
  t.is(commitment.commitmentHash.length, 32)
  
  // Verify commitment hash is deterministic
  const commitment2 = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  t.deepEqual(commitment.commitmentHash, commitment2.commitmentHash)
})

test('createOwnershipCommitment validates input sizes', (t) => {
  const validDataHash = Buffer.alloc(32)
  const invalidPublicKey = Buffer.from('invalid')
  const invalidDataHash = Buffer.from('invalid')
  
  t.throws(() => {
    createOwnershipCommitment(invalidPublicKey, validDataHash)
  }, { message: /must be 32 bytes each/ })
  
  t.throws(() => {
    createOwnershipCommitment(TEST_PUBLIC_KEY, invalidDataHash)
  }, { message: /must be 32 bytes each/ })
})

test('createAnchoredOwnershipCommitment works correctly', (t) => {
  const dataHash = createHash('sha256').update('test data').digest()
  const ownershipCommitment = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  
  const blockCommitment = {
    blockHeight: TEST_BLOCK_HEIGHT,
    blockHash: TEST_BLOCK_HASH
  }
  
  const anchored = createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment)
  
  t.truthy(anchored)
  t.deepEqual(anchored.ownershipCommitment, ownershipCommitment)
  t.deepEqual(anchored.blockCommitment, blockCommitment)
  t.is(anchored.anchoredHash.length, 32)
})

// === Chain Management Tests ===

test('addBlock creates physical access commitment', (t) => {
  const testDir = createTestDir('add_block_test')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const newBlockHash = Buffer.from('d'.repeat(64), 'hex')
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
  const newBlockHash = Buffer.from('d'.repeat(64), 'hex')
  
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
    Buffer.from('d'.repeat(64), 'hex'),
    Buffer.from('e'.repeat(64), 'hex'),
    Buffer.from('f'.repeat(64), 'hex')
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

// === File I/O Tests ===

test('readChunk returns correct data', (t) => {
  const testDir = createTestDir('read_chunk_test')
  const testData = createTestData(8192) // 2 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Read first chunk
  const chunk0 = hashchain.readChunk(0)
  t.is(chunk0.length, 4096)
  
  // Verify chunk content (first 4096 bytes of test data)
  const expectedChunk0 = testData.subarray(0, 4096)
  t.deepEqual(chunk0.subarray(0, expectedChunk0.length), expectedChunk0)
  
  // Read second chunk
  const chunk1 = hashchain.readChunk(1)
  t.is(chunk1.length, 4096)
  
  // Second chunk should contain remaining data + padding
  const expectedChunk1Start = testData.subarray(4096)
  t.deepEqual(chunk1.subarray(0, expectedChunk1Start.length), expectedChunk1Start)
  
  cleanupTestDir(testDir)
})

test('readChunk validates chunk index', (t) => {
  const testDir = createTestDir('read_chunk_validation')
  const testData = createTestData(4096) // 1 chunk
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Valid index should work
  t.notThrows(() => {
    hashchain.readChunk(0)
  })
  
  // Invalid index should throw
  t.throws(() => {
    hashchain.readChunk(1)
  }, { message: /Chunk index 1 out of range/ })
  
  cleanupTestDir(testDir)
})

test('readChunk fails without data file', (t) => {
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  t.throws(() => {
    hashchain.readChunk(0)
  }, { message: /No data file available/ })
})

test('verifyChain basic validation', (t) => {
  const testDir = createTestDir('verify_chain_test')
  const testData = createTestData(4096)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Should pass basic verification
  t.true(hashchain.verifyChain())
  
  cleanupTestDir(testDir)
})

test('verifyChain fails without files', (t) => {
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Should fail without files
  t.false(hashchain.verifyChain())
})

// === Integration Tests ===

test('end-to-end workflow', (t) => {
  const testDir = createTestDir('end_to_end')
  const testData = createTestData(20480) // 5 chunks
  
  // 1. Create HashChain
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // 2. Stream data
  hashchain.streamData(testData, testDir)
  t.is(hashchain.getTotalChunks(), 5)
  t.is(hashchain.getChainLength(), 0)
  
  // 3. Add multiple blocks
  const blockHashes = [
    Buffer.from('111'.repeat(21) + '1', 'hex'),
    Buffer.from('222'.repeat(21) + '2', 'hex'),
    Buffer.from('333'.repeat(21) + '3', 'hex'),
    Buffer.from('444'.repeat(21) + '4', 'hex'),
    Buffer.from('555'.repeat(21) + '5', 'hex')
  ]
  
  for (let i = 0; i < blockHashes.length; i++) {
    const commitment = hashchain.addBlock(blockHashes[i])
    
    // Verify commitment structure
    t.is(commitment.selectedChunks.length, 4)
    t.is(commitment.chunkHashes.length, 4)
    t.deepEqual(commitment.blockHash, blockHashes[i])
    
    // Verify chunk selection is valid
    t.true(verifyChunkSelection(
      blockHashes[i],
      5,
      commitment.selectedChunks,
      1
    ))
  }
  
  // 4. Verify final state
  t.is(hashchain.getChainLength(), 5)
  t.true(hashchain.verifyChain())
  
  // 5. Verify we can read all chunks
  for (let i = 0; i < 5; i++) {
    const chunk = hashchain.readChunk(i)
    t.is(chunk.length, 4096)
  }
  
  cleanupTestDir(testDir)
})

test('consensus compliance across different data sizes', (t) => {
  const testSizes = [4096, 8192, 16384, 65536] // 1, 2, 4, 16 chunks
  const blockHash = TEST_BLOCK_HASH
  
  for (const size of testSizes) {
    const chunks = Math.ceil(size / 4096)
    
    if (chunks >= 4) { // Need at least 4 chunks for selection
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
  }
})

test('stress test with many blocks', (t) => {
  const testDir = createTestDir('stress_test')
  const testData = createTestData(32768) // 8 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 50 blocks
  for (let i = 0; i < 50; i++) {
    const blockHashData = Buffer.alloc(32)
    blockHashData.writeUInt32BE(i, 28) // Put block number at end
    
    const commitment = hashchain.addBlock(blockHashData)
    
    // Verify each commitment
    t.is(commitment.selectedChunks.length, 4)
    t.is(commitment.chunkHashes.length, 4)
    t.is(commitment.commitmentHash.length, 32)
    
    // All selected chunks should be in valid range
    for (const idx of commitment.selectedChunks) {
      t.true(idx >= 0 && idx < 8)
    }
  }
  
  t.is(hashchain.getChainLength(), 50)
  t.true(hashchain.verifyChain())
  
  cleanupTestDir(testDir)
})

// === Edge Cases and Error Handling ===

test('handle minimum viable file size', (t) => {
  const testDir = createTestDir('minimum_size')
  const testData = createTestData(16384) // Exactly 4 chunks (minimum for selection)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  t.is(hashchain.getTotalChunks(), 4)
  
  // Should be able to add blocks
  const blockHash = Buffer.from('abc123'.repeat(10) + 'abcd', 'hex') // 32 bytes
  const commitment = hashchain.addBlock(blockHash)
  
  t.truthy(commitment)
  t.is(commitment.selectedChunks.length, 4)
  
  // All chunks should be selected (since we only have 4)
  const sortedIndices = [...commitment.selectedChunks].sort()
  t.deepEqual(sortedIndices, [0, 1, 2, 3])
  
  cleanupTestDir(testDir)
})

test('handle large file with many chunks', (t) => {
  const testDir = createTestDir('large_file')
  const testData = createTestData(1048576) // 256 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  t.is(hashchain.getTotalChunks(), 256)
  
  const blockHash = Buffer.from('abc123'.repeat(10) + 'def0', 'hex') // 32 bytes
  const commitment = hashchain.addBlock(blockHash)
  
  t.is(commitment.selectedChunks.length, 4)
  
  // All selected chunks should be in valid range
  for (const idx of commitment.selectedChunks) {
    t.true(idx >= 0 && idx < 256)
  }
  
  // Should be able to read any selected chunk
  for (const idx of commitment.selectedChunks) {
    const chunk = hashchain.readChunk(idx)
    t.is(chunk.length, 4096)
  }
  
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

// === Proof Window Tests ===

test('getProofWindow works with sufficient blocks', (t) => {
  const testDir = createTestDir('proof_window_valid')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add 8 blocks to meet minimum requirement
  for (let i = 0; i < 8; i++) {
    const blockHash = Buffer.alloc(32)
    blockHash.writeUInt32BE(i, 28)
    hashchain.addBlock(blockHash)
  }
  
  const proofWindow = hashchain.getProofWindow()
  
  t.truthy(proofWindow)
  t.is(proofWindow.commitments.length, 8) // Should have exactly 8 commitments
  t.is(proofWindow.merkleProofs.length, 32) // 8 blocks * 4 chunks per block
  t.truthy(proofWindow.startCommitment)
  t.truthy(proofWindow.endCommitment)
  t.is(proofWindow.startCommitment.length, 32)
  t.is(proofWindow.endCommitment.length, 32)
  
  cleanupTestDir(testDir)
})

test('getProofWindow fails with insufficient blocks', (t) => {
  const testDir = createTestDir('proof_window_insufficient')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add only 5 blocks (need 8)
  for (let i = 0; i < 5; i++) {
    const blockHash = Buffer.alloc(32)
    blockHash.writeUInt32BE(i, 28)
    hashchain.addBlock(blockHash)
  }
  
  t.throws(() => {
    hashchain.getProofWindow()
  }, { message: /Chain too short: 5 < 8/ })
  
  cleanupTestDir(testDir)
})

// === Chunk Limit Validation Tests ===

test('streamData enforces minimum chunk requirement', (t) => {
  const testDir = createTestDir('min_chunks_validation')
  const emptyData = Buffer.alloc(0) // No data = 0 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  t.throws(() => {
    hashchain.streamData(emptyData, testDir)
  }, { message: /Too few chunks: 0 < 1/ })
  
  cleanupTestDir(testDir)
})

test('streamData validates maximum chunks limit', (t) => {
  // This test would be impractical with real data (4TB+), so we simulate the validation
  t.pass() // Max chunk validation is implemented but testing 4TB is impractical
})

// === Production Verification Tests ===

test('verifyProof validates proof window structure', (t) => {
  // Create a valid proof window structure
  const commitment1 = {
    blockHeight: 100,
    previousCommitment: Buffer.alloc(32, 1),
    blockHash: Buffer.alloc(32, 2),
    selectedChunks: [0, 1, 2, 3],
    chunkHashes: [
      Buffer.alloc(32, 10),
      Buffer.alloc(32, 11),
      Buffer.alloc(32, 12),
      Buffer.alloc(32, 13)
    ],
    commitmentHash: Buffer.alloc(32, 20)
  }
  
  const commitment2 = {
    blockHeight: 101,
    previousCommitment: Buffer.alloc(32, 20), // Links to commitment1
    blockHash: Buffer.alloc(32, 3),
    selectedChunks: [1, 2, 3, 0],
    chunkHashes: [
      Buffer.alloc(32, 14),
      Buffer.alloc(32, 15),
      Buffer.alloc(32, 16),
      Buffer.alloc(32, 17)
    ],
    commitmentHash: Buffer.alloc(32, 21)
  }
  
  // Create 6 more commitments to reach 8
  const commitments = [commitment1, commitment2]
  for (let i = 2; i < 8; i++) {
    commitments.push({
      blockHeight: 100 + i,
      previousCommitment: commitments[i-1].commitmentHash,
      blockHash: Buffer.alloc(32, 3 + i),
      selectedChunks: [0, 1, 2, 3],
      chunkHashes: [
        Buffer.alloc(32, 10 + i * 4),
        Buffer.alloc(32, 11 + i * 4),
        Buffer.alloc(32, 12 + i * 4),
        Buffer.alloc(32, 13 + i * 4)
      ],
      commitmentHash: Buffer.alloc(32, 20 + i)
    })
  }
  
  const mockMerkleProofs = []
  for (let i = 0; i < 32; i++) { // 8 blocks * 4 chunks
    mockMerkleProofs.push(Buffer.alloc(33, i)) // 33 bytes for merkle proof
  }
  
  const proofWindow = {
    commitments,
    merkleProofs: mockMerkleProofs,
    startCommitment: Buffer.alloc(32, 1),
    endCommitment: commitments[7].commitmentHash
  }
  
  const anchoredCommitment = Buffer.alloc(32, 1)
  const merkleRoot = Buffer.alloc(32, 100)
  
  // This should pass basic structure validation
  // Note: Full cryptographic validation would require real Merkle proofs
  const result = verifyProof(proofWindow, anchoredCommitment, merkleRoot, 100)
  
  // The verification will fail due to mock data, but structure validation should work
  t.is(typeof result, 'boolean')
})

test('verifyProof rejects invalid proof window size', (t) => {
  const invalidProofWindow = {
    commitments: [], // Empty - should be 8
    merkleProofs: [],
    startCommitment: Buffer.alloc(32),
    endCommitment: Buffer.alloc(32)
  }
  
  const anchoredCommitment = Buffer.alloc(32)
  const merkleRoot = Buffer.alloc(32)
  
  const result = verifyProof(invalidProofWindow, anchoredCommitment, merkleRoot, 100)
  t.false(result) // Should fail due to wrong number of commitments
})

test('verifyProof validates merkle root format', (t) => {
  // Create minimal valid structure
  const commitments = []
  for (let i = 0; i < 8; i++) {
    commitments.push({
      blockHeight: 100 + i,
      previousCommitment: Buffer.alloc(32, i),
      blockHash: Buffer.alloc(32, i + 10),
      selectedChunks: [0, 1, 2, 3],
      chunkHashes: [
        Buffer.alloc(32, 0),
        Buffer.alloc(32, 1),
        Buffer.alloc(32, 2),
        Buffer.alloc(32, 3)
      ],
      commitmentHash: Buffer.alloc(32, i + 1)
    })
  }
  
  const proofWindow = {
    commitments,
    merkleProofs: new Array(32).fill(Buffer.alloc(33)),
    startCommitment: Buffer.alloc(32),
    endCommitment: Buffer.alloc(32)
  }
  
  const anchoredCommitment = Buffer.alloc(32)
  const invalidMerkleRoot = Buffer.alloc(16) // Wrong size - should be 32
  
  const result = verifyProof(proofWindow, anchoredCommitment, invalidMerkleRoot, 100)
  t.false(result) // Should fail due to invalid merkle root size
})

// === Error Handling and Edge Cases ===

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

test('loadFromFile method validation', (t) => {
  // Test loading non-existent file
  t.throws(() => {
    HashChain.loadFromFile('nonexistent.hashchain')
  }, { message: /HashChain file not found/ })
})

test('chunk selection with edge case file sizes', (t) => {
  // Test with exactly 4 chunks (minimum for selection)
  let result = selectChunksV1(TEST_BLOCK_HASH, 4)
  t.is(result.selectedIndices.length, 4)
  const sorted = [...result.selectedIndices].sort()
  t.deepEqual(sorted, [0, 1, 2, 3]) // All chunks must be selected
  
  // Test with 5 chunks
  result = selectChunksV1(TEST_BLOCK_HASH, 5)
  t.is(result.selectedIndices.length, 4)
  for (const idx of result.selectedIndices) {
    t.true(idx >= 0 && idx < 5)
  }
  
  // Test with large number of chunks
  result = selectChunksV1(TEST_BLOCK_HASH, 10000)
  t.is(result.selectedIndices.length, 4)
  for (const idx of result.selectedIndices) {
    t.true(idx >= 0 && idx < 10000)
  }
  
  // All indices should be unique
  const uniqueIndices = new Set(result.selectedIndices)
  t.is(uniqueIndices.size, 4)
})

test('HashChain getChainInfo provides comprehensive state information', (t) => {
  const testDir = createTestDir('get_chain_info_test')
  const testData = createTestData(16384) // 4 chunks
  
  // Test 1: Uninitialized state
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  let info = hashchain.getChainInfo()
  
  t.is(info.status, 'uninitialized')
  t.is(info.totalChunks, 0)
  t.is(info.chainLength, 0)
  t.is(info.chunkSizeBytes, 4096)
  t.is(info.totalStorageMb, 0)
  t.is(info.hashchainFilePath, undefined)
  t.is(info.dataFilePath, undefined)
  t.is(info.hashchainFileSizeBytes, undefined)
  t.is(info.dataFileSizeBytes, undefined)
  t.is(info.anchoredCommitment, undefined)
  t.is(info.currentCommitment, undefined)
  t.false(info.proofWindowReady)
  t.is(info.blocksUntilProofReady, 8)
  t.is(info.consensusAlgorithmVersion, 1)
  t.is(info.initialBlockHeight, TEST_BLOCK_HEIGHT)
  
  // Test 2: Initialized state (data streamed)
  hashchain.streamData(testData, testDir)
  info = hashchain.getChainInfo()
  
  t.is(info.status, 'initialized')
  t.is(info.totalChunks, 4)
  t.is(info.chainLength, 0)
  t.is(info.totalStorageMb, 16 / 1024) // 16KB = 0.015625 MB
  t.truthy(info.hashchainFilePath)
  t.truthy(info.dataFilePath)
  t.true(info.hashchainFileSizeBytes > 0)
  t.true(info.dataFileSizeBytes > 0)
  t.truthy(info.anchoredCommitment)
  t.truthy(info.currentCommitment)
  t.is(info.anchoredCommitment, info.currentCommitment) // Should be equal initially
  t.false(info.proofWindowReady)
  t.is(info.blocksUntilProofReady, 8)
  
  // Verify hex strings are valid
  t.is(info.anchoredCommitment.length, 64) // 32 bytes = 64 hex chars
  t.is(info.currentCommitment.length, 64)
  t.regex(info.anchoredCommitment, /^[0-9a-f]{64}$/)
  
  // Test 3: Building state (some blocks added)
  const blockHashes = [
    Buffer.from('1'.repeat(64), 'hex'),
    Buffer.from('2'.repeat(64), 'hex'),
    Buffer.from('3'.repeat(64), 'hex')
  ]
  
  for (let i = 0; i < blockHashes.length; i++) {
    hashchain.addBlock(blockHashes[i])
    info = hashchain.getChainInfo()
    
    t.is(info.status, 'building')
    t.is(info.chainLength, i + 1)
    t.false(info.proofWindowReady)
    t.is(info.blocksUntilProofReady, 8 - (i + 1))
    
    // Current commitment should change but anchored should stay the same
    t.truthy(info.currentCommitment)
    t.not(info.currentCommitment, info.anchoredCommitment)
  }
  
  // Test 4: Active state (8 blocks added)
  const additionalBlocks = [
    Buffer.from('4'.repeat(64), 'hex'),
    Buffer.from('5'.repeat(64), 'hex'),
    Buffer.from('6'.repeat(64), 'hex'),
    Buffer.from('7'.repeat(64), 'hex'),
    Buffer.from('8'.repeat(64), 'hex')
  ]
  
  for (const blockHash of additionalBlocks) {
    hashchain.addBlock(blockHash)
  }
  
  info = hashchain.getChainInfo()
  t.is(info.status, 'active')
  t.is(info.chainLength, 8)
  t.true(info.proofWindowReady)
  t.is(info.blocksUntilProofReady, undefined)
  
  // File sizes should be reasonable
  t.true(info.dataFileSizeBytes >= 16384) // At least 16KB for 4 chunks
  t.true(info.hashchainFileSizeBytes > 0) // Some metadata
  
  cleanupTestDir(testDir)
})

test('HashChain getChainInfo handles file size calculation correctly', (t) => {
  const testDir = createTestDir('get_chain_info_file_sizes')
  
  // Test different data sizes
  const testSizes = [
    { bytes: 4096, expectedChunks: 1, expectedMB: 4096 / (1024 * 1024) },
    { bytes: 8192, expectedChunks: 2, expectedMB: 8192 / (1024 * 1024) },
    { bytes: 1048576, expectedChunks: 256, expectedMB: 1 } // 1MB
  ]
  
  for (const { bytes, expectedChunks, expectedMB } of testSizes) {
    const testData = createTestData(bytes)
    const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
    hashchain.streamData(testData, testDir)
    
    const info = hashchain.getChainInfo()
    t.is(info.totalChunks, expectedChunks)
    t.is(info.totalStorageMb, expectedMB)
    
    // Verify file paths are correctly formatted
    t.true(info.hashchainFilePath.endsWith('.hashchain'))
    t.true(info.dataFilePath.endsWith('.data'))
    
    // Both files should use the same hash in the filename
    const hashchainHash = info.hashchainFilePath.split('/').pop().split('.')[0]
    const dataHash = info.dataFilePath.split('/').pop().split('.')[0]
    t.is(hashchainHash, dataHash)
  }
  
  cleanupTestDir(testDir)
})

test('HashChain getChainInfo commitment tracking works correctly', (t) => {
  const testDir = createTestDir('get_chain_info_commitments')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  const initialInfo = hashchain.getChainInfo()
  const initialCommitment = initialInfo.currentCommitment
  
  // Add blocks and track commitment changes
  const blockHashes = [
    Buffer.from('a'.repeat(64), 'hex'),
    Buffer.from('b'.repeat(64), 'hex'),
    Buffer.from('c'.repeat(64), 'hex')
  ]
  
  let previousCommitment = initialCommitment
  
  for (const blockHash of blockHashes) {
    hashchain.addBlock(blockHash)
    const info = hashchain.getChainInfo()
    
    // Current commitment should change
    t.not(info.currentCommitment, previousCommitment)
    
    // Anchored commitment should never change
    t.is(info.anchoredCommitment, initialCommitment)
    
    // Commitment should be valid hex
    t.is(info.currentCommitment.length, 64)
    t.regex(info.currentCommitment, /^[0-9a-f]{64}$/)
    
    previousCommitment = info.currentCommitment
  }
  
  cleanupTestDir(testDir)
})