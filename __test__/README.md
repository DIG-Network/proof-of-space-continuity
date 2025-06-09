# HashChain Test Suite Organization

This directory contains a comprehensive test suite for the HashChain Proof of Storage Continuity implementation, organized into focused categories for better maintainability and targeted testing.

## 📁 Test Structure

```
__test__/
├── helpers/
│   └── test-setup.mjs              # Shared utilities and test data generation
├── unit/
│   ├── constructor.spec.mjs        # HashChain constructor validation
│   ├── chunk-selection.spec.mjs    # Chunk selection algorithm tests
│   ├── commitments.spec.mjs        # Ownership & anchored commitments
│   ├── file-io.spec.mjs           # File reading/writing operations
│   └── validation.spec.mjs        # Input validation & error handling
├── integration/
│   ├── data-streaming.spec.mjs     # Data streaming and file creation
│   ├── chain-management.spec.mjs   # Block addition & chain operations
│   ├── proof-windows.spec.mjs      # Proof generation workflows
│   └── end-to-end.spec.mjs        # Complete workflow testing
├── production/
│   ├── performance.spec.mjs        # Performance benchmarks
│   ├── file-persistence.spec.mjs   # File integrity across restarts
│   ├── consensus-compliance.spec.mjs # Network consensus validation
│   └── error-handling.spec.mjs     # Production error scenarios
├── edge-cases/
│   ├── boundary-conditions.spec.mjs # Edge cases and limits
│   └── stress-testing.spec.mjs     # High-load scenarios
└── README.md                       # This file
```

## 🧪 Test Categories

### **Unit Tests** (`unit/`)
Focus on individual components in isolation:
- Constructor parameter validation
- Chunk selection algorithm correctness  
- Commitment creation and validation
- File I/O operations
- Input validation and error boundaries

### **Integration Tests** (`integration/`)
Test component interactions and workflows:
- Data streaming to files with SHA256 naming
- Chain management through state transitions
- Block addition and commitment linkage
- Proof window generation and validation
- End-to-end workflow testing

### **Production Tests** (`production/`)
Validate production readiness and performance:
- Performance benchmarks with timing requirements
- File persistence across application restarts
- Network consensus compliance validation
- Production error handling and recovery
- Memory efficiency with multiple instances

### **Edge Cases** (`edge-cases/`)
Test boundary conditions and stress scenarios:
- Minimum/maximum file sizes and chunk counts
- Prime numbers and unusual data patterns
- High-load concurrent operations
- Boundary condition edge cases

## 🔧 Shared Test Utilities

### **test-setup.mjs**
Provides common functionality used across all test categories:

```javascript
// Test constants
export const TEST_PUBLIC_KEY = Buffer.from('a'.repeat(64), 'hex')
export const TEST_BLOCK_HASH = Buffer.from('b'.repeat(64), 'hex')  
export const TEST_BLOCK_HEIGHT = 100

// Test data generation
export function createTestData(size = 16384)
export function generateTestDataWithPattern(chunks, pattern)
export function generateBlockHash(seed)
export function generateVariedBlockHash(seed, salt)

// Directory management
export function createTestDir(name)
export function cleanupTestDir(dir)
export function createTempDir()

// Setup/teardown
export function setupTest(t)
export function teardownTest(t)

// Validation helpers
export function verifyFileNaming(filePaths, data)
export function validateChainState(hashchain)
```

## 🏃 Running Tests

### **Run All Tests**
```bash
npm test
```

### **Run Specific Categories**
```bash
# Unit tests only
npm test __test__/unit/

# Integration tests only  
npm test __test__/integration/

# Production tests only
npm test __test__/production/

# Edge case tests only
npm test __test__/edge-cases/
```

### **Run Individual Test Files**
```bash
# Specific functionality
npm test __test__/unit/chunk-selection.spec.mjs
npm test __test__/integration/data-streaming.spec.mjs
npm test __test__/production/performance.spec.mjs
```

### **Run Tests with Coverage**
```bash
npm run test:coverage
```

## 📊 Test Performance Expectations

### **Unit Tests**
- **Speed**: < 50ms per test
- **Focus**: Algorithm correctness and validation
- **Coverage**: Individual component behavior

### **Integration Tests**  
- **Speed**: < 500ms per test
- **Focus**: Component interactions and workflows
- **Coverage**: Data flow and state management

### **Production Tests**
- **Speed**: < 5000ms per test  
- **Focus**: Performance benchmarks and real-world scenarios
- **Coverage**: Production readiness validation

### **Edge Cases**
- **Speed**: < 1000ms per test
- **Focus**: Boundary conditions and stress scenarios
- **Coverage**: Robustness and error handling

## ✅ Test Quality Standards

### **All Tests Must**
- Use descriptive test names explaining what is being tested
- Include proper setup and cleanup (use setupTest/teardownTest)
- Validate both success and failure scenarios
- Use deterministic test data for reproducible results
- Clean up any created files/directories

### **Production Tests Must**
- Include performance timing assertions
- Test with realistic data sizes
- Validate memory efficiency  
- Include comprehensive logging for performance analysis
- Test concurrent operation scenarios

### **Integration Tests Must**
- Test complete workflows end-to-end
- Validate state transitions between components
- Test file persistence and loading
- Verify consensus compliance throughout operations

## 🔍 Debugging Tests

### **Test Data Location**
Tests create temporary directories under:
```
/tmp/hashchain-test-{timestamp}-{random}/
```

### **Test Logging**
Enable detailed logging during tests:
```bash
DEBUG=1 npm test
```

### **Performance Profiling**
Run production tests with detailed timing:
```bash
npm test __test__/production/performance.spec.mjs -- --verbose
```

## 🚀 Adding New Tests

### **1. Choose the Right Category**
- **Unit**: Testing individual functions/methods
- **Integration**: Testing component interactions  
- **Production**: Testing performance/production scenarios
- **Edge Cases**: Testing boundary conditions

### **2. Use Test Template**
```javascript
import test from 'ava'
import { HashChain } from '../../index.js'
import { 
  TEST_PUBLIC_KEY,
  setupTest,
  teardownTest,
  createTestDir,
  cleanupTestDir
} from '../helpers/test-setup.mjs'

test.beforeEach(setupTest)
test.afterEach(teardownTest)

test('descriptive test name explaining what is tested', (t) => {
  const testDir = createTestDir('test_specific_name')
  
  try {
    // Test implementation
    // Use assertions to validate behavior
    t.truthy(result)
    t.is(actual, expected)
    
  } finally {
    cleanupTestDir(testDir)
  }
})
```

### **3. Follow Naming Conventions**
- File names: `feature-name.spec.mjs`
- Test names: `component method does expected behavior`
- Test directories: `feature_name_test`

## 📈 Test Metrics

The test suite validates:
- **Algorithm Correctness**: Consensus compliance across all operations
- **Performance**: Meeting 16-second block interval requirements  
- **Memory Efficiency**: < 1KB per HashChain instance
- **File Operations**: Efficient streaming and persistence
- **Error Handling**: Graceful failure and recovery
- **Concurrent Operations**: Multiple instances operating independently

## 🎯 Coverage Goals

- **Unit Tests**: 100% function coverage
- **Integration Tests**: 100% workflow coverage  
- **Production Tests**: 100% performance requirement coverage
- **Edge Cases**: 100% boundary condition coverage

Total test count target: **60+ comprehensive tests** covering all aspects of the HashChain implementation. 