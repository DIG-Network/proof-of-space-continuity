# Proof of Work

A boilerplate Rust project with NAPI bindings for Node.js, providing simple "Hello World" functionality.

## Features

- **Proof of Work Algorithm**: High-performance mining similar to dig_plot implementation
- **Synchronous & Asynchronous Mining**: Blocking and non-blocking proof of work computation
- **Cancellable Mining**: Start mining with ability to cancel anytime
- **Progress Tracking**: Real-time progress monitoring with attempt counts and rates
- **Background Processing**: Async mining won't block the Node.js event loop
- **Nonce Verification**: Verify that a nonce meets difficulty requirements
- **Difficulty Analysis**: Count leading zero hex digits in hashes
- **Cross-platform Support**: Builds for Windows, macOS, and Linux
- **Multiple Architectures**: Supports x64 and ARM64 architectures
- **Automated CI/CD**: GitHub Actions workflow for building and testing

## Installation

```bash
npm install @dignetwork/proof-of-work
# or
yarn add @dignetwork/proof-of-work
```

## Usage

### JavaScript/Node.js

```javascript
const { 
  computeProofOfWork, 
  computeProofOfWorkAsync, 
  verifyProofOfWork,
  countDifficulty 
} = require('@dignetwork/proof-of-work')

// Synchronous proof of work (blocks until found)
const entropySeed = Buffer.from('my_plot_entropy_seed', 'utf-8')
const difficulty = 3 // 3 leading zero hex digits

const result = computeProofOfWork(entropySeed, difficulty, 1000000)
console.log('Found nonce:', result.nonce)
console.log('Hash:', result.hash)
console.log('Attempts:', result.attempts)
console.log('Time:', result.time_ms, 'ms')

// Asynchronous proof of work (non-blocking)
async function mineAsync() {
  const result = await computeProofOfWorkAsync(entropySeed, difficulty)
  console.log('Async mining completed:', result)
}

// Verify a nonce
const isValid = verifyProofOfWork(entropySeed, Number(result.nonce), difficulty)
console.log('Nonce is valid:', isValid)

// Count difficulty of a hash
const hash = Buffer.from(result.hash, 'hex')
const achievedDifficulty = countDifficulty(hash)
console.log('Achieved difficulty:', achievedDifficulty)

// Cancellable proof of work with progress tracking
const handle = startProofOfWorkCancellable(entropySeed, difficulty, 1000000)

// Poll for progress
const progressInterval = setInterval(() => {
  const progress = handle.getProgress()
  console.log(`Progress: ${progress.attempts} attempts`)
  
  if (handle.isCompleted()) {
    console.log('Mining completed!')
    clearInterval(progressInterval)
  } else if (handle.isCancelled()) {
    console.log('Mining was cancelled')
    clearInterval(progressInterval)
  }
}, 1000)

// Cancel after 5 seconds if not completed
setTimeout(() => {
  if (!handle.isCompleted()) {
    console.log('Cancelling mining...')
    handle.cancel()
  }
}, 5000)
```

### TypeScript

```typescript
import { 
  computeProofOfWork, 
  computeProofOfWorkAsync, 
  verifyProofOfWork,
  countDifficulty,
  ProofOfWorkResult 
} from '@dignetwork/proof-of-work'

// Synchronous proof of work
const entropySeed = Buffer.from('my_plot_entropy_seed', 'utf-8')
const difficulty = 12

const result: ProofOfWorkResult = computeProofOfWork(entropySeed, difficulty)
console.log('Mining result:', result)

// Asynchronous proof of work with proper typing
async function mineWithTypes(): Promise<void> {
  const result: ProofOfWorkResult = await computeProofOfWorkAsync(
    entropySeed, 
    difficulty, 
    1000000 // max attempts
  )
  
  const isValid: boolean = verifyProofOfWork(
    entropySeed, 
    Number(result.nonce), 
    difficulty
  )
  
  console.log('Mining completed, valid:', isValid)
}
```

## API Reference

### Proof of Work Functions

### `computeProofOfWork(entropySeed: Buffer, difficulty: number, maxAttempts?: number): ProofOfWorkResult`

Computes proof of work synchronously by finding a nonce that satisfies the difficulty requirement. **Warning:** This function blocks until a solution is found and may take a long time for high difficulties.

**Parameters:**
- `entropySeed` (Buffer): The entropy seed (plotId) to bind the work to
- `difficulty` (number): The difficulty level (number of leading zero hex digits required)
- `maxAttempts` (number, optional): Maximum number of attempts before giving up (default: 1,000,000)

