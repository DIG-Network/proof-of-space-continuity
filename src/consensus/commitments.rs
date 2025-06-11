use napi::bindgen_prelude::*;

use crate::core::{
    errors::{HashChainError, HashChainResult},
    types::*,
    utils::{compute_sha256, validate_block_hash, validate_public_key},
};

/// Create ownership commitment
pub fn create_ownership_commitment_internal(
    public_key: Buffer,
    data_hash: Buffer,
) -> Result<OwnershipCommitment> {
    validate_public_key(&public_key)?;

    if data_hash.len() != 32 {
        return Err(Error::new(
            Status::InvalidArg,
            "Data hash must be 32 bytes".to_string(),
        ));
    }

    let mut commitment_data = Vec::new();
    commitment_data.extend_from_slice(&data_hash);
    commitment_data.extend_from_slice(&public_key);
    let commitment_hash = compute_sha256(&commitment_data);

    Ok(OwnershipCommitment {
        public_key,
        data_hash,
        commitment_hash: Buffer::from(commitment_hash.to_vec()),
    })
}

/// Create anchored ownership commitment
pub fn create_anchored_ownership_commitment_internal(
    ownership_commitment: OwnershipCommitment,
    block_commitment: BlockCommitment,
) -> Result<AnchoredOwnershipCommitment> {
    validate_block_hash(&block_commitment.block_hash)?;

    if block_commitment.block_height < 0.0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Block height must be non-negative".to_string(),
        ));
    }

    let mut anchored_data = Vec::new();
    anchored_data.extend_from_slice(&ownership_commitment.commitment_hash);
    anchored_data.extend_from_slice(&block_commitment.block_hash);
    // Include block height to ensure uniqueness
    anchored_data.extend_from_slice(&(block_commitment.block_height as u64).to_be_bytes());
    let anchored_hash = compute_sha256(&anchored_data);

    Ok(AnchoredOwnershipCommitment {
        ownership_commitment,
        block_commitment,
        anchored_hash: Buffer::from(anchored_hash.to_vec()),
    })
}

/// Create physical access commitment proving data access at specific block
pub fn create_physical_access_commitment_internal(
    previous_commitment: Buffer,
    block_hash: Buffer,
    block_height: f64,
    selected_chunks: Vec<u32>,
    chunk_hashes: Vec<Buffer>,
) -> Result<PhysicalAccessCommitment> {
    validate_block_hash(&block_hash)?;

    if previous_commitment.len() != 32 {
        return Err(Error::new(
            Status::InvalidArg,
            "Previous commitment must be 32 bytes".to_string(),
        ));
    }

    if block_height < 0.0 {
        return Err(Error::new(
            Status::InvalidArg,
            "Block height must be non-negative".to_string(),
        ));
    }

    if selected_chunks.len() != chunk_hashes.len() {
        return Err(Error::new(
            Status::InvalidArg,
            "Selected chunks and chunk hashes must have same length".to_string(),
        ));
    }

    if selected_chunks.len() != CHUNKS_PER_BLOCK as usize {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Must select exactly {} chunks", CHUNKS_PER_BLOCK),
        ));
    }

    // Validate all chunk hashes are 32 bytes
    for (i, chunk_hash) in chunk_hashes.iter().enumerate() {
        if chunk_hash.len() != 32 {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Chunk hash {} must be 32 bytes", i),
            ));
        }
    }

    // Create commitment hash
    let mut commitment_data = Vec::new();
    commitment_data.extend_from_slice(&previous_commitment);
    commitment_data.extend_from_slice(&block_hash);
    commitment_data.extend_from_slice(&(block_height as u64).to_be_bytes());

    for &chunk_idx in &selected_chunks {
        commitment_data.extend_from_slice(&chunk_idx.to_be_bytes());
    }

    for chunk_hash in &chunk_hashes {
        commitment_data.extend_from_slice(chunk_hash);
    }

    let commitment_hash = compute_sha256(&commitment_data);

    Ok(PhysicalAccessCommitment {
        block_height,
        previous_commitment,
        block_hash,
        selected_chunks,
        chunk_hashes,
        commitment_hash: Buffer::from(commitment_hash.to_vec()),
    })
}

