import test from 'ava'
import { 
  HashChain, 
  selectChunksV1, 
  verifyChunkSelection,
  createOwnershipCommitment,
  createAnchoredOwnershipCommitment,
  verifyProof
} from '../../index.js'
import { 
  TEST_PUBLIC_KEY, 
  TEST_BLOCK_HASH, 
  TEST_BLOCK_HEIGHT,
  createTestData,
  createTestDir,
  cleanupTestDir,
  setupTest,
  teardownTest
} from '../helpers/test-setup.mjs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Input Validation Unit Tests ===

test('HashChain constructor validates public key length', (t) => {
  const validBlockHash = Buffer.alloc(32, 1)
  
  // Invalid public key lengths
  const invalidKeys = [
    Buffer.alloc(0),      // Empty
    Buffer.alloc(16),     // Too short
    Buffer.alloc(31),     // One byte short
    Buffer.alloc(33),     // One byte too long
    Buffer.alloc(64),     // Way too long
  ]
  
  for (const invalidKey of invalidKeys) {
    t.throws(() => {
      new HashChain(invalidKey, TEST_BLOCK_HEIGHT, validBlockHash)
    }, { message: /Public key must be 32 bytes/ })
  }
  
  // Valid 32-byte key should work
  t.notThrows(() => {
    new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, validBlockHash)
  })
})

