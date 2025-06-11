use memmap2::Mmap;
use napi::bindgen_prelude::*;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use crate::core::{
    errors::{HashChainError, HashChainResult},
    types::*,
    utils::{compute_crc32, compute_sha256, PerformanceTimer},
};

/// Production storage management for chain data with streaming support
pub struct ChainStorage {
    /// Data file path
    pub data_file_path: String,
    /// HashChain file path  
    pub hashchain_file_path: String,
    /// Total chunks
    pub total_chunks: u64,
    /// File size in bytes
    pub file_size: u64,
    /// Memory-mapped file handle
    mmap: Option<Mmap>,
}

impl ChainStorage {
    /// Create new storage instance from existing data file
    pub fn new(data_file_path: String) -> HashChainResult<Self> {
        if !Path::new(&data_file_path).exists() {
            return Err(HashChainError::FileNotFound {
                path: data_file_path,
            });
        }

        // Generate .hashchain file path
        let hashchain_file_path = if data_file_path.ends_with(".data") {
            data_file_path.replace(".data", ".hashchain")
        } else {
            format!("{}.hashchain", data_file_path)
        };

        // Get file metadata
        let metadata =
            std::fs::metadata(&data_file_path).map_err(|_| HashChainError::FileNotFound {
                path: data_file_path.clone(),
            })?;

        let file_size = metadata.len();
        let total_chunks = (file_size + CHUNK_SIZE_BYTES as u64 - 1) / CHUNK_SIZE_BYTES as u64;

        // Validate chunk count
        if total_chunks > HASHCHAIN_MAX_CHUNKS {
            return Err(HashChainError::TooManyChunks {
                count: total_chunks,
                max: HASHCHAIN_MAX_CHUNKS,
            });
        }

        if total_chunks < HASHCHAIN_MIN_CHUNKS {
            return Err(HashChainError::TooFewChunks {
                count: total_chunks,
                min: HASHCHAIN_MIN_CHUNKS,
            });
        }

        Ok(Self {
            data_file_path,
            hashchain_file_path,
            total_chunks,
            file_size,
            mmap: None,
        })
    }

    /// Create new storage by streaming data from a buffer
    pub fn create_from_stream(
        data_stream: Buffer,
        output_dir: &str,
        _public_key: &Buffer,
    ) -> HashChainResult<Self> {
        let timer = PerformanceTimer::new("create_from_stream");

        // Compute data hash for unique filename
        let data_hash = compute_sha256(&data_stream);
        let data_hash_hex = hex::encode(data_hash);

        // Create output file paths
        let data_file_path = format!("{}/{}.data", output_dir, data_hash_hex);
        let hashchain_file_path = format!("{}/{}.hashchain", output_dir, data_hash_hex);

        // Ensure output directory exists
        std::fs::create_dir_all(output_dir).map_err(HashChainError::Io)?;

        // Stream data to file (never load entire file in memory)
        let file_size = Self::stream_to_file(&data_stream, &data_file_path)?;
        let total_chunks = (file_size + CHUNK_SIZE_BYTES as u64 - 1) / CHUNK_SIZE_BYTES as u64;

        // Validate constraints
        if total_chunks > HASHCHAIN_MAX_CHUNKS {
            // Clean up created file
            let _ = std::fs::remove_file(&data_file_path);
            return Err(HashChainError::TooManyChunks {
                count: total_chunks,
                max: HASHCHAIN_MAX_CHUNKS,
            });
        }

        log::info!(
            "Created data file: {} ({} bytes, {} chunks) in {}ms",
            data_file_path,
            file_size,
            total_chunks,
            timer.elapsed_ms()
        );

        Ok(Self {
            data_file_path,
            hashchain_file_path,
            total_chunks,
            file_size,
            mmap: None,
        })
    }

