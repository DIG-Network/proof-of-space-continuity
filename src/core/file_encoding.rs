use napi::bindgen_prelude::*;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

use crate::core::{types::*, utils::compute_sha256};

/// File encoding system to prevent deduplication attacks
/// Each prover stores a unique version of the file by XORing with their public key
pub struct FileEncoder {
    prover_key: Buffer,
    encoding_version: u32,
}

impl FileEncoder {
    /// Create new file encoder with prover's public key
    pub fn new(prover_key: Buffer) -> Result<Self> {
        if prover_key.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "Prover key must be 32 bytes".to_string(),
            ));
        }

        Ok(FileEncoder {
            prover_key,
            encoding_version: 1,
        })
    }

    /// Encode a chunk of data with prover-specific encoding (streaming compatible)
    pub fn encode_chunk(&self, chunk_data: &[u8], chunk_index: u32) -> Result<Vec<u8>> {
        if chunk_data.is_empty() || chunk_data.len() > CHUNK_SIZE_BYTES as usize {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Chunk must be 1-{} bytes", CHUNK_SIZE_BYTES),
            ));
        }

        let mut encoded = Vec::with_capacity(chunk_data.len());

        // Generate chunk-specific encoding key
        let encoding_key = self.generate_chunk_key(chunk_index)?;

        // XOR each byte with corresponding key byte (cycle through key for any chunk size)
        for (i, &data_byte) in chunk_data.iter().enumerate() {
            let key_byte = encoding_key[i % 32];
            encoded.push(data_byte ^ key_byte);
        }

        Ok(encoded)
    }

    /// Decode a chunk of data back to original (streaming compatible)
    pub fn decode_chunk(&self, encoded_data: &[u8], chunk_index: u32) -> Result<Vec<u8>> {
        if encoded_data.is_empty() || encoded_data.len() > CHUNK_SIZE_BYTES as usize {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Encoded chunk must be 1-{} bytes", CHUNK_SIZE_BYTES),
            ));
        }

        // XOR operation is its own inverse
        self.encode_chunk(encoded_data, chunk_index)
    }

    /// Generate chunk-specific encoding key
    fn generate_chunk_key(&self, chunk_index: u32) -> Result<[u8; 32]> {
        let mut key_input = Vec::new();
        key_input.extend_from_slice(&self.prover_key);
        key_input.extend_from_slice(&chunk_index.to_be_bytes());
        key_input.extend_from_slice(&self.encoding_version.to_be_bytes());
        key_input.extend_from_slice(b"chunk_encoding_key");

        Ok(compute_sha256(&key_input))
    }

    /// Create file encoding information
    pub fn create_encoding_info(
        &self,
        original_hash: Buffer,
        encoded_hash: Buffer,
    ) -> FileEncodingInfo {
        // Generate encoding parameters
        let mut params = Vec::new();
        params.extend_from_slice(&self.encoding_version.to_be_bytes());
        params.extend_from_slice(b"xor_encoding");

        FileEncodingInfo {
            original_hash,
            encoded_hash,
            prover_key: self.prover_key.clone(),
            encoding_version: self.encoding_version,
            encoding_params: Buffer::from(params),
        }
    }

    /// Verify that encoded data belongs to this prover
    pub fn verify_encoding(&self, encoding_info: &FileEncodingInfo) -> Result<bool> {
        // Verify prover key matches
        if encoding_info.prover_key.as_ref() != self.prover_key.as_ref() {
            return Ok(false);
        }

        // Verify encoding version is supported
        if encoding_info.encoding_version != self.encoding_version {
            return Ok(false);
        }

        Ok(true)
    }
}

