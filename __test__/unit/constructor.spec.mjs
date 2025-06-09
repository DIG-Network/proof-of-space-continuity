import test from 'ava'
import { HashChain } from '../../index.js'
import { 
  TEST_PUBLIC_KEY, 
  TEST_BLOCK_HASH, 
  TEST_BLOCK_HEIGHT,
  setupTest,
  teardownTest 
} from '../helpers/test-setup.mjs'

// Setup and teardown for each test
test.beforeEach(setupTest)
test.afterEach(teardownTest)

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

test('HashChain constructor validates block height', (t) => {
  // Test negative block height
  t.throws(() => {
    new HashChain(TEST_PUBLIC_KEY, -1, TEST_BLOCK_HASH)
  }, { message: /Block height must be non-negative/ })
})

test('HashChain constructor handles edge case values', (t) => {
  // Test with zero block height (should be valid)
  t.notThrows(() => {
    new HashChain(TEST_PUBLIC_KEY, 0, TEST_BLOCK_HASH)
  })
  
  // Test with very large block height
  t.notThrows(() => {
    new HashChain(TEST_PUBLIC_KEY, Number.MAX_SAFE_INTEGER, TEST_BLOCK_HASH)
  })
})

test('HashChain constructor creates independent instances', (t) => {
  const hashchain1 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  const hashchain2 = new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT + 1, TEST_BLOCK_HASH)
  
  // Instances should be independent
  t.not(hashchain1, hashchain2)
  t.is(hashchain1.getChainLength(), 0)
  t.is(hashchain2.getChainLength(), 0)
})

test('HashChain constructor preserves input parameters', (t) => {
  const customPublicKey = Buffer.from('c'.repeat(64), 'hex')
  const customBlockHeight = 999999
  const customBlockHash = Buffer.from('d'.repeat(64), 'hex')
  
  const hashchain = new HashChain(customPublicKey, customBlockHeight, customBlockHash)
  
  // Get chain info to verify parameters are preserved
  const info = hashchain.getChainInfo()
  t.is(info.initialBlockHeight, customBlockHeight)
})

test('HashChain constructor handles Buffer variations', (t) => {
  // Test with different ways of creating identical buffers
  const publicKey1 = Buffer.from('a'.repeat(64), 'hex')
  const publicKey2 = Buffer.alloc(32, 0xaa)
  const blockHash1 = Buffer.from('b'.repeat(64), 'hex')
  const blockHash2 = Buffer.alloc(32, 0xbb)
  
  t.notThrows(() => {
    new HashChain(publicKey1, TEST_BLOCK_HEIGHT, blockHash1)
  })
  
  t.notThrows(() => {
    new HashChain(publicKey2, TEST_BLOCK_HEIGHT, blockHash2)
  })
})

test('HashChain constructor validates exact buffer lengths', (t) => {
  // Test 31-byte public key (too short)
  t.throws(() => {
    new HashChain(Buffer.alloc(31), TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  }, { message: /Public key must be 32 bytes/ })
  
  // Test 33-byte public key (too long)
  t.throws(() => {
    new HashChain(Buffer.alloc(33), TEST_BLOCK_HEIGHT, TEST_BLOCK_HASH)
  }, { message: /Public key must be 32 bytes/ })
  
  // Test 31-byte block hash (too short)
  t.throws(() => {
    new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, Buffer.alloc(31))
  }, { message: /Block hash must be 32 bytes/ })
  
  // Test 33-byte block hash (too long)
  t.throws(() => {
    new HashChain(TEST_PUBLIC_KEY, TEST_BLOCK_HEIGHT, Buffer.alloc(33))
  }, { message: /Block hash must be 32 bytes/ })
}) 