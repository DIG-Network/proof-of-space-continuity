import test from 'ava'

import { 
  helloWorld, 
  greet, 
  add, 
  computeProofOfWork, 
  computeProofOfWorkAsync, 
  verifyProofOfWork, 
  hashToDifficulty,
  difficultyToTargetHex,
  startProofOfWorkCancellable
} from '../index.js'

test('helloWorld function returns correct greeting', (t) => {
  const result = helloWorld()
  t.is(result, 'Hello World!')
})

test('greet function returns personalized greeting', (t) => {
  const result = greet('Alice')
  t.is(result, 'Hello, Alice!')
})

test('greet function works with different names', (t) => {
  const result = greet('Bob')
  t.is(result, 'Hello, Bob!')
})

test('add function adds two numbers correctly', (t) => {
  const result = add(2, 3)
  t.is(result, 5)
})

test('add function works with negative numbers', (t) => {
  const result = add(-1, 1)
  t.is(result, 0)
})

test('add function works with zero', (t) => {
  const result = add(0, 5)
  t.is(result, 5)
})

test('computeProofOfWork finds valid nonce for low difficulty', (t) => {
  const entropySeed = Buffer.from('test_entropy_seed_for_mining_12345', 'utf-8')
  const difficulty = 1.0 // Bitcoin difficulty 1.0 (easiest)
  
  const result = computeProofOfWork(entropySeed, difficulty, 100000)
  
  t.truthy(result)
  t.is(typeof result.nonce, 'bigint')
  t.is(typeof result.hash, 'string')
  t.is(typeof result.attempts, 'bigint')
  t.is(typeof result.time_ms, 'number')
  t.is(result.difficulty, difficulty)
  t.is(typeof result.target, 'string')
  
  // Verify the result
  const isValid = verifyProofOfWork(entropySeed, Number(result.nonce), difficulty)
  t.true(isValid)
})

test('computeProofOfWorkAsync finds valid nonce for low difficulty', async (t) => {
  const entropySeed = Buffer.from('async_test_entropy_seed_123456', 'utf-8')
  const difficulty = 1.0 // Bitcoin difficulty 1.0 (easiest)
  
  const result = await computeProofOfWorkAsync(entropySeed, difficulty, 100000)
  
  t.truthy(result)
  t.is(typeof result.nonce, 'bigint')
  t.is(typeof result.hash, 'string')
  t.is(typeof result.attempts, 'bigint')
  t.is(typeof result.time_ms, 'number')
  t.is(result.difficulty, difficulty)
  t.is(typeof result.target, 'string')
  
  // Verify the result
  const isValid = verifyProofOfWork(entropySeed, Number(result.nonce), difficulty)
  t.true(isValid)
})

test('verifyProofOfWork correctly validates nonces', (t) => {
  const entropySeed = Buffer.from('verification_test_entropy', 'utf-8')
  const difficulty = 1.0 // Bitcoin difficulty 1.0
  
  // First find a valid nonce
  const result = computeProofOfWork(entropySeed, difficulty, 50000)
  const validNonce = Number(result.nonce)
  
  // Test valid nonce
  t.true(verifyProofOfWork(entropySeed, validNonce, difficulty))
  
  // Test invalid nonce
  t.false(verifyProofOfWork(entropySeed, validNonce + 1, difficulty))
  
  // Test with different entropy seed
  const differentSeed = Buffer.from('different_entropy_seed', 'utf-8')
  t.false(verifyProofOfWork(differentSeed, validNonce, difficulty))
})

test('hashToDifficulty correctly calculates difficulty', (t) => {
  // Test hash with many leading zeros (high difficulty)
  const hash1 = Buffer.from('00000000ff' + 'a'.repeat(54), 'hex')
  const difficulty1 = hashToDifficulty(hash1)
  t.true(difficulty1 >= 1.0)
  
  // Test hash with fewer leading zeros (lower difficulty)
  const hash2 = Buffer.from('000fff' + 'a'.repeat(58), 'hex')
  const difficulty2 = hashToDifficulty(hash2)
  t.true(difficulty2 >= 1.0)
  
  // Hash1 should have higher difficulty than hash2
  t.true(difficulty1 > difficulty2)
  
  // Test hash with no leading zeros (minimum difficulty)
  const hash3 = Buffer.from('ff' + 'a'.repeat(62), 'hex')
  const difficulty3 = hashToDifficulty(hash3)
  t.true(difficulty3 >= 1.0)
})

test('difficultyToTargetHex converts difficulty to target', (t) => {
  const difficulty1 = 1.0
  const target1 = difficultyToTargetHex(difficulty1)
  t.is(typeof target1, 'string')
  t.is(target1.length, 64) // 32 bytes = 64 hex chars
  
  const difficulty2 = 2.0
  const target2 = difficultyToTargetHex(difficulty2)
  t.is(typeof target2, 'string')
  t.is(target2.length, 64)
  
  // Higher difficulty should result in smaller target
  t.true(target2 < target1)
})

