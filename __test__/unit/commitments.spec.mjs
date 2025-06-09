import test from 'ava'
import { 
  createOwnershipCommitment,
  createAnchoredOwnershipCommitment
} from '../../index.js'
import { 
  TEST_PUBLIC_KEY, 
  TEST_BLOCK_HASH, 
  TEST_BLOCK_HEIGHT,
  setupTest,
  teardownTest 
} from '../helpers/test-setup.mjs'
import { createHash } from 'crypto'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

// === Ownership Commitment Tests ===

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

test('createOwnershipCommitment produces unique hashes for different inputs', (t) => {
  const dataHash1 = createHash('sha256').update('test data 1').digest()
  const dataHash2 = createHash('sha256').update('test data 2').digest()
  const publicKey2 = Buffer.from('b'.repeat(64), 'hex')
  
  const commitment1 = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash1)
  const commitment2 = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash2)
  const commitment3 = createOwnershipCommitment(publicKey2, dataHash1)
  
  // Different data hashes should produce different commitments
  t.notDeepEqual(commitment1.commitmentHash, commitment2.commitmentHash)
  
  // Different public keys should produce different commitments
  t.notDeepEqual(commitment1.commitmentHash, commitment3.commitmentHash)
})

test('createOwnershipCommitment handles edge case data', (t) => {
  // Test with all zeros
  const zeroDataHash = Buffer.alloc(32, 0)
  const zeroPublicKey = Buffer.alloc(32, 0)
  
  t.notThrows(() => {
    createOwnershipCommitment(zeroPublicKey, zeroDataHash)
  })
  
  // Test with all 0xFF
  const maxDataHash = Buffer.alloc(32, 0xFF)
  const maxPublicKey = Buffer.alloc(32, 0xFF)
  
  t.notThrows(() => {
    createOwnershipCommitment(maxPublicKey, maxDataHash)
  })
  
  // Results should be different
  const commitment1 = createOwnershipCommitment(zeroPublicKey, zeroDataHash)
  const commitment2 = createOwnershipCommitment(maxPublicKey, maxDataHash)
  
  t.notDeepEqual(commitment1.commitmentHash, commitment2.commitmentHash)
})

test('createOwnershipCommitment commitment hash format', (t) => {
  const dataHash = createHash('sha256').update('test data').digest()
  const commitment = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  
  // Should be exactly 32 bytes
  t.is(commitment.commitmentHash.length, 32)
  
  // Should be a Buffer
  t.true(Buffer.isBuffer(commitment.commitmentHash))
  
  // Should not be all zeros (very low probability)
  const allZeros = Buffer.alloc(32, 0)
  t.notDeepEqual(commitment.commitmentHash, allZeros)
})

// === Anchored Ownership Commitment Tests ===

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

test('createAnchoredOwnershipCommitment is deterministic', (t) => {
  const dataHash = createHash('sha256').update('test data').digest()
  const ownershipCommitment = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  
  const blockCommitment = {
    blockHeight: TEST_BLOCK_HEIGHT,
    blockHash: TEST_BLOCK_HASH
  }
  
  const anchored1 = createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment)
  const anchored2 = createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment)
  
  t.deepEqual(anchored1.anchoredHash, anchored2.anchoredHash)
})

test('createAnchoredOwnershipCommitment produces different hashes for different blocks', (t) => {
  const dataHash = createHash('sha256').update('test data').digest()
  const ownershipCommitment = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  
  const blockCommitment1 = {
    blockHeight: TEST_BLOCK_HEIGHT,
    blockHash: TEST_BLOCK_HASH
  }
  
  const blockCommitment2 = {
    blockHeight: TEST_BLOCK_HEIGHT + 1,
    blockHash: Buffer.from('c'.repeat(64), 'hex')
  }
  
  const anchored1 = createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment1)
  const anchored2 = createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment2)
  
  t.notDeepEqual(anchored1.anchoredHash, anchored2.anchoredHash)
})

test('createAnchoredOwnershipCommitment produces different hashes for different ownership', (t) => {
  const dataHash1 = createHash('sha256').update('test data 1').digest()
  const dataHash2 = createHash('sha256').update('test data 2').digest()
  
  const ownershipCommitment1 = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash1)
  const ownershipCommitment2 = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash2)
  
  const blockCommitment = {
    blockHeight: TEST_BLOCK_HEIGHT,
    blockHash: TEST_BLOCK_HASH
  }
  
  const anchored1 = createAnchoredOwnershipCommitment(ownershipCommitment1, blockCommitment)
  const anchored2 = createAnchoredOwnershipCommitment(ownershipCommitment2, blockCommitment)
  
  t.notDeepEqual(anchored1.anchoredHash, anchored2.anchoredHash)
})

