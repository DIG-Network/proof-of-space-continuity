const { computeProofOfWorkAsync, difficultyToTargetHex, verifyProofOfWork } = require('./index.js')

// Parse command line arguments
const args = process.argv.slice(2)
const difficulty = args.length > 0 ? parseFloat(args[0]) : 1.0
const maxAttempts = args.length > 1 ? parseInt(args[1]) : undefined // undefined = unlimited
const logAttempts = args.includes('--log') || args.includes('-l')
const doubleSha = !args.includes('--single-sha')

// Validate difficulty
if (isNaN(difficulty) || difficulty <= 0) {
  console.error('❌ Invalid difficulty. Must be a positive number.')
  console.log('Usage: node example.js [difficulty] [maxAttempts] [--log] [--single-sha]')
  console.log('Examples:')
  console.log('  node example.js 1.0               # Unlimited attempts')
  console.log('  node example.js 2.0 500000 --log  # Limited attempts with logging')
  console.log('  node example.js 4.0 --single-sha  # Single SHA-256')
  process.exit(1)
}

async function runProofOfWorkTest() {
  console.log('🔥 Bitcoin-Style Proof of Work Test (Cancellable)\n')

  // Show configuration
  console.log('⚙️  Configuration:')
  console.log(`   Difficulty: ${difficulty}`)
  console.log(`   Max Attempts: ${maxAttempts ? maxAttempts.toLocaleString() : 'Unlimited'}`)
  console.log(`   Hash Algorithm: ${doubleSha ? 'Double SHA-256 (Bitcoin-style)' : 'Single SHA-256'}`)
  console.log(`   Logging: ${logAttempts ? 'Enabled' : 'Disabled'}`)
  console.log(`   Mode: Cancellable (Press Ctrl+C to stop)`)

  const entropySeed = Buffer.from('proof_of_work_test_12345', 'utf-8')
  const target = difficultyToTargetHex(difficulty)

  console.log(`   Entropy Seed: "${entropySeed.toString('utf-8')}"`)
  console.log(`   Target: ${target.substring(0, 16)}...`)
  console.log()

  // Start the cancellable proof of work
  console.log('🚀 Starting cancellable proof of work...')
  console.log('   Press Ctrl+C to cancel at any time')
  const startTime = Date.now()

  try {
    const handle = computeProofOfWorkAsync(entropySeed, difficulty, maxAttempts, logAttempts, doubleSha)
    
    // Set up signal handler for graceful cancellation
    let cancelled = false
    process.on('SIGINT', () => {
      if (!cancelled) {
        cancelled = true
        console.log('\n🛑 Cancelling proof of work...')
        handle.cancel()
      } else {
        console.log('\n💥 Force exit')
        process.exit(1)
      }
    })

    // Monitor progress and wait for completion
    const progressInterval = setInterval(() => {
      const attempts = handle.getAttempts()
      if (Number(attempts) > 0) {
        const elapsed = Date.now() - startTime
        const rate = Number(attempts) / (elapsed / 1000)
        process.stdout.write(`\r⏳ Attempts: ${Number(attempts).toLocaleString()}, Rate: ${Math.floor(rate)} attempts/sec`)
      }
    }, 1000)

    // Wait for completion or error
    await new Promise((resolve, reject) => {
      // Give the task a moment to start
      setTimeout(() => {
        const checkCompletion = setInterval(() => {
          if (handle.isCompleted()) {
            clearInterval(checkCompletion)
            clearInterval(progressInterval)
            resolve()
          } else if (handle.hasError()) {
            clearInterval(checkCompletion)
            clearInterval(progressInterval)
            const error = handle.getError()
            reject(new Error(error || 'Unknown error'))
          }
          // If still running, check if we should give up (only if max attempts is set)
          else if (maxAttempts) {
            const attempts = Number(handle.getAttempts())
            if (attempts >= maxAttempts) {
              clearInterval(checkCompletion)
              clearInterval(progressInterval)
              reject(new Error(`Reached maximum attempts: ${maxAttempts}`))
            }
          }
        }, 100)
      }, 50) // Give task 50ms to start
    })

    const elapsed = Date.now() - startTime
    console.log('\n') // New line after progress display

    // Get the result
    const result = handle.getResult()
    if (result) {
      const rate = Number(result.attempts) / (elapsed / 1000)
      
      console.log('✅ Success! Solution found:')
      console.log(`   Nonce: ${result.nonce}`)
      console.log(`   Hash: ${result.hash}`)
      console.log(`   Target: ${result.target}`)
      console.log(`   Attempts: ${result.attempts.toLocaleString()}`)
      console.log(`   Time: ${elapsed.toLocaleString()}ms`)
      console.log(`   Rate: ${rate.toFixed(0)} attempts/second`)
      console.log(`   Difficulty: ${result.difficulty}`)
      
      // Show that hash meets target
      const hashMeetsTarget = result.hash <= result.target
      console.log(`   ✅ Hash ≤ Target: ${hashMeetsTarget}`)
      
      // Verify the proof of work
      console.log('\n🔍 Verifying proof of work...')
      const isValid = verifyProofOfWork(entropySeed, Number(result.nonce), difficulty, doubleSha)
      
      if (isValid) {
        console.log('✅ Verification successful! The proof of work is valid.')
        console.log(`   ✓ Nonce ${result.nonce} produces a hash that meets difficulty ${difficulty}`)
        console.log(`   ✓ Hash: ${result.hash}`)
        console.log(`   ✓ Target: ${result.target}`)
        console.log(`   ✓ Hash ≤ Target: ${result.hash <= result.target}`)
      } else {
        console.log('❌ Verification failed! The proof of work is invalid.')
        console.log(`   ✗ Nonce ${result.nonce} does NOT produce a valid hash`)
        console.log(`   ✗ This should never happen - there may be a bug in the implementation`)
        process.exit(1)
      }
    } else {
      console.log('❌ No result available despite completion')
      process.exit(1)
    }
    
  } catch (error) {
    const elapsed = Date.now() - startTime
    console.log(`\n❌ ${error.message}`)
    console.log(`   Time: ${elapsed.toLocaleString()}ms`)
    
    if (error.message.includes('cancelled')) {
      console.log('   Operation was cancelled by user')
    } else if (maxAttempts) {
      console.log(`   Max attempts reached: ${maxAttempts.toLocaleString()}`)
      console.log('\n💡 Try:')
      console.log(`   - Lower difficulty: node example.js ${Math.max(0.5, difficulty - 1)}`)
      console.log(`   - More attempts: node example.js ${difficulty} ${maxAttempts * 10}`)
    } else {
      console.log('   This should not happen with unlimited attempts - there may be a bug')
    }
    process.exit(1)
  }

  console.log('\n🎉 Cancellable test completed successfully!')
  console.log('✅ Both proof of work generation and verification passed!')
  console.log('🚀 The computation was cancellable and could be stopped at any time!')
  process.exit(0)
}

// Run the async test
runProofOfWorkTest().catch((error) => {
  console.error('❌ Test failed:', error)
  process.exit(1)
})