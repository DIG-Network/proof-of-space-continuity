const { computeProofOfWork, countDifficulty } = require('./index.js')

console.log('Testing hex digit difficulty calculation...')

// Test 1: Find a nonce with difficulty 2 (2 leading zero hex digits)
const entropySeed = Buffer.from('test_seed_for_hex_difficulty', 'utf-8')
const difficulty = 2

console.log(`\n🎯 Testing difficulty ${difficulty} (${difficulty} leading zero hex digits)`)
console.log(`📍 Entropy seed: "${entropySeed.toString('utf-8')}"`)

try {
  const result = computeProofOfWork(entropySeed, difficulty, 100000, 0)
  
  console.log('✅ Found solution!')
  console.log(`   Nonce: ${result.nonce}`)
  console.log(`   Hash: ${result.hash}`)
  console.log(`   Attempts: ${result.attempts}`)
  console.log(`   Time: ${result.time_ms}ms`)
  console.log(`   Achieved Difficulty: ${result.difficulty}`)
  
  // Verify the hash starts with the expected number of zeros
  const expectedPrefix = '0'.repeat(difficulty)
  const actualPrefix = result.hash.substring(0, difficulty)
  
  console.log(`\n🔍 Verification:`)
  console.log(`   Expected prefix: "${expectedPrefix}"`)
  console.log(`   Actual prefix: "${actualPrefix}"`)
  console.log(`   Matches: ${actualPrefix === expectedPrefix ? '✅ YES' : '❌ NO'}`)
  
  // Test countDifficulty function
  const hashBuffer = Buffer.from(result.hash, 'hex')
  const measuredDifficulty = countDifficulty(hashBuffer)
  console.log(`   Measured difficulty: ${measuredDifficulty}`)
  console.log(`   Meets requirement: ${measuredDifficulty >= difficulty ? '✅ YES' : '❌ NO'}`)
  
} catch (error) {
  console.error('❌ Failed to find solution:', error.message)
}

// Test 2: Manually test the countDifficulty function
console.log(`\n🧪 Manual countDifficulty tests:`)

const testCases = [
  { hash: '00abcdef', expected: 2, description: '2 leading zero hex digits' },
  { hash: '000abcde', expected: 3, description: '3 leading zero hex digits' },
  { hash: '0000abcd', expected: 4, description: '4 leading zero hex digits' },
  { hash: 'abcdef00', expected: 0, description: 'no leading zeros' },
  { hash: '0abcdef0', expected: 1, description: '1 leading zero hex digit' },
]

testCases.forEach(({ hash, expected, description }) => {
  const hashBuffer = Buffer.from(hash.padEnd(64, '0'), 'hex')
  const measured = countDifficulty(hashBuffer)
  const result = measured === expected ? '✅' : '❌'
  console.log(`   ${result} ${hash} -> ${measured} (expected ${expected}) - ${description}`)
})

console.log('\n✨ Hex digit difficulty testing complete!') 