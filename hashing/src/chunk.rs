use std::convert::TryFrom;

use crate::{
    primitives::{blake2b_hash, IndexedMerkleProof},
    MerkleConstructionError,
};

#[cfg_attr(
    feature = "std",
    derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize),
    schemars(with = "String", description = "Hex-encoded hash digest."),
    serde(
        deny_unknown_fields,
        try_from = "ChunkWithProofDeserializeValidator<N>"
    )
)]
pub struct ChunkWithProof<const N: usize> {
    proof: IndexedMerkleProof,
    chunk: Vec<u8>,
}

#[cfg_attr(
    feature = "std",
    derive(serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct ChunkWithProofDeserializeValidator<const N: usize> {
    proof: IndexedMerkleProof,
    chunk: Vec<u8>,
}

impl<const N: usize> TryFrom<ChunkWithProofDeserializeValidator<N>> for ChunkWithProof<N> {
    type Error = MerkleConstructionError;
    fn try_from(value: ChunkWithProofDeserializeValidator<N>) -> Result<Self, Self::Error> {
        let candidate = Self {
            proof: value.proof,
            chunk: value.chunk,
        };
        if candidate.is_valid() {
            Ok(candidate)
        } else {
            Err(MerkleConstructionError::IncorrectChunkProof)
        }
    }
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

    pub fn is_valid(&self) -> bool {
        let chunk_hash = blake2b_hash(self.chunk());
        self.proof
            .merkle_proof()
            .iter()
            .next()
            .map_or(false, |first_hash| chunk_hash == *first_hash)
    }

    /// Get a reference to the chunk with proof's chunk.
    pub fn chunk(&self) -> &[u8] {
        self.chunk.as_slice()
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
        primitives::{blake2b_hash, hash_merkle_tree, Blake2bHash, IndexedMerkleProof},
    };

    fn prepare_bytes(length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();

        let mut v = Vec::with_capacity(length);
        (0..length).into_iter().for_each(|_| v.push(rng.gen()));
        v
    }

    #[test]
    fn generates_correct_proof() {
        let large_data_size = (CHUNK_SIZE * 1024) as usize;
        let large_data = prepare_bytes(large_data_size);

        let number_of_chunks: u64 = large_data.chunks(CHUNK_SIZE).len().try_into().unwrap();

        assert!(!(0..number_of_chunks)
            .into_iter()
            .map(|chunk_index| {
                ChunkWithProof::<CHUNK_SIZE>::new(large_data.as_slice(), chunk_index).unwrap()
            })
            .map(|chunk_with_proof| chunk_with_proof.is_valid())
            .any(|valid| !valid));
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

    #[test]
    fn validates_chunk_with_proofs() {
        let large_data_size = (CHUNK_SIZE * 2) as usize;
        let large_data = prepare_bytes(large_data_size);

        impl<const N: usize> ChunkWithProof<N> {
            fn replace_first_proof(self) -> Self {
                let ChunkWithProof { mut proof, chunk } = self;

                let mut merkle_proof: Vec<_> = proof.merkle_proof().to_vec();
                merkle_proof.swap(0, 1);
                proof.inject_merkle_proof(merkle_proof);

                ChunkWithProof { proof, chunk }
            }
        }

        let chunk_with_proof = ChunkWithProof::<CHUNK_SIZE>::new(large_data.as_slice(), 0).unwrap();
        assert!(chunk_with_proof.is_valid());

        let chunk_with_incorrect_proof = chunk_with_proof.replace_first_proof();
        assert!(!chunk_with_incorrect_proof.is_valid());
    }

    #[test]
    fn validates_chunk_with_proof_after_deserialization() {
        let large_data_size = (CHUNK_SIZE * 2) as usize;
        let large_data = prepare_bytes(large_data_size);
        let chunk_with_proof = ChunkWithProof::<CHUNK_SIZE>::new(large_data.as_slice(), 0).unwrap();

        let json = serde_json::to_string(&chunk_with_proof).unwrap();
        serde_json::from_str::<ChunkWithProof<CHUNK_SIZE>>(&json)
            .expect("should deserialize correctly");

        let chunk_with_incorrect_proof = chunk_with_proof.replace_first_proof();
        let json = serde_json::to_string(&chunk_with_incorrect_proof).unwrap();
        serde_json::from_str::<ChunkWithProof<CHUNK_SIZE>>(&json)
            .expect_err("shoud not deserialize correctly");
    }
}
