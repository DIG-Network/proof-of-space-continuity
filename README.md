# Proof of Work

A high-performance Bitcoin-compatible proof of work library for Node.js, built with Rust and NAPI bindings. This library provides efficient mining capabilities using Bitcoin's target-based difficulty system with double SHA-256 hashing.

## Features

- **Bitcoin-Compatible**: Uses Bitcoin's target-based difficulty system with double SHA-256 hashing
- **High Performance**: Written in Rust for maximum mining efficiency
- **Asynchronous Mining**: Non-blocking proof of work computation that won't freeze your application
- **Cancellable Operations**: Start mining with ability to cancel anytime using handles
- **Progress Tracking**: Real-time progress monitoring with attempt counts and timing
- **Unlimited Attempts**: Mine until solution is found (unless explicitly limited)
- **Nonce Verification**: Verify that a nonce meets difficulty requirements
- **Difficulty Analysis**: Calculate difficulty levels from hash values
- **Cross-platform Support**: Builds for Windows, macOS, and Linux
- **Multiple Architectures**: Supports x64 and ARM64 architectures
- **TypeScript Support**: Full TypeScript definitions included

## Installation

```bash
npm install @dignetwork/proof-of-work
```

## Quick Start

```javascript
const { computeProofOfWorkAsync } = require('@dignetwork/proof-of-work')

async function mine() {
  const entropySeed = Buffer.from('my_plot_entropy_seed', 'utf-8')
  const difficulty = 1.0 // Bitcoin difficulty (1.0 = easiest)
  
  // Start mining (returns immediately with a handle)
  const handle = computeProofOfWorkAsync(entropySeed, difficulty)
  
  // Set up Ctrl+C handling for cancellation
  process.on('SIGINT', () => {
    console.log('\nCancelling mining...')
    handle.cancel()
    process.exit(0)
  })
  
  // Wait for completion
  while (!handle.isCompleted() && !handle.hasError()) {
    console.log(`Mining... attempts: ${handle.getAttempts()}`)
    await new Promise(resolve => setTimeout(resolve, 1000))
  }
  
  if (handle.hasError()) {
    console.log('Mining failed:', handle.getError())
  } else {
    const result = handle.getResult()
    console.log('Solution found!')
    console.log('Nonce:', result.nonce)
    console.log('Hash:', result.hash)
    console.log('Attempts:', result.attempts)
    console.log('Time:', result.time_ms, 'ms')
  }
}

mine().catch(console.error)
```

## API Reference

### Main Functions

#### `computeProofOfWorkAsync(entropySeed, difficulty, maxAttempts?, logAttempts?, doubleSha?): ProofOfWorkHandle`

Computes proof of work asynchronously using Bitcoin's target-based difficulty system. Returns immediately with a handle for cancellation and progress tracking.

**Parameters:**
- `entropySeed` (Buffer): The entropy seed (plotId) to bind the work to
- `difficulty` (number): Bitcoin-style difficulty level (1.0 = easiest, higher = harder)
- `maxAttempts` (number, optional): Maximum attempts before giving up (default: unlimited)
- `logAttempts` (boolean, optional): Whether to log each hash attempt (default: false)
- `doubleSha` (boolean, optional): Whether to use double SHA-256 like Bitcoin (default: true)

**Returns:** `ProofOfWorkHandle` for cancellation and progress tracking

#### `verifyProofOfWork(entropySeed, nonce, difficulty, doubleSha?): boolean`

Verifies that a nonce produces a hash that meets the Bitcoin difficulty target.

**Parameters:**
- `entropySeed` (Buffer): The entropy seed that was used
- `nonce` (number): The nonce to verify
- `difficulty` (number): The required difficulty level
- `doubleSha` (boolean, optional): Whether to use double SHA-256 (default: true)

**Returns:** `true` if the nonce is valid for the given difficulty

### Utility Functions

#### `difficultyToTargetHex(difficulty): string`

Convert a Bitcoin-style difficulty to the corresponding target value as hex.

**Parameters:**
- `difficulty` (number): The difficulty level

**Returns:** The target as a hex string

#### `hashToDifficulty(hash): number`

Calculate the difficulty that a given hash would satisfy.

**Parameters:**
- `hash` (Buffer): The hash to analyze (32 bytes)

**Returns:** The difficulty level this hash would satisfy

### ProofOfWorkHandle Methods

The handle returned by `computeProofOfWorkAsync` provides these methods:

