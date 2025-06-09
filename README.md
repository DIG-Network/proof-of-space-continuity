# HashChain Proof of Storage Continuity

A high-performance HashChain Proof of Storage Continuity (PoSC) library for Node.js, built with Rust and NAPI bindings. This library enables cryptographic proof that data remains continuously accessible over time using blockchain entropy.

## Overview

HashChain implements a Proof of Storage Continuity system where provers must demonstrate continuous possession of data over time. The system uses blockchain block hashes as entropy sources to create unpredictable data access patterns, preventing pre-computation attacks and ensuring genuine data availability.

## Features

- **Consensus-Critical Implementation**: Network-standardized algorithms ensuring compatibility across all participants
- **Cryptographic Security**: Production-grade Merkle proof verification with SHA256 hashing
- **Blockchain Integration**: Uses Chia blockchain block hashes as entropy sources
- **File-Based Storage**: Separate `.data` and `.hashchain` files with SHA256-based naming
- **Continuous Proof Generation**: 8-block proof windows with 16-second intervals
- **Deterministic Chunk Selection**: Consensus-compliant algorithm preventing manipulation
- **Memory Efficient**: Minimal memory footprint supporting hundreds of concurrent instances
- **Cross-platform Support**: Builds for Windows, macOS, and Linux
- **Multiple Architectures**: Supports x64 and ARM64 architectures
- **TypeScript Support**: Full TypeScript definitions included

## Installation

```bash
npm install @dignetwork/proof-of-space-continuity
```

## Quick Start

```javascript
const { HashChain } = require('@dignetwork/proof-of-space-continuity')
const fs = require('fs')

async function createHashChain() {
  // Initialize with blockchain parameters
  const publicKey = Buffer.from('your_32_byte_public_key_here...', 'hex') // 32 bytes
  const blockHeight = 12345
  const blockHash = Buffer.from('blockchain_block_hash_32_bytes...', 'hex') // 32 bytes
  
  // Create new HashChain instance
  const hashchain = new HashChain(publicKey, blockHeight, blockHash)
  
  // Load your data
  const data = fs.readFileSync('your_file.bin')
  
  // Stream data to create hashchain files (named by SHA256 of data)
  const outputDir = './hashchain_storage'
  hashchain.streamData(data, outputDir)
  
  console.log('Files created:', hashchain.getFilePaths())
  console.log('Total chunks:', hashchain.getTotalChunks())
  console.log('Anchored commitment:', hashchain.getAnchoredCommitment()?.toString('hex'))
  
  // Add blockchain blocks to continue the proof of storage continuity
  const newBlockHash = Buffer.from('next_block_hash_32_bytes...', 'hex')
  const commitment = hashchain.addBlock(newBlockHash)
  
  console.log('Physical access commitment created:')
  console.log('- Selected chunks:', commitment.selectedChunks)
  console.log('- Block hash:', commitment.blockHash.toString('hex'))
  console.log('- Commitment hash:', commitment.commitmentHash.toString('hex'))
  
  // Verify chain integrity
  const isValid = hashchain.verifyChain()
  console.log('Chain valid:', isValid)
  
  // Generate proof window after 8 blocks
  if (hashchain.getChainLength() >= 8) {
    const proofWindow = hashchain.getProofWindow()
    console.log('Proof window generated with', proofWindow.commitments.length, 'commitments')
  }
}

createHashChain().catch(console.error)
```

## API Reference

### HashChain Class

#### `new HashChain(publicKey, blockHeight, blockHash)`

Creates a new HashChain instance for Proof of Storage Continuity.

**Parameters:**
- `publicKey` (Buffer): 32-byte public key for ownership commitment
- `blockHeight` (number): Initial blockchain block height
- `blockHash` (Buffer): Initial blockchain block hash (32 bytes)

#### `streamData(data, outputDir): void`

Streams data to files with SHA256-based naming, creating `.data` and `.hashchain` files.

**Parameters:**
- `data` (Buffer): The data to stream and create proofs for
- `outputDir` (string): Directory path for output files

**Files Created:**
- `{sha256}.data` - Raw data file chunked into 4KB segments
- `{sha256}.hashchain` - Metadata file with Merkle tree and chain links

#### `addBlock(blockHash): PhysicalAccessCommitment`

Adds a new blockchain block to the hash chain, creating a physical access commitment.

**Parameters:**
- `blockHash` (Buffer): New blockchain block hash (32 bytes)

**Returns:** `PhysicalAccessCommitment` object with selected chunks and proofs

#### `getProofWindow(): ProofWindow`

Gets proof window for the last 8 blocks (required for network submission).

**Returns:** `ProofWindow` object containing commitments and Merkle proofs

**Requirements:** Chain must have at least 8 blocks

#### `verifyChain(): boolean`

Verifies the entire hash chain follows network consensus rules.

