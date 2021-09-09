use crate::{
    error::MerkleVerificationError,
    primitives::{blake2b_hash, IndexedMerkleProof},
    MerkleConstructionError,
};

pub struct ChunkWithProof<const N: usize> {
    proof: IndexedMerkleProof,
    chunk: Vec<u8>,
}

impl<const N: usize> ChunkWithProof<N> {
    pub const CHUNK_SIZE: usize = N;

    pub fn new(data: &[u8], index: u64) -> Result<Self, MerkleConstructionError> {
        if data.len() < Self::CHUNK_SIZE * (index as usize) {
            return Err(todo!());
        }
        // TODO: empty data as chunks of empty slices
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
    // Make proptests to make sure that ChunkWithProof agrees with hash_merkle_tree of the chunked
    // data

    const CHUNK_SIZE: usize = 10;

    use rand::Rng;
    use std::convert::TryInto;

    use crate::{
        chunk::ChunkWithProof,
        primitives::{blake2b_hash, hash_merkle_tree, IndexedMerkleProof},
    };

    fn prepare_bytes(length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();

        let mut v = Vec::with_capacity(length);
        (0..length).into_iter().for_each(|_| v.push(rng.gen()));
        v
    }

    #[test]
    fn first_hash_in_proof_matches_data_hash() {
        let large_data_size = (CHUNK_SIZE * 1024) as usize;
        let large_data = prepare_bytes(large_data_size);

        let number_of_chunks: u64 = large_data.chunks(CHUNK_SIZE).len().try_into().unwrap();

        (0..number_of_chunks)
            .into_iter()
            .map(|chunk_index| {
                ChunkWithProof::<CHUNK_SIZE>::new(large_data.as_slice(), chunk_index).unwrap()
            })
            .map(|cwp| (cwp.chunk, cwp.proof.merkle_proof()[0].clone()))
            .map(|(chunk, first_hash)| (blake2b_hash(chunk), first_hash))
            .for_each(|(chunk_hash, first_hash)| assert_eq!(chunk_hash, first_hash));
    }

    #[test]
    fn validate_chunks_against_hash_merkle_tree() {
        let data = prepare_bytes(CHUNK_SIZE * 2);
        let expected_root = hash_merkle_tree(data.chunks(CHUNK_SIZE).map(blake2b_hash));

        // Calculate proof with `ChunkWithProof`
        let ChunkWithProof {
            proof: proof_0,
            chunk: _,
        } = ChunkWithProof::<CHUNK_SIZE>::new(data.as_slice(), 0).unwrap();
        let ChunkWithProof {
            proof: proof_1,
            chunk: _,
        } = ChunkWithProof::<CHUNK_SIZE>::new(data.as_slice(), 1).unwrap();

        assert_eq!(proof_0.root_hash(), expected_root);
        assert_eq!(proof_1.root_hash(), expected_root);
    }
}