test('proof of work fails when max attempts exceeded', (t) => {
  const entropySeed = Buffer.from('impossible_test_entropy', 'utf-8')
  const difficulty = 1000000.0 // Very high difficulty, should be impossible with low attempts
  const maxAttempts = 10 // Very low max attempts
  
  t.throws(() => {
    computeProofOfWork(entropySeed, difficulty, maxAttempts)
  }, { message: /Failed to find solution after \d+ attempts/ })
})

test('proof of work fails with invalid difficulty', (t) => {
  const entropySeed = Buffer.from('invalid_difficulty_test', 'utf-8')
  
  // Test negative difficulty
  t.throws(() => {
    computeProofOfWork(entropySeed, -1.0, 1000)
  }, { message: /Difficulty must be greater than 0/ })
  
  // Test zero difficulty
  t.throws(() => {
    computeProofOfWork(entropySeed, 0.0, 1000)
  }, { message: /Difficulty must be greater than 0/ })
})

test('startProofOfWorkCancellable returns a handle', (t) => {
  const entropySeed = Buffer.from('cancellable_test_entropy', 'utf-8')
  const difficulty = 1.0 // Low difficulty for quick completion
  
  const handle = startProofOfWorkCancellable(entropySeed, difficulty, 50000)
  
  t.truthy(handle)
  t.is(typeof handle.cancel, 'function')
  t.is(typeof handle.isCancelled, 'function')
  t.is(typeof handle.getAttempts, 'function')
  t.is(typeof handle.isCompleted, 'function')
  t.is(typeof handle.getProgress, 'function')
  
  // Clean up
  handle.cancel()
})

test('cancellable proof of work can be cancelled', (t) => {
  const entropySeed = Buffer.from('cancel_test_entropy_hard', 'utf-8')
  const difficulty = 10000.0 // High difficulty to ensure it runs long enough
  
  const handle = startProofOfWorkCancellable(entropySeed, difficulty, 10000000)
  
  // Cancel immediately
  handle.cancel()
  
  // Should be cancelled
  t.true(handle.isCancelled())
})

test('cancellable proof of work reports progress', async (t) => {
  const entropySeed = Buffer.from('progress_test_entropy', 'utf-8')
  const difficulty = 100.0 // Medium difficulty
  
  const handle = startProofOfWorkCancellable(entropySeed, difficulty, 50000)
  
  // Wait a bit for some progress
  await new Promise(resolve => setTimeout(resolve, 100))
  
  const progress = handle.getProgress()
  t.truthy(progress)
  t.is(typeof progress.attempts, 'bigint')
  t.is(typeof progress.nonce, 'bigint')
  t.is(typeof progress.elapsed_ms, 'number')
  t.is(typeof progress.attempts_per_second, 'number')
  
  // Clean up
  handle.cancel()
})

test('cancellable proof of work detects completion', async (t) => {
  const entropySeed = Buffer.from('completion_test_entropy', 'utf-8')
  const difficulty = 1.0 // Very low difficulty for quick completion
  
  const handle = startProofOfWorkCancellable(entropySeed, difficulty, 100000)
  
  // Wait for completion (should be quick with difficulty 1.0)
  let completed = false
  for (let i = 0; i < 50; i++) {
    await new Promise(resolve => setTimeout(resolve, 100))
    if (handle.isCompleted()) {
      completed = true
      break
    }
  }
  
  t.true(completed, 'Proof of work should complete with low difficulty')
})

test('proof of work with logging enabled works correctly', (t) => {
  const entropySeed = Buffer.from('logging_test_entropy', 'utf-8')
  const difficulty = 1.0 // Low difficulty for quick completion
  
  // Test with logging enabled - should not throw and should find solution
  const result = computeProofOfWork(entropySeed, difficulty, 100000, true)
  
  t.truthy(result)
  t.is(typeof result.nonce, 'bigint')
  t.is(typeof result.hash, 'string')
  t.is(typeof result.attempts, 'bigint')
  t.is(typeof result.time_ms, 'number')
  t.is(result.difficulty, difficulty)
  t.is(typeof result.target, 'string')
  
  // Verify the result
  const isValid = verifyProofOfWork(entropySeed, Number(result.nonce), difficulty)
  t.true(isValid)
})

test('double SHA-256 vs single SHA-256 produces different results', (t) => {
  const entropySeed = Buffer.from('sha_comparison_test', 'utf-8')
  const difficulty = 1.0
  
  // Test with double SHA-256 (Bitcoin style)
  const resultDouble = computeProofOfWork(entropySeed, difficulty, 100000, false, true)
  
  // Test with single SHA-256
  const resultSingle = computeProofOfWork(entropySeed, difficulty, 100000, false, false)
  
  // Both should succeed but produce different results
  t.truthy(resultDouble)
  t.truthy(resultSingle)
  t.not(resultDouble.hash, resultSingle.hash)
  
  // Verify both results with their respective hash functions
  t.true(verifyProofOfWork(entropySeed, Number(resultDouble.nonce), difficulty, true))
  t.true(verifyProofOfWork(entropySeed, Number(resultSingle.nonce), difficulty, false))
  
  // Cross-verification should fail
  t.false(verifyProofOfWork(entropySeed, Number(resultDouble.nonce), difficulty, false))
  t.false(verifyProofOfWork(entropySeed, Number(resultSingle.nonce), difficulty, true))
}) 