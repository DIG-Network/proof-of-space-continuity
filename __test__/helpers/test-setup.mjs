import { readFileSync, writeFileSync, mkdirSync, rmSync, existsSync } from 'fs'
import { join as pathJoin } from 'path'
import path from 'path'
import { createHash } from 'crypto'

// Test constants for deterministic testing
export const TEST_PUBLIC_KEY = Buffer.from('a'.repeat(64), 'hex') // 32 bytes
export const TEST_BLOCK_HASH = Buffer.from('b'.repeat(64), 'hex') // 32 bytes
export const TEST_BLOCK_HEIGHT = 100

/**
 * Helper function to create test data
 * @param {number} size - Size in bytes (default: 16384 = 4 chunks)
 * @returns {Buffer} Test data buffer
 */
export function createTestData(size = 16384) {
  const data = Buffer.alloc(size)
  for (let i = 0; i < size; i++) {
    data[i] = i % 256
  }
  return data
}

/**
 * Helper function to create test directory
 * @param {string} testName - Name for the test directory
 * @returns {string} Path to created directory
 */
export function createTestDir(testName) {
  const testDir = pathJoin(process.cwd(), 'test_output', testName)
  
  // Clean up existing directory with retry logic for Windows
  if (existsSync(testDir)) {
    try {
      rmSync(testDir, { recursive: true, force: true })
    } catch (error) {
      if (error.code === 'ENOTEMPTY' || error.code === 'EBUSY') {
        // On Windows, directories might still be in use
        // Try to create a unique directory instead
        const timestamp = Date.now()
        const uniqueTestDir = `${testDir}_${timestamp}`
        if (!existsSync(uniqueTestDir)) {
          mkdirSync(uniqueTestDir, { recursive: true })
          return uniqueTestDir
        }
      }
      console.warn(`Failed to cleanup directory: ${testDir}, continuing anyway`)
    }
  }
  
  mkdirSync(testDir, { recursive: true })
  return testDir
}

/**
 * Helper function to clean up test directory
 * @param {string} testDir - Directory path to clean up
 */
export function cleanupTestDir(testDir) {
  if (existsSync(testDir)) {
    try {
      rmSync(testDir, { recursive: true, force: true })
    } catch (error) {
      // On Windows, try multiple times with delay for file handle cleanup
      if (error.code === 'ENOTEMPTY' || error.code === 'EBUSY') {
        setTimeout(() => {
          try {
            rmSync(testDir, { recursive: true, force: true })
          } catch (e) {
            console.warn(`Failed to cleanup test directory: ${testDir}`, e.message)
          }
        }, 100)
      } else {
        console.warn(`Failed to cleanup test directory: ${testDir}`, error.message)
      }
    }
  }
}

/**
 * Setup function to run before each test
 */
export function setupTest() {
  // Clean up any existing test output
  const testOutputDir = pathJoin(process.cwd(), 'test_output')
  if (existsSync(testOutputDir)) {
    try {
      rmSync(testOutputDir, { recursive: true, force: true })
    } catch (error) {
      // On Windows, directories might still be in use
      console.warn(`Failed to cleanup test output directory: ${error.message}`)
    }
  }
}

/**
 * Teardown function to run after each test
 */
export function teardownTest() {
  // Clean up test output after each test
  const testOutputDir = pathJoin(process.cwd(), 'test_output')
  if (existsSync(testOutputDir)) {
    try {
      rmSync(testOutputDir, { recursive: true, force: true })
    } catch (error) {
      // On Windows, directories might still be in use
      console.warn(`Failed to cleanup test output directory: ${error.message}`)
    }
  }
}

/**
 * Create a temporary directory for async tests
 * @param {string} prefix - Prefix for the directory name
 * @returns {string} Path to temporary directory
 */
export function createTempDir(prefix) {
  return path.join(process.cwd(), 'temp', `${prefix}-${Date.now()}`)
}

/**
 * Generate deterministic block hash for testing
 * @param {number} blockNumber - Block number for deterministic generation
 * @returns {Buffer} 32-byte block hash
 */
export function generateBlockHash(blockNumber) {
  const hash = Buffer.alloc(32)
  hash.writeUInt32BE(blockNumber, 28)
  return hash
}

/**
 * Generate varied block hash with additional entropy
 * @param {number} blockNumber - Base block number
 * @param {number} variation - Additional variation factor
 * @returns {Buffer} 32-byte block hash
 */
export function generateVariedBlockHash(blockNumber, variation = 0) {
  const hash = Buffer.alloc(32)
  hash.writeUInt32BE(blockNumber, 28)
  hash[0] = variation % 256
  hash[1] = (variation * 7) % 256
  return hash
}

/**
 * Verify file naming follows SHA256 convention
 * @param {Array<string>} filePaths - Array of file paths to check
 * @param {Buffer} originalData - Original data that should match the hash
 * @returns {boolean} True if all paths contain correct hash
 */
export function verifyFileNaming(filePaths, originalData) {
  const expectedHash = createHash('sha256').update(originalData).digest('hex')
  return filePaths.every(path => path.includes(expectedHash))
}

/**
 * Generate test data with specific patterns for better validation
 * @param {number} chunks - Number of 4KB chunks to generate
 * @param {string} pattern - Pattern type: 'sequential', 'random', 'deterministic'
 * @returns {Buffer} Generated test data
 */
export function generateTestDataWithPattern(chunks, pattern = 'sequential') {
  const size = chunks * 4096
  const data = Buffer.alloc(size)
  
  switch (pattern) {
    case 'sequential':
      for (let i = 0; i < size; i++) {
        data[i] = i % 256
      }
      break
    case 'deterministic':
      for (let i = 0; i < size; i++) {
        data[i] = (i * 13 + 7) % 256
      }
      break
    case 'random':
      // Seeded random for reproducibility
      let seed = 12345
      for (let i = 0; i < size; i++) {
        seed = (seed * 9301 + 49297) % 233280
        data[i] = (seed / 233280) * 256
      }
      break
    // Handle pattern_N formats
    default:
      if (pattern.startsWith('pattern_')) {
        const patternNum = parseInt(pattern.split('_')[1]) || 0
        // Generate deterministic pattern based on number
        for (let i = 0; i < size; i++) {
          data[i] = (i * (patternNum + 1) + patternNum * 17) % 256
        }
      } else {
        console.warn(`Unknown pattern: ${pattern}, using sequential`)
        // Fall back to sequential instead of throwing
        for (let i = 0; i < size; i++) {
          data[i] = i % 256
        }
      }
      break
  }
  
  return data
} 