/// Stream encode entire file with prover-specific encoding (never loads entire file in memory)
pub fn stream_encode_file(
    input_file_path: &str,
    output_file_path: &str,
    prover_key: Buffer,
) -> Result<FileEncodingInfo> {
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer for streaming

    let encoder = FileEncoder::new(prover_key)?;

    let input_file = File::open(input_file_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to open input file: {}", e),
        )
    })?;

    let output_file = File::create(output_file_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to create output file: {}", e),
        )
    })?;

    let mut reader = BufReader::new(input_file);
    let mut writer = BufWriter::new(output_file);

    // Stream hashing for original file
    let mut original_hasher = blake3::Hasher::new();
    let mut encoded_hasher = blake3::Hasher::new();

    let mut chunk_index = 0u32;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to read from input: {}", e),
            )
        })?;

        if bytes_read == 0 {
            break; // End of file
        }

        // Process data in chunk-sized pieces for encoding
        let actual_data = &buffer[..bytes_read];
        original_hasher.update(actual_data);

        let mut offset = 0;
        while offset < bytes_read {
            let chunk_end = std::cmp::min(offset + CHUNK_SIZE_BYTES as usize, bytes_read);
            let chunk_data = &actual_data[offset..chunk_end];

            // Encode this chunk
            let encoded_chunk = encoder.encode_chunk(chunk_data, chunk_index)?;

            // Update encoded hasher
            encoded_hasher.update(&encoded_chunk);

            // Write encoded chunk
            writer.write_all(&encoded_chunk).map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to write encoded chunk: {}", e),
                )
            })?;

            offset = chunk_end;
            chunk_index += 1;
        }
    }

    // Flush the writer
    writer.flush().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to flush output: {}", e),
        )
    })?;

    // Finalize hashes
    let original_hash = Buffer::from(original_hasher.finalize().as_bytes().to_vec());
    let encoded_hash = Buffer::from(encoded_hasher.finalize().as_bytes().to_vec());

    Ok(encoder.create_encoding_info(original_hash, encoded_hash))
}

/// Stream decode entire file back to original (never loads entire file in memory)
pub fn stream_decode_file(
    input_file_path: &str,
    output_file_path: &str,
    prover_key: Buffer,
) -> Result<FileEncodingInfo> {
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer for streaming

    let encoder = FileEncoder::new(prover_key)?;

    let input_file = File::open(input_file_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to open input file: {}", e),
        )
    })?;

    let output_file = File::create(output_file_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to create output file: {}", e),
        )
    })?;

    let mut reader = BufReader::new(input_file);
    let mut writer = BufWriter::new(output_file);

    // Stream hashing for files
    let mut encoded_hasher = blake3::Hasher::new();
    let mut decoded_hasher = blake3::Hasher::new();

    let mut chunk_index = 0u32;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to read from input: {}", e),
            )
        })?;

        if bytes_read == 0 {
            break; // End of file
        }

        // Process data in chunk-sized pieces for decoding
        let actual_data = &buffer[..bytes_read];
        encoded_hasher.update(actual_data);

        let mut offset = 0;
        while offset < bytes_read {
            let chunk_end = std::cmp::min(offset + CHUNK_SIZE_BYTES as usize, bytes_read);
            let chunk_data = &actual_data[offset..chunk_end];

            // Decode this chunk
            let decoded_chunk = encoder.decode_chunk(chunk_data, chunk_index)?;

            // Update decoded hasher
            decoded_hasher.update(&decoded_chunk);

            // Write decoded chunk
            writer.write_all(&decoded_chunk).map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Failed to write decoded chunk: {}", e),
                )
            })?;

            offset = chunk_end;
            chunk_index += 1;
        }
    }

    // Flush the writer
    writer.flush().map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to flush output: {}", e),
        )
    })?;

    // Finalize hashes
    let encoded_hash = Buffer::from(encoded_hasher.finalize().as_bytes().to_vec());
    let original_hash = Buffer::from(decoded_hasher.finalize().as_bytes().to_vec());

    Ok(encoder.create_encoding_info(original_hash, encoded_hash))
}

/// Calculate hash of entire file using streaming (never loads entire file in memory)
pub fn stream_calculate_file_hash(file_path: &str) -> Result<Buffer> {
    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer

    let file = File::open(file_path).map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("Failed to open file: {}", e),
        )
    })?;

    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("Failed to read file: {}", e),
            )
        })?;

        if bytes_read == 0 {
            break; // End of file
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(Buffer::from(hasher.finalize().as_bytes().to_vec()))
}