test('createAnchoredOwnershipCommitment preserves input data', (t) => {
  const dataHash = createHash('sha256').update('test data').digest()
  const ownershipCommitment = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  
  const blockCommitment = {
    blockHeight: TEST_BLOCK_HEIGHT,
    blockHash: TEST_BLOCK_HASH
  }
  
  const anchored = createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment)
  
  // Should preserve exact ownership commitment
  t.deepEqual(anchored.ownershipCommitment.publicKey, ownershipCommitment.publicKey)
  t.deepEqual(anchored.ownershipCommitment.dataHash, ownershipCommitment.dataHash)
  t.deepEqual(anchored.ownershipCommitment.commitmentHash, ownershipCommitment.commitmentHash)
  
  // Should preserve exact block commitment
  t.is(anchored.blockCommitment.blockHeight, blockCommitment.blockHeight)
  t.deepEqual(anchored.blockCommitment.blockHash, blockCommitment.blockHash)
})

test('createAnchoredOwnershipCommitment handles various block heights', (t) => {
  const dataHash = createHash('sha256').update('test data').digest()
  const ownershipCommitment = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  
  const blockHeights = [0, 1, 1000, 999999, Number.MAX_SAFE_INTEGER]
  const anchoredHashes = []
  
  for (const height of blockHeights) {
    const blockCommitment = {
      blockHeight: height,
      blockHash: TEST_BLOCK_HASH
    }
    
    const anchored = createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment)
    
    t.is(anchored.blockCommitment.blockHeight, height)
    t.is(anchored.anchoredHash.length, 32)
    
    anchoredHashes.push(anchored.anchoredHash)
  }
  
  // All hashes should be different
  for (let i = 0; i < anchoredHashes.length; i++) {
    for (let j = i + 1; j < anchoredHashes.length; j++) {
      t.notDeepEqual(anchoredHashes[i], anchoredHashes[j])
    }
  }
})

// === Integration Tests for Commitment Chain ===

test('ownership and anchored commitments work together correctly', (t) => {
  // Create test data
  const testData = 'Integration test data for commitments'
  const dataHash = createHash('sha256').update(testData).digest()
  
  // Create ownership commitment
  const ownershipCommitment = createOwnershipCommitment(TEST_PUBLIC_KEY, dataHash)
  
  // Create multiple block commitments to simulate blockchain progression
  const blocks = [
    { height: 100, hash: Buffer.from('a'.repeat(64), 'hex') },
    { height: 101, hash: Buffer.from('b'.repeat(64), 'hex') },
    { height: 102, hash: Buffer.from('c'.repeat(64), 'hex') }
  ]
  
  const anchoredCommitments = []
  
  for (const block of blocks) {
    const blockCommitment = {
      blockHeight: block.height,
      blockHash: block.hash
    }
    
    const anchored = createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment)
    anchoredCommitments.push(anchored)
    
    // Verify structure
    t.is(anchored.anchoredHash.length, 32)
    t.deepEqual(anchored.ownershipCommitment, ownershipCommitment)
    t.deepEqual(anchored.blockCommitment, blockCommitment)
  }
  
  // All anchored commitments should be different (different blocks)
  for (let i = 0; i < anchoredCommitments.length; i++) {
    for (let j = i + 1; j < anchoredCommitments.length; j++) {
      t.notDeepEqual(anchoredCommitments[i].anchoredHash, anchoredCommitments[j].anchoredHash)
    }
  }
  
  // But they should all reference the same ownership commitment
  for (const anchored of anchoredCommitments) {
    t.deepEqual(anchored.ownershipCommitment, ownershipCommitment)
  }
})

test('commitment hash cryptographic properties', (t) => {
  const dataHash = createHash('sha256').update('test data').digest()
  
  // Test avalanche effect - small input change should cause large output change
  const publicKey1 = Buffer.from('a'.repeat(64), 'hex')
  const publicKey2 = Buffer.from('a'.repeat(63) + 'b', 'hex') // One bit difference
  
  const commitment1 = createOwnershipCommitment(publicKey1, dataHash)
  const commitment2 = createOwnershipCommitment(publicKey2, dataHash)
  
  // Count different bits
  let differentBits = 0
  for (let i = 0; i < 32; i++) {
    const xor = commitment1.commitmentHash[i] ^ commitment2.commitmentHash[i]
    for (let bit = 0; bit < 8; bit++) {
      if (xor & (1 << bit)) differentBits++
    }
  }
  
  // Should have substantial bit difference (avalanche effect)
  // For good hash functions, expect ~50% of bits to be different
  t.true(differentBits > 50, `Only ${differentBits} bits different, expected > 50`)
  t.true(differentBits < 200, `Too many bits different: ${differentBits}, expected < 200`)
}) 