#### `cancel(): void`
Cancels the running proof of work computation.

#### `isCancelled(): boolean`
Returns `true` if the computation has been cancelled.

#### `isCompleted(): boolean`
Returns `true` if the computation has found a valid solution.

#### `hasError(): boolean`
Returns `true` if there was an error (cancelled or max attempts reached).

#### `getError(): string | null`
Returns the error message if there was an error, or `null` if no error.

#### `getResult(): ProofOfWorkResult | null`
Returns the result if computation completed successfully, or `null` if not completed.

#### `getAttempts(): bigint`
Returns the current number of attempts made (approximate).

#### `getProgress(): ProofOfWorkProgress`
Returns detailed progress information (attempts, elapsed time, etc.).

#### `getDifficulty(): number`
Returns the difficulty level for this computation.

### Result Types

#### `ProofOfWorkResult`
- `nonce` (bigint): The nonce that was found
- `hash` (string): The resulting hash as hex string
- `attempts` (bigint): Number of attempts made
- `time_ms` (number): Time taken in milliseconds
- `difficulty` (number): The difficulty that was satisfied
- `target` (string): The target that was used (as hex string)

#### `ProofOfWorkProgress`
- `attempts` (bigint): Current number of attempts
- `nonce` (bigint): Current nonce being tested
- `elapsed_ms` (number): Time elapsed in milliseconds
- `attempts_per_second` (number): Estimated attempts per second

## TypeScript Usage

```typescript
import { 
  computeProofOfWorkAsync, 
  verifyProofOfWork,
  hashToDifficulty,
  ProofOfWorkResult,
  ProofOfWorkHandle 
} from '@dignetwork/proof-of-work'

async function mineWithTypes(): Promise<void> {
  const entropySeed = Buffer.from('my_plot_entropy_seed', 'utf-8')
  const difficulty = 2.0
  
  const handle: ProofOfWorkHandle = computeProofOfWorkAsync(entropySeed, difficulty)
  
  // Wait for completion with proper typing
  const waitForCompletion = async (handle: ProofOfWorkHandle): Promise<ProofOfWorkResult> => {
    while (!handle.isCompleted() && !handle.hasError()) {
      await new Promise(resolve => setTimeout(resolve, 100))
    }
    
    if (handle.hasError()) {
      throw new Error(handle.getError() || 'Unknown error')
    }
    
    const result = handle.getResult()
    if (!result) {
      throw new Error('No result available')
    }
    
    return result
  }
  
  const result: ProofOfWorkResult = await waitForCompletion(handle)
  
  const isValid: boolean = verifyProofOfWork(
    entropySeed, 
    Number(result.nonce), 
    difficulty
  )
  
  console.log('Mining completed, valid:', isValid)
}
```

## Difficulty System

This library uses Bitcoin's target-based difficulty system:

- **Difficulty 1.0**: Easiest level, requires 8 leading zero bits in hash
- **Difficulty 2.0**: Requires 12 leading zero bits
- **Difficulty 4.0**: Requires 16 leading zero bits (2 zero bytes)
- **Higher difficulties**: Exponentially more difficult

The difficulty directly corresponds to how computationally expensive it is to find a valid nonce.

## Performance Tips

1. **Use appropriate difficulty levels**: Start with 1.0-4.0 for testing
2. **Monitor progress**: Use the handle to track mining progress
3. **Set reasonable limits**: Use `maxAttempts` for time-critical applications
4. **Handle cancellation**: Always provide a way to cancel long-running operations

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (16 or later)
- [npm](https://www.npmjs.com/)

### Setup

```bash
# Clone and install dependencies
git clone <repository-url>
cd proof-of-work
npm install

# Build the native module
npm run build

# Run tests
npm test
```

### Project Structure

```
proof-of-work/
├── src/
│   └── lib.rs          # Rust implementation with NAPI bindings
├── __test__/
│   └── index.spec.mjs  # Test suite
├── npm/                # Platform-specific native binaries
├── .github/workflows/  # CI/CD pipeline
├── Cargo.toml          # Rust configuration
├── package.json        # Node.js configuration
└── index.d.ts          # TypeScript definitions
```

## CI/CD & Publishing

This project uses GitHub Actions for:
- Cross-platform builds (Windows, macOS, Linux)
- Multiple architectures (x64, ARM64)
- Automated testing
- npm publishing based on commit messages

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
