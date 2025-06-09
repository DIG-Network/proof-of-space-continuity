const { computeProofOfWorkAsync, hashToDifficulty, verifyProofOfWork } = require('./index.js')

console.log('Testing hex digit difficulty calculation...')

// Helper function to wait for completion
async function waitForCompletion(handle, timeoutMs = 30000) {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (handle.isCompleted()) {
      return handle.getResult()
    }
    if (handle.hasError()) {
      throw new Error(handle.getError())
    }
    await new Promise(resolve => setTimeout(resolve, 10))
  }
  throw new Error('Timeout waiting for completion')
}

async function testDifficulty() {
  // Test 1: Find a nonce with difficulty 2
  const entropySeed = Buffer.from('test_seed_for_hex_difficulty', 'utf-8')
  const difficulty = 2

  console.log(`\n🎯 Testing difficulty ${difficulty}`)
  console.log(`📍 Entropy seed: "${entropySeed.toString('utf-8')}"`)

  try {
    const handle = computeProofOfWorkAsync(entropySeed, difficulty, 100000, false)
    const result = await waitForCompletion(handle)
    
    console.log('✅ Found solution!')
    console.log(`   Nonce: ${result.nonce}`)
    console.log(`   Hash: ${result.hash}`)
    console.log(`   Attempts: ${result.attempts}`)
    console.log(`   Time: ${result.time_ms}ms`)
    console.log(`   Achieved Difficulty: ${result.difficulty}`)
    
    console.log(`\n🔍 Verification:`)
    
    // Verify using verifyProofOfWork
    const isValid = verifyProofOfWork(entropySeed, Number(result.nonce), difficulty)
    console.log(`   Valid proof: ${isValid ? '✅ YES' : '❌ NO'}`)
    
    // Test hashToDifficulty function
    const hashBuffer = Buffer.from(result.hash, 'hex')
    const measuredDifficulty = hashToDifficulty(hashBuffer)
    console.log(`   Measured difficulty: ${measuredDifficulty}`)
    console.log(`   Meets requirement: ${measuredDifficulty >= difficulty ? '✅ YES' : '❌ NO'}`)
    
  } catch (error) {
    console.error('❌ Failed to find solution:', error.message)
  }

  // Test 2: Manually test the hashToDifficulty function
  console.log(`\n🧪 Manual hashToDifficulty tests:`)

  const testCases = [
    { hash: '00abcdef', description: '2 leading zero hex digits' },
    { hash: '000abcde', description: '3 leading zero hex digits' },
    { hash: '0000abcd', description: '4 leading zero hex digits' },
    { hash: 'abcdef00', description: 'no leading zeros' },
    { hash: '0abcdef0', description: '1 leading zero hex digit' },
  ]

  testCases.forEach(({ hash, description }) => {
    const hashBuffer = Buffer.from(hash.padEnd(64, '0'), 'hex')
    const measured = hashToDifficulty(hashBuffer)
    console.log(`   📊 ${hash} -> difficulty ${measured.toFixed(1)} - ${description}`)
  })

  console.log('\n✨ Hex digit difficulty testing complete!')
}

// Run the test
testDifficulty().catch(console.error) 