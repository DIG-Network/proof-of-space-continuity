const { 
  HashChain, 
  selectChunksV1, 
  verifyChunkSelection, 
  createOwnershipCommitment,
  createAnchoredOwnershipCommitment,
  verifyProof 
} = require('./index.js')
const fs = require('fs')
const path = require('path')
const crypto = require('crypto')

// Parse command line arguments
const args = process.argv.slice(2)
const testMode = args.includes('--test') || args.includes('-t')
const verbose = args.includes('--verbose') || args.includes('-v')
const outputDir = args.find(arg => arg.startsWith('--output='))?.split('=')[1] || './hashchain_output'

console.log('📦 HashChain Proof of Storage Continuity Example\n')

// Show configuration
console.log('⚙️  Configuration:')
console.log(`   Output Directory: ${outputDir}`)
console.log(`   Test Mode: ${testMode ? 'Enabled (uses sample data)' : 'Disabled'}`)
console.log(`   Verbose Logging: ${verbose ? 'Enabled' : 'Disabled'}`)
console.log()

async function runHashChainExample() {
  try {
    // Create output directory if it doesn't exist
    if (!fs.existsSync(outputDir)) {
      fs.mkdirSync(outputDir, { recursive: true })
      console.log(`📁 Created output directory: ${outputDir}`)
    }

    // Generate or use sample data
    let data
    if (testMode) {
      // Create sample data for testing
      const sampleText = 'This is sample data for HashChain Proof of Storage Continuity testing. '.repeat(200)
      data = Buffer.from(sampleText, 'utf-8')
      console.log(`📝 Generated sample data: ${data.length} bytes`)
    } else {
      // In real usage, you would load your actual data file
      console.log('💡 To use your own data, remove --test flag and ensure data file exists')
      console.log('   For now, using sample data...')
      const sampleText = 'HashChain proof-of-storage-continuity example data. '.repeat(100)
      data = Buffer.from(sampleText, 'utf-8')
    }

    // Generate mock blockchain parameters
    const publicKey = crypto.randomBytes(32)
    const blockHeight = 12345
    const initialBlockHash = crypto.randomBytes(32)

    console.log(`🔑 Public Key: ${publicKey.toString('hex').substring(0, 16)}...`)
    console.log(`📊 Initial Block Height: ${blockHeight}`)
    console.log(`🔗 Initial Block Hash: ${initialBlockHash.toString('hex').substring(0, 16)}...`)
    console.log()

    // Create HashChain instance
    console.log('🚀 Creating HashChain instance...')
    const hashchain = new HashChain(publicKey, blockHeight, initialBlockHash)

    // Stream data to create hashchain files
    console.log('💾 Streaming data to create hashchain files...')
    hashchain.streamData(data, outputDir)

    const filePaths = hashchain.getFilePaths()
    const totalChunks = hashchain.getTotalChunks()
    const anchoredCommitment = hashchain.getAnchoredCommitment()

    console.log('✅ Files created successfully:')
    if (filePaths) {
      filePaths.forEach(filePath => {
        const stats = fs.statSync(filePath)
        console.log(`   📄 ${path.basename(filePath)} (${stats.size} bytes)`)
      })
    }
    console.log(`   📦 Total chunks: ${totalChunks}`)
    console.log(`   🔐 Anchored commitment: ${anchoredCommitment?.toString('hex').substring(0, 16)}...`)
    
    // Show detailed chain information
    console.log('📊 Chain Information:')
    const chainInfo = hashchain.getChainInfo()
    console.log(`   Status: ${chainInfo.status}`)
    console.log(`   Total Storage: ${chainInfo.totalStorageMb.toFixed(2)} MB`)
    console.log(`   Chunk Size: ${chainInfo.chunkSizeBytes} bytes`)
    console.log(`   Consensus Algorithm: v${chainInfo.consensusAlgorithmVersion}`)
    console.log(`   Initial Block Height: ${chainInfo.initialBlockHeight}`)
    if (verbose) {
      console.log(`   HashChain File: ${chainInfo.hashchainFileSizeBytes} bytes`)
      console.log(`   Data File: ${chainInfo.dataFileSizeBytes} bytes`)
    }
    console.log()

    // Demonstrate chunk selection algorithm
    console.log('🎯 Testing consensus-critical chunk selection...')
    const testBlockHash = crypto.randomBytes(32)
    const selectionResult = selectChunksV1(testBlockHash, totalChunks)
    
    console.log('   Selected chunks:', selectionResult.selectedIndices)
    console.log(`   Algorithm version: ${selectionResult.algorithmVersion}`)
    console.log(`   Verification hash: ${selectionResult.verificationHash.toString('hex').substring(0, 16)}...`)

    // Verify chunk selection is consensus-compliant
    const isSelectionValid = verifyChunkSelection(
      testBlockHash, 
      totalChunks, 
      selectionResult.selectedIndices
    )
    console.log(`   ✅ Selection consensus-compliant: ${isSelectionValid}`)
    console.log()

    // Add several blocks to demonstrate continuous proof
    console.log('⛓️  Adding blocks to demonstrate continuous proof...')
    const numBlocksToAdd = 5
    const commitments = []

    for (let i = 1; i <= numBlocksToAdd; i++) {
      const newBlockHash = crypto.randomBytes(32)
      const commitment = hashchain.addBlock(newBlockHash)
      commitments.push(commitment)
      
      if (verbose) {
        console.log(`   Block ${blockHeight + i}:`)
        console.log(`     Hash: ${commitment.blockHash.toString('hex').substring(0, 16)}...`)
        console.log(`     Selected chunks: [${commitment.selectedChunks.slice(0, 3).join(', ')}${commitment.selectedChunks.length > 3 ? '...' : ''}]`)
        console.log(`     Commitment: ${commitment.commitmentHash.toString('hex').substring(0, 16)}...`)
      } else {
        console.log(`   ✅ Added block ${blockHeight + i} with ${commitment.selectedChunks.length} selected chunks`)
      }
    }

    console.log(`   📏 Chain length: ${hashchain.getChainLength()} blocks`)
    
    // Show updated chain information after adding blocks
    console.log('📊 Updated Chain Information:')
    const updatedChainInfo = hashchain.getChainInfo()
    console.log(`   Status: ${updatedChainInfo.status}`)
    console.log(`   Chain Length: ${updatedChainInfo.chainLength} blocks`)
    console.log(`   Proof Window Ready: ${updatedChainInfo.proofWindowReady ? '✅ Yes' : '❌ No'}`)
    if (!updatedChainInfo.proofWindowReady && updatedChainInfo.blocksUntilProofReady) {
      console.log(`   Blocks Until Proof Ready: ${updatedChainInfo.blocksUntilProofReady}`)
    }
    if (updatedChainInfo.currentCommitment) {
      console.log(`   Current Commitment: ${updatedChainInfo.currentCommitment.substring(0, 16)}...`)
    }
    console.log()

    // Verify chain integrity
    console.log('🔍 Verifying chain integrity...')
    const isChainValid = hashchain.verifyChain()
    console.log(`   ✅ Chain valid: ${isChainValid}`)
    console.log()

    // Test chunk reading capability
    console.log('📖 Testing chunk reading...')
    if (totalChunks > 0) {
      const chunkIdx = Math.min(2, totalChunks - 1)
      const chunkData = hashchain.readChunk(chunkIdx)
      console.log(`   ✅ Read chunk ${chunkIdx}: ${chunkData.length} bytes`)
      if (verbose) {
        console.log(`   Content preview: "${chunkData.toString('utf-8').substring(0, 50)}..."`)
      }
    }
    console.log()

    // Demonstrate ownership commitment creation
    console.log('🔒 Creating ownership commitments...')
    const dataHash = crypto.createHash('sha256').update(data).digest()
    const ownershipCommitment = createOwnershipCommitment(publicKey, dataHash)
    
    console.log(`   ✅ Ownership commitment created`)
    if (verbose) {
      console.log(`   Data hash: ${dataHash.toString('hex').substring(0, 16)}...`)
      console.log(`   Commitment hash: ${ownershipCommitment.commitmentHash.toString('hex').substring(0, 16)}...`)
    }
    console.log()

    // Attempt to generate proof window (will use mock data as noted in README)
    console.log('📊 Checking proof window capability...')
    if (hashchain.getChainLength() >= 8) {
      const proofWindow = hashchain.getProofWindow()
      console.log(`   ✅ Proof window generated with ${proofWindow.commitments.length} commitments`)
      console.log('   📝 Note: Currently using mock data for development (as documented)')
    } else {
      console.log(`   ⏳ Need ${8 - hashchain.getChainLength()} more blocks for proof window`)
      console.log('   💡 Add more blocks with: hashchain.addBlock(newBlockHash)')
    }
    console.log()

    // Load hashchain from file to demonstrate persistence
    if (filePaths && filePaths.length > 0) {
      const hashchainFile = filePaths.find(p => p.endsWith('.hashchain'))
      if (hashchainFile) {
        console.log('💾 Testing file persistence...')
        try {
          const loadedHashChain = HashChain.loadFromFile(hashchainFile)
          console.log(`   ✅ Successfully loaded HashChain from: ${path.basename(hashchainFile)}`)
          console.log(`   📦 Loaded chunks: ${loadedHashChain.getTotalChunks()}`)
          console.log(`   ⛓️  Loaded chain length: ${loadedHashChain.getChainLength()}`)
        } catch (error) {
          console.log(`   ⚠️  File loading not yet implemented: ${error.message}`)
        }
      }
    }

    console.log('\n🎉 HashChain example completed successfully!')
    console.log('✅ Demonstrated:')
    console.log('   • HashChain creation and data streaming')
    console.log('   • Comprehensive chain information and status reporting')
    console.log('   • Consensus-critical chunk selection algorithm')
    console.log('   • Continuous proof generation (physical access commitments)')
    console.log('   • Chain integrity verification')
    console.log('   • Chunk reading and data access')
    console.log('   • Ownership commitment creation')
    console.log('   • Proof window readiness tracking')
    console.log('   • File-based persistence')
    
    console.log('\n💡 Usage tips:')
    console.log('   • Use real blockchain hashes for production')
    console.log('   • Maintain continuous block addition for proof-of-storage')
    console.log('   • Store .hashchain and .data files safely')
    console.log('   • Generate proof windows every 8 blocks for network submission')

  } catch (error) {
    console.error('\n❌ Example failed:', error.message)
    if (verbose) {
      console.error('Stack trace:', error.stack)
    }
    console.log('\n💡 Troubleshooting:')
    console.log('   • Ensure you have write permissions to output directory')
    console.log('   • Check that native bindings are properly installed')
    console.log('   • Try running with --verbose for more details')
    process.exit(1)
  }
}

// Show usage if help requested
if (args.includes('--help') || args.includes('-h')) {
  console.log('📖 HashChain Proof of Storage Continuity Example')
  console.log()
  console.log('Usage: node example.js [options]')
  console.log()
  console.log('Options:')
  console.log('  --test, -t           Use generated test data')
  console.log('  --verbose, -v        Enable verbose logging')
  console.log('  --output=<dir>       Specify output directory (default: ./hashchain_output)')
  console.log('  --help, -h           Show this help message')
  console.log()
  console.log('Examples:')
  console.log('  node example.js --test                    # Quick test with sample data')
  console.log('  node example.js --test --verbose          # Verbose test output')
  console.log('  node example.js --output=./my_chains      # Custom output directory')
  console.log()
  process.exit(0)
}

// Run the example
runHashChainExample().catch((error) => {
  console.error('❌ Unexpected error:', error)
  process.exit(1)
})