**Returns:** `ProofOfWorkResult` object containing:
- `nonce` (bigint): The nonce that was found
- `hash` (string): The resulting hash as hex string
- `attempts` (bigint): Number of attempts made
- `time_ms` (number): Time taken in milliseconds
- `difficulty` (number): The difficulty achieved

### `computeProofOfWorkAsync(entropySeed: Buffer, difficulty: number, maxAttempts?: number): Promise<ProofOfWorkResult>`

Computes proof of work asynchronously in the background. This function returns a Promise and won't block the Node.js event loop.

**Parameters:** Same as `computeProofOfWork`

**Returns:** Promise that resolves with `ProofOfWorkResult`

### `verifyProofOfWork(entropySeed: Buffer, nonce: number, difficulty: number): boolean`

Verifies that a nonce produces a hash that meets the difficulty requirement.

**Parameters:**
- `entropySeed` (Buffer): The entropy seed that was used
- `nonce` (number): The nonce to verify
- `difficulty` (number): The required difficulty level

**Returns:** `true` if the nonce is valid for the given difficulty

### `countDifficulty(hash: Buffer): number`

Count the number of leading zero hex digits in a hash (for difficulty measurement).

**Parameters:**
- `hash` (Buffer): The hash to analyze

**Returns:** Number of leading zero hex digits

### `startProofOfWorkCancellable(entropySeed: Buffer, difficulty: number, maxAttempts?: number): ProofOfWorkHandle`

Starts a cancellable proof of work computation that runs in the background. Returns a handle for controlling and monitoring the operation.

**Parameters:**
- `entropySeed` (Buffer): The entropy seed (plotId) to bind the work to
- `difficulty` (number): The difficulty level (number of leading zero hex digits required)
- `maxAttempts` (number, optional): Maximum number of attempts before giving up (default: 1,000,000)

**Returns:** `ProofOfWorkHandle` object with methods:
- `cancel()`: Cancel the computation
- `isCancelled()`: Check if cancelled
- `isCompleted()`: Check if solution was found
- `getAttempts()`: Get current attempt count
- `getProgress()`: Get detailed progress information

### ProofOfWork Handle Methods

#### `handle.cancel(): void`

Cancels the running proof of work computation.

#### `handle.isCancelled(): boolean`

Returns `true` if the computation has been cancelled.

#### `handle.isCompleted(): boolean`

Returns `true` if the computation has found a valid solution.

#### `handle.getAttempts(): bigint`

Returns the current number of attempts made (approximate).

#### `handle.getProgress(): ProofOfWorkProgress`

Returns detailed progress information:
- `attempts` (bigint): Current number of attempts
- `nonce` (bigint): Current nonce being tested
- `elapsed_ms` (number): Time elapsed in milliseconds
- `attempts_per_second` (number): Estimated mining rate

### Utility Functions

### `helloWorld(): string`

Returns a simple "Hello World!" greeting.

**Returns:** A string containing "Hello World!"

### `greet(name: string): string`

Returns a personalized greeting message.

**Parameters:**
- `name` (string): The name to include in the greeting

**Returns:** A string containing "Hello, {name}!"

### `add(a: number, b: number): number`

Adds two numbers together.

**Parameters:**
- `a` (number): First number
- `b` (number): Second number

**Returns:** The sum of a and b

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (16 or later)
- [Yarn](https://yarnpkg.com/) (v4.3.1)

### Setup

1. Clone the repository
2. Install dependencies:
   ```bash
   yarn install
   ```

### Building

```bash
# Development build
yarn build:debug

# Production build
yarn build
```

### Testing

```bash
yarn test
```

### Project Structure

```
proof-of-work/
├── src/
│   └── lib.rs          # Rust source code with NAPI bindings
├── __test__/
│   └── index.spec.mjs  # Test files
├── .github/
│   └── workflows/
│       └── CI.yml      # GitHub Actions CI/CD pipeline
├── Cargo.toml          # Rust project configuration
├── package.json        # Node.js project configuration
├── build.rs            # Build script
├── index.js            # Main entry point
├── index.d.ts          # TypeScript definitions
└── README.md           # This file
```

## CI/CD

This project uses GitHub Actions for continuous integration and deployment:

- **Rust Checks**: Runs clippy, unused dependency checks, and formatting
- **Cross-platform Builds**: Builds for multiple platforms and architectures
- **Testing**: Runs tests on different Node.js versions and platforms
- **Publishing**: Automatically publishes to npm on version tags

### Supported Platforms

- **Windows**: x64, x86, ARM64
- **macOS**: x64, ARM64 (Apple Silicon)
- **Linux**: x64, ARM64

## License

MIT

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request# proof-of-work