**Returns:** `true` if chain is valid, `false` otherwise

#### `readChunk(chunkIdx): Buffer`

Reads a specific 4KB chunk from the data file.

**Parameters:**
- `chunkIdx` (number): Index of the chunk to read

**Returns:** 4KB chunk data

#### Getters

- `getChainLength(): number` - Current number of blocks in chain
- `getTotalChunks(): number` - Total number of 4KB chunks in data
- `getCurrentCommitment(): Buffer | null` - Latest commitment hash
- `getAnchoredCommitment(): Buffer | null` - Initial anchored commitment
- `getFilePaths(): string[] | null` - Paths to `.hashchain` and `.data` files

#### `static loadFromFile(hashchainFilePath): HashChain`

Loads an existing HashChain from a `.hashchain` file.

**Parameters:**
- `hashchainFilePath` (string): Path to existing `.hashchain` file

**Returns:** Loaded `HashChain` instance

### Consensus-Critical Functions

#### `selectChunksV1(blockHash, totalChunks): ChunkSelectionResult`

**CONSENSUS CRITICAL:** Standardized chunk selection algorithm V1.

**Parameters:**
- `blockHash` (Buffer): Block hash for entropy (32 bytes)
- `totalChunks` (number): Total chunks in file

**Returns:** `ChunkSelectionResult` with selected indices and verification hash

#### `verifyChunkSelection(blockHash, totalChunks, claimedIndices, expectedVersion?): boolean`

Verifies chunk selection matches network consensus algorithm.

**Parameters:**
- `blockHash` (Buffer): Block hash used for selection
- `totalChunks` (number): Total chunks in file
- `claimedIndices` (number[]): Claimed selected chunk indices
- `expectedVersion` (number, optional): Expected algorithm version

**Returns:** `true` if selection is consensus-compliant

#### `verifyProof(proofWindow, anchoredCommitment, merkleRoot, totalChunks): boolean`

**CONSENSUS CRITICAL:** Verifies a complete proof window with cryptographic validation.

**Parameters:**
- `proofWindow` (ProofWindow): Proof window to verify
- `anchoredCommitment` (Buffer): Original anchored commitment (32 bytes)
- `merkleRoot` (Buffer): Merkle root for data integrity (32 bytes)
- `totalChunks` (number): Total chunks in original data

**Returns:** `true` if proof is cryptographically valid

### Commitment Functions

#### `createOwnershipCommitment(publicKey, dataHash): OwnershipCommitment`

Creates an ownership commitment binding data to a public key.

**Parameters:**
- `publicKey` (Buffer): 32-byte public key
- `dataHash` (Buffer): 32-byte SHA256 hash of data

**Returns:** `OwnershipCommitment` object

#### `createAnchoredOwnershipCommitment(ownershipCommitment, blockCommitment): AnchoredOwnershipCommitment`

Creates an anchored ownership commitment combining ownership and blockchain state.

**Parameters:**
- `ownershipCommitment` (OwnershipCommitment): The ownership commitment
- `blockCommitment` (BlockCommitment): Blockchain commitment

**Returns:** `AnchoredOwnershipCommitment` object

## Data Structures

### `PhysicalAccessCommitment`
- `blockHeight` (number): Blockchain block height
- `previousCommitment` (Buffer): Previous commitment in chain (32 bytes)
- `blockHash` (Buffer): Current block hash (32 bytes)
- `selectedChunks` (number[]): Indices of selected chunks
- `chunkHashes` (Buffer[]): SHA256 hashes of selected chunks
- `commitmentHash` (Buffer): SHA256 of all fields (32 bytes)

### `ProofWindow`
- `commitments` (PhysicalAccessCommitment[]): Last 8 commitments
- `merkleProofs` (Buffer[]): Merkle proofs for selected chunks
- `startCommitment` (Buffer): Commitment from 8 blocks ago
- `endCommitment` (Buffer): Latest commitment

### `ChunkSelectionResult`
- `selectedIndices` (number[]): Selected chunk indices
- `algorithmVersion` (number): Algorithm version used
- `totalChunks` (number): Total chunks in file
- `blockHash` (Buffer): Block hash used for selection
- `verificationHash` (Buffer): Hash for consensus validation

## TypeScript Usage