/// Verify file encoding matches expected prover using streaming
pub fn stream_verify_file_encoding(
    encoded_file_path: &str,
    encoding_info: &FileEncodingInfo,
) -> Result<bool> {
    let encoder = FileEncoder::new(encoding_info.prover_key.clone())?;

    // Verify encoder recognizes this encoding
    if !encoder.verify_encoding(encoding_info)? {
        return Ok(false);
    }

    // Calculate actual encoded file hash using streaming
    let actual_hash = stream_calculate_file_hash(encoded_file_path)?;

    // Compare with expected hash
    Ok(actual_hash.as_ref() == encoding_info.encoded_hash.as_ref())
}

/// Generate random local entropy for encoding
pub fn generate_local_entropy() -> Buffer {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = DefaultHasher::new();

    // Use current time as entropy source
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    timestamp.hash(&mut hasher);

    // Add process-specific entropy
    std::process::id().hash(&mut hasher);

    // Create 32-byte entropy from hasher
    let hash_value = hasher.finish();
    let mut entropy = Vec::new();

    // Expand 8-byte hash to 32 bytes
    for i in 0..4 {
        entropy.extend_from_slice(&(hash_value.wrapping_add(i)).to_be_bytes());
    }

    Buffer::from(entropy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_encoding_decoding() {
        let prover_key = Buffer::from([42u8; 32].to_vec());
        let encoder = FileEncoder::new(prover_key).unwrap();

        let original_data = vec![1u8, 2u8, 3u8, 4u8];
        let mut chunk = original_data.clone();
        chunk.resize(CHUNK_SIZE_BYTES as usize, 0); // Pad to chunk size

        let encoded = encoder.encode_chunk(&chunk, 0).unwrap();
        let decoded = encoder.decode_chunk(&encoded, 0).unwrap();

        assert_eq!(chunk, decoded);
        assert_ne!(chunk, encoded); // Should be different after encoding
    }

    #[test]
    fn test_chunk_encoding_deterministic() {
        let prover_key = Buffer::from([42u8; 32].to_vec());
        let encoder = FileEncoder::new(prover_key).unwrap();

        let chunk = vec![5u8; CHUNK_SIZE_BYTES as usize];

        let encoded1 = encoder.encode_chunk(&chunk, 1).unwrap();
        let encoded2 = encoder.encode_chunk(&chunk, 1).unwrap();

        assert_eq!(encoded1, encoded2); // Should be deterministic
    }

    #[test]
    fn test_different_provers_different_encoding() {
        let prover_key1 = Buffer::from([1u8; 32].to_vec());
        let prover_key2 = Buffer::from([2u8; 32].to_vec());

        let encoder1 = FileEncoder::new(prover_key1).unwrap();
        let encoder2 = FileEncoder::new(prover_key2).unwrap();

        let chunk = vec![10u8; CHUNK_SIZE_BYTES as usize];

        let encoded1 = encoder1.encode_chunk(&chunk, 0).unwrap();
        let encoded2 = encoder2.encode_chunk(&chunk, 0).unwrap();

        assert_ne!(encoded1, encoded2); // Different provers should produce different encodings
    }

    #[test]
    fn test_encoding_info_creation() {
        let prover_key = Buffer::from([42u8; 32].to_vec());
        let encoder = FileEncoder::new(prover_key.clone()).unwrap();

        let original_hash = Buffer::from([1u8; 32].to_vec());
        let encoded_hash = Buffer::from([2u8; 32].to_vec());

        let info = encoder.create_encoding_info(original_hash.clone(), encoded_hash.clone());

        assert_eq!(info.prover_key.as_ref(), prover_key.as_ref());
        assert_eq!(info.original_hash.as_ref(), original_hash.as_ref());
        assert_eq!(info.encoded_hash.as_ref(), encoded_hash.as_ref());
        assert_eq!(info.encoding_version, 1);
    }

    #[test]
    fn test_local_entropy_generation() {
        let entropy1 = generate_local_entropy();
        let entropy2 = generate_local_entropy();

        assert_eq!(entropy1.len(), 32);
        assert_eq!(entropy2.len(), 32);
        // Should be different (with very high probability)
        assert_ne!(entropy1.as_ref(), entropy2.as_ref());
    }
}
