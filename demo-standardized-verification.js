const { 
  computeProofOfWorkAsync, 
  verifyProofOfWork, 
  verifyProofOfWorkStandardized,
  getAlgorithmVersion,
  getAlgorithmSpec,
  getAlgorithmParameters
} = require('./index.js')

async function demonstrateStandardizedVerification() {
  console.log('=== STANDARDIZED VERIFICATION ALGORITHM ===')
  console.log()
  
  // Show algorithm specification
  const version = getAlgorithmVersion()
  const spec = getAlgorithmSpec()
  const params = getAlgorithmParameters()
  
  console.log('🔒 CONSENSUS ALGORITHM SPECIFICATION:')
  console.log(`   Version: ${version}`)
  console.log(`   Spec Hash: ${spec}`)
  console.log(`   Base Zero Bits: ${params.baseZeroBits}`)
  console.log(`   Log Multiplier: ${params.logMultiplier}`)
  console.log(`   Max Zero Bits: ${params.maxZeroBits}`)
  console.log()
  
  // Mine a solution
  console.log('⛏️  MINING WITH STANDARDIZED ALGORITHM:')
  const entropySeed = Buffer.from('standardized_proof_test', 'utf-8')
  const difficulty = 2.0
  
  console.log(`   Mining difficulty ${difficulty}...`)
  const handle = computeProofOfWorkAsync(entropySeed, difficulty, 10000)
  
  // Wait for solution
  while (!handle.isCompleted() && !handle.hasError()) {
    await new Promise(resolve => setTimeout(resolve, 10))
  }
  
  if (handle.hasError()) {
    console.log('   ❌ Mining failed:', handle.getError())
    return
  }
  
  const result = handle.getResult()
  console.log(`   ✅ Solution found: nonce=${result.nonce}`)
  console.log(`   Hash: ${result.hash}`)
  console.log(`   Attempts: ${result.attempts}`)
  console.log()
  
  // Verify with different methods
  console.log('🔍 VERIFICATION TESTS:')
  
  // Standard verification (can be tampered with)
  const standardValid = verifyProofOfWork(entropySeed, Number(result.nonce), difficulty)
  console.log(`   Standard verification: ${standardValid ? '✅ VALID' : '❌ INVALID'}`)
  
  // Standardized verification (tamper-resistant)
  const standardizedValid = verifyProofOfWorkStandardized(entropySeed, Number(result.nonce), difficulty)
  console.log(`   Standardized verification: ${standardizedValid ? '✅ VALID' : '❌ INVALID'}`)
  
  // Test algorithm version validation
  try {
    const futureVersionValid = verifyProofOfWorkStandardized(entropySeed, Number(result.nonce), difficulty, 999)
    console.log(`   Future version test: ${futureVersionValid ? '✅ VALID' : '❌ INVALID'}`)
  } catch (error) {
    console.log(`   Future version test: ❌ REJECTED (${error.message})`)
  }
  
  console.log()
  console.log('🛡️  SECURITY GUARANTEES:')
  console.log('   ✓ Algorithm version is locked and validated')
  console.log('   ✓ Difficulty formula is consensus-critical and immutable')
  console.log('   ✓ All network participants must use identical verification')
  console.log('   ✓ Tampering with algorithm breaks consensus compatibility')
  console.log('   ✓ Algorithm spec hash prevents silent modifications')
  console.log()
  console.log('📋 USAGE RECOMMENDATIONS:')
  console.log('   🔹 Miners: Can use computeProofOfWorkAsync() with any settings')
  console.log('   🔹 Validators: MUST use verifyProofOfWorkStandardized()')
  console.log('   🔹 Network: MUST validate algorithm version matches')
  console.log('   🔹 Upgrades: Require coordinated hard fork with new version')
  console.log()
  
  // Show formula verification
  console.log('🧮 FORMULA VERIFICATION:')
  console.log(`   Difficulty ${difficulty} target calculation:`)
  console.log(`   zero_bits = ${params.baseZeroBits} + log2(${difficulty}) * ${params.logMultiplier}`)
  console.log(`   zero_bits = ${params.baseZeroBits} + ${Math.log2(difficulty).toFixed(3)} * ${params.logMultiplier}`)
  console.log(`   zero_bits = ${(params.baseZeroBits + Math.log2(difficulty) * params.logMultiplier).toFixed(3)}`)
  console.log()
  console.log('   This formula is now CONSENSUS CRITICAL and cannot be changed!')
}

demonstrateStandardizedVerification().catch(console.error) 