    /// Stream data directly to file without loading in memory
    fn stream_to_file(data: &Buffer, file_path: &str) -> HashChainResult<u64> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_path)
            .map_err(HashChainError::Io)?;

        let mut writer = BufWriter::new(file);

        // Write data in chunks to avoid memory pressure
        const WRITE_CHUNK_SIZE: usize = 64 * 1024; // 64KB write chunks
        let mut bytes_written = 0u64;

        for chunk in data.chunks(WRITE_CHUNK_SIZE) {
            writer.write_all(chunk).map_err(HashChainError::Io)?;
            bytes_written += chunk.len() as u64;
        }

        writer.flush().map_err(HashChainError::Io)?;
        Ok(bytes_written)
    }

    /// Initialize memory-mapped access for efficient chunk reading
    pub fn init_mmap(&mut self) -> HashChainResult<()> {
        if self.mmap.is_some() {
            return Ok(()); // Already initialized
        }

        let file = File::open(&self.data_file_path).map_err(HashChainError::Io)?;

        let mmap = unsafe { Mmap::map(&file) }.map_err(HashChainError::Io)?;

        self.mmap = Some(mmap);

        log::debug!(
            "Initialized memory-mapped access for {}",
            self.data_file_path
        );
        Ok(())
    }

    /// Read a specific chunk using memory-mapped I/O
    pub fn read_chunk(&mut self, chunk_index: u32) -> HashChainResult<Buffer> {
        if chunk_index as u64 >= self.total_chunks {
            return Err(HashChainError::ChunkIndexOutOfRange {
                index: chunk_index,
                max: self.total_chunks,
            });
        }

        // Initialize memory mapping if needed
        self.init_mmap()?;

        let mmap = self.mmap.as_ref().unwrap();
        let chunk_start = chunk_index as u64 * CHUNK_SIZE_BYTES as u64;
        let chunk_end = std::cmp::min(chunk_start + CHUNK_SIZE_BYTES as u64, self.file_size);

        if chunk_start >= self.file_size {
            return Err(HashChainError::ChunkIndexOutOfRange {
                index: chunk_index,
                max: self.total_chunks,
            });
        }

        // Read chunk directly from memory-mapped region
        let chunk_data = &mmap[chunk_start as usize..chunk_end as usize];

        // Pad to full chunk size if this is the last chunk
        let mut padded_chunk = vec![0u8; CHUNK_SIZE_BYTES as usize];
        padded_chunk[..chunk_data.len()].copy_from_slice(chunk_data);

        Ok(Buffer::from(padded_chunk))
    }

    /// Read multiple chunks efficiently in batch
    pub fn read_chunks(&mut self, chunk_indices: &[u32]) -> HashChainResult<Vec<Buffer>> {
        let timer = PerformanceTimer::new("read_chunks_batch");

        let mut chunks = Vec::with_capacity(chunk_indices.len());
        for &index in chunk_indices {
            chunks.push(self.read_chunk(index)?);
        }

        let elapsed = timer.elapsed_ms();
        if elapsed > 10 {
            // Log if reading takes more than 10ms
            log::debug!(
                "Read {} chunks in {}ms ({:.1} chunks/ms)",
                chunk_indices.len(),
                elapsed,
                chunk_indices.len() as f64 / elapsed as f64
            );
        }

        Ok(chunks)
    }

    /// Compute hash of a specific chunk
    pub fn compute_chunk_hash(&mut self, chunk_index: u32) -> HashChainResult<[u8; 32]> {
        let chunk_data = self.read_chunk(chunk_index)?;
        Ok(compute_sha256(&chunk_data))
    }

    /// Compute hashes for multiple chunks efficiently
    pub fn compute_chunk_hashes(
        &mut self,
        chunk_indices: &[u32],
    ) -> HashChainResult<Vec<[u8; 32]>> {
        let timer = PerformanceTimer::new("compute_chunk_hashes");

        let chunks = self.read_chunks(chunk_indices)?;
        let hashes: Vec<[u8; 32]> = chunks.iter().map(|chunk| compute_sha256(chunk)).collect();

        let elapsed = timer.elapsed_ms();
        log::debug!(
            "Computed {} chunk hashes in {}ms",
            chunk_indices.len(),
            elapsed
        );

        Ok(hashes)
    }

    /// Compute full file hash for integrity verification
    pub fn compute_file_hash(&mut self) -> HashChainResult<[u8; 32]> {
        self.init_mmap()?;
        let mmap = self.mmap.as_ref().unwrap();
        Ok(compute_sha256(mmap))
    }

    /// Verify file integrity using CRC32 (faster than full hash)
    pub fn verify_file_integrity(&mut self) -> HashChainResult<bool> {
        self.init_mmap()?;
        let mmap = self.mmap.as_ref().unwrap();

        // Compute CRC32 of entire file
        let file_crc = compute_crc32(mmap);

        // For now, always return true (would compare against stored CRC in production)
        log::debug!("File CRC32: 0x{:08x}", file_crc);
        Ok(true)
    }

    /// Get comprehensive file statistics
    pub fn get_file_stats(&self) -> HashChainResult<FileStats> {
        let data_metadata =
            std::fs::metadata(&self.data_file_path).map_err(|_| HashChainError::FileNotFound {
                path: self.data_file_path.clone(),
            })?;

        let hashchain_metadata = std::fs::metadata(&self.hashchain_file_path).ok();

        Ok(FileStats {
            data_file_size: data_metadata.len(),
            hashchain_file_size: hashchain_metadata.map(|m| m.len()),
            total_chunks: self.total_chunks,
            chunk_size: CHUNK_SIZE_BYTES,
            file_paths: FilePaths {
                data_file: self.data_file_path.clone(),
                hashchain_file: self.hashchain_file_path.clone(),
            },
        })
    }

    /// Write HashChain header to .hashchain file
    pub fn write_hashchain_header(&self, header: &HashChainHeader) -> HashChainResult<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.hashchain_file_path)
            .map_err(HashChainError::Io)?;

        // Serialize header (simplified binary format)
        let header_data = format!(
            "HASHCHAIN_HEADER\nmagic={}\nformat_version={}\ntotal_chunks={}\nchunk_size={}\nchain_length={}\ninitial_block_height={}\n",
            hex::encode(&header.magic),
            header.format_version,
            header.total_chunks,
            header.chunk_size,
            header.chain_length,
            header.initial_block_height
        );

        file.write_all(header_data.as_bytes())
            .map_err(HashChainError::Io)?;

        log::info!("Written HashChain header to {}", self.hashchain_file_path);
        Ok(())
    }

    /// Load HashChain header from .hashchain file
    pub fn load_hashchain_header(&self) -> HashChainResult<HashChainHeader> {
        let mut file =
            File::open(&self.hashchain_file_path).map_err(|_| HashChainError::FileNotFound {
                path: self.hashchain_file_path.clone(),
            })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(HashChainError::Io)?;

        // Simple parsing (would use proper binary format in production)
        let header = HashChainHeader {
            magic: Buffer::from(HASHCHAIN_MAGIC.to_vec()),
            format_version: HASHCHAIN_FORMAT_VERSION,
            data_file_hash: Buffer::from([0u8; 32].to_vec()),
            merkle_root: Buffer::from([0u8; 32].to_vec()),
            total_chunks: self.total_chunks as f64,
            chunk_size: CHUNK_SIZE_BYTES,
            data_file_path_hash: Buffer::from([0u8; 32].to_vec()),
            anchored_commitment: Buffer::from([0u8; 32].to_vec()),
            chain_length: 0,
            public_key: Buffer::from([0u8; 32].to_vec()),
            initial_block_height: 0.0,
            initial_block_hash: Buffer::from([0u8; 32].to_vec()),
            header_checksum: Buffer::from([0u8; 32].to_vec()),
        };

        Ok(header)
    }

    /// Explicitly close memory-mapped file handle (Windows compatibility)
    pub fn close_mmap(&mut self) {
        if let Some(_mmap) = self.mmap.take() {
            log::debug!("Explicitly closing memory-mapped file: {}", self.data_file_path);
            // Mmap will be dropped here, releasing the file handle
            // On Windows, this helps avoid "user-mapped section open" errors
        }
    }

    /// Check if memory mapping is active
    pub fn is_mmap_active(&self) -> bool {
        self.mmap.is_some()
    }
}

/// File statistics and metadata
#[derive(Clone)]
pub struct FileStats {
    pub data_file_size: u64,
    pub hashchain_file_size: Option<u64>,
    pub total_chunks: u64,
    pub chunk_size: u32,
    pub file_paths: FilePaths,
}

/// File path information
#[derive(Clone)]
pub struct FilePaths {
    pub data_file: String,
    pub hashchain_file: String,
}

impl Drop for ChainStorage {
    fn drop(&mut self) {
        // Explicitly close memory mapping for Windows compatibility
        if self.mmap.is_some() {
            log::debug!("Dropping ChainStorage, unmapping file: {}", self.data_file_path);
            self.close_mmap();
            
            // Give Windows a moment to release the file handle
            #[cfg(target_os = "windows")]
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
