/**
 * Test Setup - Loaded before all AVA tests
 * Configures the testing environment for the HashChain Proof of Storage system
 */

const path = require('path');
const fs = require('fs');

// Load the native module
let nativeModule;
try {
    nativeModule = require('../index.js');
} catch (error) {
    console.error('Failed to load native module:', error);
    process.exit(1);
}

// Export for use in tests
global.nativeModule = nativeModule;

// Test constants based on specification
global.TEST_CONSTANTS = {
    // Chia blockchain constants
    CHIA_BLOCK_TIME_SECONDS: 52,
    BLOCKS_PER_SUB_SLOT: 64,
    SUB_SLOT_TIME: 600,
    
    // Processing constants
    CHUNKS_PER_BLOCK: 16,
    MIN_FILE_SIZE: 100 * 1024 * 1024, // 100MB
    MEMORY_HARD_VDF_MEMORY: 256 * 1024 * 1024, // 256MB
    MEMORY_HARD_ITERATIONS: 15_000_000,
    
    // DIG token economics (in mojos)
    DIG_CHECKPOINT_BOND: 1000 * Math.pow(10, 12),
    DIG_AVAILABILITY_REWARD: 1 * Math.pow(10, 12),
    DIG_CHAIN_REGISTRATION: 100 * Math.pow(10, 12),
    DIG_SLASHING_PENALTY: 1000 * Math.pow(10, 12),
    
    // Timing requirements
    MIN_BLOCKS_BETWEEN_CHECKPOINTS: 69,
    MAX_BLOCKS_BETWEEN_CHECKPOINTS: 276,
    AVAILABILITY_CHALLENGES_PER_BLOCK: 10,
    AVAILABILITY_RESPONSE_TIME: 0.5, // 500ms
    
    // Security constants
    HASH_SIZE: 32,
    PROOF_WINDOW_BLOCKS: 8,
    CHUNK_SELECTION_VERSION: 2,
    CHUNK_SIZE_BYTES: 1024 * 1024, // 1MB
    
    // Attack test parameters
    ATTACK_ITERATIONS: 1000,
    PERFORMANCE_TEST_TIMEOUT: 10000, // 10 seconds
    MEMORY_ATTACK_SIZE: 1024 * 1024 * 1024, // 1GB
};

// Test utilities
global.TestUtils = {
    /**
     * Generate test buffer of specified size
     */
    generateBuffer: (size, pattern = 0x42) => {
        const buffer = Buffer.alloc(size);
        buffer.fill(pattern);
        return buffer;
    },

    /**
     * Generate random buffer
     */
    randomBuffer: (size) => {
        return Buffer.from(Array.from({ length: size }, () => Math.floor(Math.random() * 256)));
    },

    /**
     * Generate test entropy for deterministic testing
     */
    generateTestEntropy: (seed = 12345) => {
        const random = (seed) => {
            let x = Math.sin(seed) * 10000;
            return x - Math.floor(x);
        };
        
        return {
            blockchainEntropy: Buffer.from(Array.from({ length: 32 }, (_, i) => Math.floor(random(seed + i) * 256))),
            beaconEntropy: Buffer.from(Array.from({ length: 32 }, (_, i) => Math.floor(random(seed + i + 100) * 256))),
            localEntropy: Buffer.from(Array.from({ length: 32 }, (_, i) => Math.floor(random(seed + i + 200) * 256))),
            timestamp: Date.now(),
            combinedHash: Buffer.from(Array.from({ length: 32 }, (_, i) => Math.floor(random(seed + i + 300) * 256)))
        };
    },

    /**
     * Create test prover key
     */
    generateProverKey: (id = 1) => {
        return Buffer.from(Array.from({ length: 32 }, (_, i) => (id + i) % 256));
    },

    /**
     * Measure execution time
     */
    timeExecution: async (fn) => {
        const start = process.hrtime.bigint();
        const result = await fn();
        const end = process.hrtime.bigint();
        return {
            result,
            timeMs: Number(end - start) / 1000000
        };
    },

    /**
     * Create large test file for performance tests
     */
    createLargeTestFile: (sizeBytes) => {
        const chunks = Math.ceil(sizeBytes / (64 * 1024)); // 64KB chunks
        const buffers = [];
        
        for (let i = 0; i < chunks; i++) {
            const chunkSize = Math.min(64 * 1024, sizeBytes - (i * 64 * 1024));
            buffers.push(TestUtils.generateBuffer(chunkSize, i % 256));
        }
        
        return Buffer.concat(buffers);
    },

    /**
     * Simulate network delay
     */
    simulateNetworkDelay: (ms) => {
        return new Promise(resolve => setTimeout(resolve, ms));
    },

    /**
     * Generate attack scenarios from extended-proofs.md
     */
    generateAttackScenarios: () => {
        return {
            // Hardware attacks
            asicOptimization: {
                speedupFactor: 1000,
                description: "ASIC acceleration for VDF computation"
            },
            
            fastMemory: {
                accessSpeedup: 500,
                description: "Ultra-fast memory arrays for chunk access"
            },
            
            // Storage attacks  
            partialStorage: {
                storageReduction: 0.1, // Store only 90%
                reconstructionDelay: 50, // 50ms to reconstruct
                description: "Partial storage with erasure coding"
            },
            
            deduplication: {
                sharedFiles: 100,
                costReduction: 0.99,
                description: "Multiple provers sharing same files"
            },
            
            // Protocol attacks
            chainSplit: {
                branchCount: 2,
                description: "Maintain multiple chains from same checkpoint"
            },
            
            checkpointSpam: {
                invalidRate: 0.8,
                description: "Spam L1 with invalid checkpoints"
            },
            
            // Economic attacks
            selectiveAvailability: {
                serveRate: 0.0, // Store but don't serve
                description: "Store data but refuse to serve"
            },
            
            outsourcing: {
                centralizationFactor: 10,
                description: "Outsource to fast retrieval service"
            }
        };
    }
};

// Performance monitoring
global.PerformanceMonitor = {
    measurements: new Map(),
    
    start: (label) => {
        global.PerformanceMonitor.measurements.set(label, process.hrtime.bigint());
    },
    
    end: (label) => {
        const start = global.PerformanceMonitor.measurements.get(label);
        if (start) {
            const end = process.hrtime.bigint();
            const durationMs = Number(end - start) / 1000000;
            global.PerformanceMonitor.measurements.delete(label);
            return durationMs;
        }
        return 0;
    },
    
    measure: async (label, fn) => {
        global.PerformanceMonitor.start(label);
        const result = await fn();
        const duration = global.PerformanceMonitor.end(label);
        return { result, duration };
    }
};

// Console setup for test output
if (process.env.NODE_ENV !== 'test') {
    console.log('🧪 HashChain Test Suite Setup Complete');
    console.log(`📊 Native module loaded: ${!!nativeModule}`);
    console.log(`⚡ Test constants configured: ${Object.keys(global.TEST_CONSTANTS).length} items`);
    console.log(`🛠️  Test utilities available: ${Object.keys(global.TestUtils).length} functions`);
} 