/// Calculate commitment hash for verification
pub fn calculate_commitment_hash_internal(commitment: &PhysicalAccessCommitment) -> Result<Buffer> {
    let mut commitment_data = Vec::new();
    commitment_data.extend_from_slice(&commitment.previous_commitment);
    commitment_data.extend_from_slice(&commitment.block_hash);
    commitment_data.extend_from_slice(&(commitment.block_height as u64).to_be_bytes());

    for &chunk_idx in &commitment.selected_chunks {
        commitment_data.extend_from_slice(&chunk_idx.to_be_bytes());
    }

    for chunk_hash in &commitment.chunk_hashes {
        commitment_data.extend_from_slice(chunk_hash);
    }

    let hash = compute_sha256(&commitment_data);
    Ok(Buffer::from(hash.to_vec()))
}

/// Verify a physical access commitment
pub fn verify_physical_access_commitment(commitment: &PhysicalAccessCommitment) -> Result<bool> {
    // Recalculate commitment hash
    let calculated_hash = calculate_commitment_hash_internal(commitment)?;

    // Compare with stored hash
    Ok(calculated_hash.as_ref() == commitment.commitment_hash.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership_commitment_creation() {
        let public_key = Buffer::from([1u8; 32].to_vec());
        let data_hash = Buffer::from([2u8; 32].to_vec());

        let commitment =
            create_ownership_commitment_internal(public_key.clone(), data_hash.clone()).unwrap();

        assert_eq!(commitment.public_key.as_ref(), public_key.as_ref());
        assert_eq!(commitment.data_hash.as_ref(), data_hash.as_ref());
        assert_eq!(commitment.commitment_hash.len(), 32);
    }

    #[test]
    fn test_ownership_commitment_validation() {
        let invalid_public_key = Buffer::from([1u8; 31].to_vec()); // Wrong size
        let data_hash = Buffer::from([2u8; 32].to_vec());

        let result = create_ownership_commitment_internal(invalid_public_key, data_hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_anchored_ownership_commitment() {
        let public_key = Buffer::from([1u8; 32].to_vec());
        let data_hash = Buffer::from([2u8; 32].to_vec());
        let block_hash = Buffer::from([3u8; 32].to_vec());

        let ownership_commitment =
            create_ownership_commitment_internal(public_key, data_hash).unwrap();
        let block_commitment = BlockCommitment {
            block_height: 12345.0,
            block_hash,
        };

        let anchored = create_anchored_ownership_commitment_internal(
            ownership_commitment.clone(),
            block_commitment.clone(),
        )
        .unwrap();

        assert_eq!(
            anchored.ownership_commitment.public_key.as_ref(),
            ownership_commitment.public_key.as_ref()
        );
        assert_eq!(
            anchored.block_commitment.block_height,
            block_commitment.block_height
        );
        assert_eq!(anchored.anchored_hash.len(), 32);
    }

    #[test]
    fn test_physical_access_commitment() {
        let previous_commitment = Buffer::from([1u8; 32].to_vec());
        let block_hash = Buffer::from([2u8; 32].to_vec());
        let selected_chunks = vec![0, 1, 2, 3];
        let chunk_hashes = vec![
            Buffer::from([10u8; 32].to_vec()),
            Buffer::from([11u8; 32].to_vec()),
            Buffer::from([12u8; 32].to_vec()),
            Buffer::from([13u8; 32].to_vec()),
        ];

        let commitment = create_physical_access_commitment_internal(
            previous_commitment.clone(),
            block_hash.clone(),
            12345.0,
            selected_chunks.clone(),
            chunk_hashes.clone(),
        )
        .unwrap();

        assert_eq!(
            commitment.previous_commitment.as_ref(),
            previous_commitment.as_ref()
        );
        assert_eq!(commitment.block_hash.as_ref(), block_hash.as_ref());
        assert_eq!(commitment.block_height, 12345.0);
        assert_eq!(commitment.selected_chunks, selected_chunks);
        assert_eq!(commitment.chunk_hashes.len(), chunk_hashes.len());
        assert_eq!(commitment.commitment_hash.len(), 32);

        // Verify the commitment
        let is_valid = verify_physical_access_commitment(&commitment).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_physical_access_commitment_validation() {
        let previous_commitment = Buffer::from([1u8; 32].to_vec());
        let block_hash = Buffer::from([2u8; 32].to_vec());
        let selected_chunks = vec![0, 1, 2]; // Wrong count
        let chunk_hashes = vec![
            Buffer::from([10u8; 32].to_vec()),
            Buffer::from([11u8; 32].to_vec()),
            Buffer::from([12u8; 32].to_vec()),
        ];

        let result = create_physical_access_commitment_internal(
            previous_commitment,
            block_hash,
            12345.0,
            selected_chunks,
            chunk_hashes,
        );

        assert!(result.is_err());
    }
}