test('HashChain constructor validates block hash length', (t) => {
  const validPublicKey = Buffer.alloc(32, 1)
  
  // Invalid block hash lengths
  const invalidHashes = [
    Buffer.alloc(0),      // Empty
    Buffer.alloc(16),     // Too short
    Buffer.alloc(31),     // One byte short
    Buffer.alloc(33),     // One byte too long
    Buffer.alloc(64),     // Way too long
  ]
  
  for (const invalidHash of invalidHashes) {
    t.throws(() => {
      new HashChain(validPublicKey, TEST_BLOCK_HEIGHT, invalidHash)
    }, { message: /Block hash must be 32 bytes/ })
  }
  
  // Valid 32-byte hash should work
  t.notThrows(() => {
    new HashChain(validPublicKey, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  })
})

test('HashChain constructor handles various block heights', (t) => {
  // Valid block heights
  const validHeights = [0, 1, 100, 999999, Number.MAX_SAFE_INTEGER]
  
  for (const height of validHeights) {
    t.notThrows(() => {
      new HashChain(TEST_PUBLIC_KEY, height, TEST_BLOCK_HASH)
    })
  }
})

test('addBlock validates block hash length', (t) => {
  const testDir = createTestDir('add_block_validation')
  const testData = createTestData(16384)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Invalid block hash lengths should fail
  const invalidHashes = [
    Buffer.alloc(0),
    Buffer.alloc(16),
    Buffer.alloc(31),
    Buffer.alloc(33),
  ]
  
  for (const invalidHash of invalidHashes) {
    t.throws(() => {
      hashchain.addBlock(invalidHash)
    }, { message: /Block hash must be 32 bytes/ })
  }
  
  // Valid hash should work
  const validHash = Buffer.alloc(32, 0xff)
  t.notThrows(() => {
    hashchain.addBlock(validHash)
  })
  
  cleanupTestDir(testDir)
})

test('selectChunksV1 validates parameters', (t) => {
  // Invalid block hash lengths
  const invalidHashes = [Buffer.alloc(0), Buffer.alloc(16), Buffer.alloc(31)]
  
  for (const invalidHash of invalidHashes) {
    t.throws(() => {
      selectChunksV1(invalidHash, 100)
    }, { message: /must be exactly 32 bytes/ })
  }
  
  // Invalid chunk counts
  t.throws(() => {
    selectChunksV1(TEST_BLOCK_HASH, 0)
  }, { message: /Total chunks must be positive/ })
  
  t.throws(() => {
    selectChunksV1(TEST_BLOCK_HASH, 3) // Less than 4 chunks required
  }, { message: /must be >= CHUNKS_PER_BLOCK/ })
  
  // Valid parameters should work
  t.notThrows(() => {
    selectChunksV1(TEST_BLOCK_HASH, 100)
  })
})

test('verifyChunkSelection validates parameters', (t) => {
  // Invalid block hash
  t.throws(() => {
    verifyChunkSelection(Buffer.alloc(16), 100, [0, 1, 2, 3])
  })
  
  // Invalid total chunks
  t.throws(() => {
    verifyChunkSelection(TEST_BLOCK_HASH, 0, [0, 1, 2, 3])
  })
  
  // Valid parameters should work
  t.notThrows(() => {
    verifyChunkSelection(TEST_BLOCK_HASH, 100, [0, 1, 2, 3])
  })
})

test('createOwnershipCommitment validates input lengths', (t) => {
  // Invalid public key lengths
  const invalidKeys = [Buffer.alloc(16), Buffer.alloc(33)]
  for (const key of invalidKeys) {
    t.throws(() => {
      createOwnershipCommitment(key, Buffer.alloc(32))
    }, { message: /must be 32 bytes each/ })
  }
  
  // Invalid data hash lengths
  const invalidHashes = [Buffer.alloc(16), Buffer.alloc(33)]
  for (const hash of invalidHashes) {
    t.throws(() => {
      createOwnershipCommitment(Buffer.alloc(32), hash)
    }, { message: /must be 32 bytes each/ })
  }
  
  // Valid inputs should work
  t.notThrows(() => {
    createOwnershipCommitment(Buffer.alloc(32), Buffer.alloc(32))
  })
})

test('streamData validates directory creation', (t) => {
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const testData = createTestData(4096)
  
  // Should handle non-existent directory
  const testDir = createTestDir('nonexistent_parent/child')
  
  t.notThrows(() => {
    hashchain.streamData(testData, testDir)
  })
  
  cleanupTestDir(testDir)
})

test('streamData prevents duplicate streaming', (t) => {
  const testDir = createTestDir('duplicate_streaming')
  const testData = createTestData(4096)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // First streaming should work
  hashchain.streamData(testData, testDir)
  
  // Second streaming should fail
  t.throws(() => {
    hashchain.streamData(testData, testDir)
  }, { message: /HashChain already has data/ })
  
  cleanupTestDir(testDir)
})

test('streamData enforces chunk count limits', (t) => {
  const testDir = createTestDir('chunk_limits')
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Empty data should fail
  const emptyData = Buffer.alloc(0)
  t.throws(() => {
    hashchain.streamData(emptyData, testDir)
  }, { message: /Too few chunks: 0 < 1/ })
  
  // Minimum data should work (1 byte = 1 chunk)
  const minData = Buffer.alloc(1)
  t.notThrows(() => {
    hashchain.streamData(minData, testDir)
  })
  
  cleanupTestDir(testDir)
})

test('readChunk validates chunk index bounds', (t) => {
  const testDir = createTestDir('chunk_index_validation')
  const testData = createTestData(8192) // 2 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Valid indices should work
  t.notThrows(() => {
    hashchain.readChunk(0)
    hashchain.readChunk(1)
  })
  
  // Invalid indices should fail
  t.throws(() => {
    hashchain.readChunk(2) // Beyond range
  }, { message: /out of range/ })
  
  t.throws(() => {
    hashchain.readChunk(-1) // Negative
  }, { message: /out of range/ })
  
  t.throws(() => {
    hashchain.readChunk(999) // Way beyond
  }, { message: /out of range/ })
  
  cleanupTestDir(testDir)
})

test('operations fail on uninitialized hashchain', (t) => {
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  
  // Operations requiring data should fail
  t.throws(() => {
    hashchain.addBlock(TEST_BLOCK_HASH)
  }, { message: /No data has been streamed/ })
  
  t.throws(() => {
    hashchain.readChunk(0)
  }, { message: /No data file available/ })
  
  t.throws(() => {
    hashchain.getProofWindow()
  })
  
  // Info operations should work
  t.notThrows(() => {
    hashchain.getChainLength()
    hashchain.getTotalChunks()
    hashchain.getChainInfo()
  })
})

test('getProofWindow validates chain length', (t) => {
  const testDir = createTestDir('proof_window_validation')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Should fail with insufficient blocks
  for (let i = 0; i < 7; i++) {
    t.throws(() => {
      hashchain.getProofWindow()
    }, { message: /Chain too short/ })
    
    hashchain.addBlock(Buffer.from((i + 1).toString(16).repeat(64), 'hex'))
  }
  
  // Should work with 8+ blocks
  hashchain.addBlock(Buffer.from('8'.repeat(64), 'hex'))
  t.notThrows(() => {
    hashchain.getProofWindow()
  })
  
  cleanupTestDir(testDir)
})

test('verifyProof validates input parameters', (t) => {
  // Create valid test data
  const testDir = createTestDir('verify_proof_validation')
  const testData = createTestData(16384) // 4 chunks
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add enough blocks for proof window
  for (let i = 0; i < 8; i++) {
    hashchain.addBlock(Buffer.from((i + 1).toString(16).repeat(64), 'hex'))
  }
  
  const proofWindow = hashchain.getProofWindow()
  const validAnchoredCommitment = hashchain.getAnchoredCommitment()
  const validMerkleRoot = Buffer.alloc(32, 0xaa)
  
  // Valid proof should work
  t.notThrows(() => {
    verifyProof(proofWindow, validAnchoredCommitment, validMerkleRoot, 4)
  })
  
  // Invalid anchored commitment length
  t.throws(() => {
    verifyProof(proofWindow, Buffer.alloc(16), validMerkleRoot, 4)
  })
  
  // Invalid merkle root length
  t.throws(() => {
    verifyProof(proofWindow, validAnchoredCommitment, Buffer.alloc(16), 4)
  })
  
  // Invalid total chunks
  t.throws(() => {
    verifyProof(proofWindow, validAnchoredCommitment, validMerkleRoot, 0)
  })
  
  cleanupTestDir(testDir)
})

test('type safety with edge case inputs', (t) => {
  // Test with maximum safe integer values
  const maxHeight = Number.MAX_SAFE_INTEGER
  t.notThrows(() => {
    new HashChain(TEST_PUBLIC_KEY, maxHeight, TEST_BLOCK_HASH)
  })
  
  // Test with maximum chunk counts
  const maxChunks = 1000000 // Large but reasonable
  t.notThrows(() => {
    selectChunksV1(TEST_BLOCK_HASH, maxChunks)
  })
  
  // Test with various buffer patterns
  const patterns = [
    Buffer.alloc(32, 0x00),    // All zeros
    Buffer.alloc(32, 0xFF),    // All ones
    Buffer.alloc(32, 0xAA),    // Alternating pattern
  ]
  
  for (const pattern of patterns) {
    t.notThrows(() => {
      new HashChain(pattern, TEST_BLOCK_HEIGHT, pattern)
    })
  }
})

test('error state preservation after validation failures', (t) => {
  const testDir = createTestDir('error_state_preservation')
  const testData = createTestData(16384)
  
  const hashchain = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  hashchain.streamData(testData, testDir)
  
  // Add a valid block
  hashchain.addBlock(Buffer.alloc(32, 0xaa))
  const initialLength = hashchain.getChainLength()
  const initialCommitment = hashchain.getCurrentCommitment()
  
  // Attempt invalid operations
  const invalidOperations = [
    () => hashchain.addBlock(Buffer.alloc(16)),     // Invalid hash length
    () => hashchain.readChunk(-1),                  // Invalid index
    () => hashchain.readChunk(999),                 // Out of range
  ]
  
  for (const invalidOp of invalidOperations) {
    t.throws(() => invalidOp())
    
    // State should be unchanged
    t.is(hashchain.getChainLength(), initialLength)
    t.deepEqual(hashchain.getCurrentCommitment(), initialCommitment)
  }
  
  // Should still be able to perform valid operations
  t.notThrows(() => {
    hashchain.addBlock(Buffer.alloc(32, 0xbb))
    hashchain.readChunk(0)
  })
  
  cleanupTestDir(testDir)
}) 