```typescript
import { 
  HashChain, 
  selectChunksV1,
  verifyChunkSelection,
  verifyProof,
  PhysicalAccessCommitment,
  ProofWindow,
  ChunkSelectionResult
} from '@dignetwork/proof-of-space-continuity'

async function mineWithTypes(): Promise<void> {
  const publicKey = Buffer.from('your_32_byte_public_key_here...', 'hex')
  const blockHeight = 12345
  const blockHash = Buffer.from('blockchain_block_hash_32_bytes...', 'hex')
  
  // Create HashChain with proper typing
  const hashchain: HashChain = new HashChain(publicKey, blockHeight, blockHash)
  
  // Stream data
  const data = Buffer.from('your data here')
  hashchain.streamData(data, './output')
  
  // Add blocks with typed returns
  const newBlockHash = Buffer.from('next_block_hash...', 'hex')
  const commitment: PhysicalAccessCommitment = hashchain.addBlock(newBlockHash)
  
  // Type-safe chunk selection
  const result: ChunkSelectionResult = selectChunksV1(blockHash, 100)
  const isValid: boolean = verifyChunkSelection(
    blockHash, 
    100, 
    result.selectedIndices
  )
  
  console.log('Chunk selection valid:', isValid)
  console.log('Selected chunks:', result.selectedIndices)
}
```

## Network Consensus

This library implements **consensus-critical algorithms** that must be identical across all network participants.

### Algorithm Standardization

- **Chunk Selection V1**: Deterministic SHA256-based selection with retry logic
- **Proof Window**: Exactly 8 blocks (2 minutes at 16-second intervals)
- **Chunk Size**: 4KB (4096 bytes) fixed size
- **Chunks Per Block**: 4 chunks selected per block
- **File Format**: Network-standard binary format with big-endian byte order

### Security Guarantees

1. **Deterministic Selection**: Same inputs always produce same chunk selections
2. **Tamper Detection**: Cryptographic hashes prevent data modification
3. **Continuous Proof**: Gaps in chain require full recomputation
4. **Network Compatibility**: Consensus compliance across all validators
5. **Storage Requirement**: Must maintain full dataset for unpredictable access

### Consensus Constants

```javascript
// These constants are part of network consensus and cannot be changed
const PROOF_WINDOW_BLOCKS = 8        // 8 blocks (2 minutes)
const CHUNK_SIZE_BYTES = 4096        // 4KB chunks
const CHUNKS_PER_BLOCK = 4           // 4 chunks per block
const CHUNK_SELECTION_VERSION = 1    // Algorithm version
const HASHCHAIN_FORMAT_VERSION = 1   // File format version
```

## File Format

### HashChain Files

- **`.data` files**: Raw data chunked into 4KB segments with padding
- **`.hashchain` files**: Metadata with Merkle tree and commitment chain
- **Naming**: Files named by SHA256 hash of data content
- **Format**: Network-consensus binary format (184-byte header)

### File Structure

```
{sha256_hash}.data     - Raw data file (4KB chunks)
{sha256_hash}.hashchain - Metadata file:
  ├── Header (184 bytes)
  ├── Merkle Tree Section
  ├── Hash Chain Section  
  └── Footer (40 bytes)
```

## Performance Characteristics

- **Memory Usage**: ~1KB per HashChain instance
- **Block Addition**: <100ms per block (typical)
- **Proof Generation**: <500ms for 8-block window
- **Concurrent Instances**: 100+ supported simultaneously
- **File Operations**: Optimized for SSD storage

## Use Cases

1. **Decentralized Storage Networks**: Prove continuous data availability
2. **Blockchain Integration**: Timestamped storage proofs on Chia blockchain
3. **Data Integrity Verification**: Cryptographic proof of data possession
4. **Storage Provider Validation**: Verify storage providers maintain data
5. **Incentive Mechanisms**: Reward continuous storage over time

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (16 or later)
- [npm](https://www.npmjs.com/)

### Setup

```bash
# Clone and install dependencies
git clone <repository-url>
cd proof-of-storage-continuity
npm install

# Build the native module
npm run build

# Run tests
npm test
```

### Project Structure

```
proof-of-storage-continuity/
├── src/
│   └── lib.rs              # Rust implementation with NAPI bindings
├── __test__/
│   └── index.spec.mjs      # Test suite (40 comprehensive tests)
├── docs/
│   └── hashchain.md        # Technical specification
├── npm/                    # Platform-specific native binaries
├── .github/workflows/      # CI/CD pipeline
├── Cargo.toml              # Rust configuration
├── package.json            # Node.js configuration
└── index.d.ts              # TypeScript definitions
```

## Testing

The project includes 40 comprehensive tests covering:

```bash
npm test  # Run all tests

# Test categories:
# - Constructor validation (3 tests)
# - Data streaming (4 tests) 
# - Chunk selection algorithm (5 tests)
# - Commitment creation (3 tests)
# - Chain management (4 tests)
# - File I/O operations (4 tests)
# - Integration workflows (3 tests)
# - Proof window generation (2 tests)
# - Production verification (3 tests)
# - Edge cases & error handling (4 tests)
# - Stress testing (5 tests)
```

## License

MIT

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes and add tests
4. Ensure all tests pass (`npm test`)
5. Commit your changes (`git commit -m 'Add some amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## Specification

For detailed technical specifications, see [hashchain.md](docs/hashchain.md).
