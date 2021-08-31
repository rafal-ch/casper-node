use crate::{
    error::MerkleVerificationError,
    primitives::{blake2b_hash, IndexedMerkleProof},
    MerkleConstructionError,
};

pub struct ChunkWithProof {
    proof: IndexedMerkleProof,
    chunk: Vec<u8>,
}

impl ChunkWithProof {
    pub const CHUNK_SIZE: usize = 1_048_576; // 2^20
    pub fn new(data: &[u8], index: u64) -> Result<Self, MerkleConstructionError> {
        if data.len() < Self::CHUNK_SIZE * (index as usize) {
            return Err(todo!());
        }
        // TODO: empty data as zero chunks
        let proof =
            IndexedMerkleProof::new(data.chunks(Self::CHUNK_SIZE).map(blake2b_hash), index)?;
        let chunk = data[Self::CHUNK_SIZE * (index as usize)
            ..data.len().min(Self::CHUNK_SIZE * ((index as usize) + 1))]
            .to_vec();
        Ok(ChunkWithProof { proof, chunk })
    }
}

#[cfg(test)]
mod test {
    // TODO: test empty chunk
}
