//! This file contains primitive hashing operations.
//! Primitives are defined in terms of [BLAKE2b][1] hashes.
//!
//! Outside of this file, only [`Digest`] is permitted for constructing hashes.
//!
//! [1]: https://datatracker.ietf.org/doc/html/rfc7693

#[cfg(all(feature = "no-std", no_std))]
use alloc::vec::Vec;
#[cfg(all(features = "std", not(no_std)))]
use std::vec::Vec;

use blake2::{
    digest::{Update, VariableOutput},
    VarBlake2b,
};
use itertools::Itertools;

use crate::Digest;

/// The output of the [BLAKE2b][1] hashing algorithm.
///
/// [1]: https://datatracker.ietf.org/doc/html/rfc7693
#[derive(Clone, PartialEq, Eq, Debug)]
struct Blake2bHash([u8; Digest::LENGTH]);

impl AsRef<[u8]> for Blake2bHash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Sentinel hash to be used for hashing options in the case of [None].
const SENTINEL0: Blake2bHash = Blake2bHash([0u8; 32]);
/// Sentinel hash to be used by [hash_slice_rfold]. Terminates the fold.
const SENTINEL1: Blake2bHash = Blake2bHash([1u8; 32]);
/// Sentinel hash to be used by [hash_vec_merkle_tree] in the case of an empty list.
const SENTINEL2: Blake2bHash = Blake2bHash([2u8; 32]);

/// Creates a 32-byte hash digest from a given a piece of data
fn blake2b_hash<T: AsRef<[u8]>>(data: T) -> Blake2bHash {
    let mut result = [0; Digest::LENGTH];

    let mut hasher = VarBlake2b::new(Digest::LENGTH).expect("should create hasher");
    hasher.update(data);
    hasher.finalize_variable(|slice| {
        result.copy_from_slice(slice);
    });
    Blake2bHash(result)
}

/// Hashes a pair of byte slices into a single [`Blake2bHash`]
fn hash_pair<T: AsRef<[u8]>, U: AsRef<[u8]>>(data1: T, data2: U) -> Blake2bHash {
    let mut result = [0; Digest::LENGTH];
    let mut hasher = VarBlake2b::new(Digest::LENGTH).unwrap();
    hasher.update(data1);
    hasher.update(data2);
    hasher.finalize_variable(|slice| {
        result.copy_from_slice(slice);
    });
    Blake2bHash(result)
}

fn u64_to_byte_slice(x: u64) -> [u8; 8] {
    [
        x as u8,
        (x >> 8) as u8,
        (x >> 16) as u8,
        (x >> 24) as u8,
        (x >> 32) as u8,
        (x >> 40) as u8,
        (x >> 48) as u8,
        (x >> 56) as u8,
    ]
}

/// Hashes a `impl IntoIterator` of [`Blake2bHash`]s into a single [`Blake2bHash`] by constructing a
/// [Merkle tree][1]. Reduces pairs of elements in the [`Vec`] by repeatedly calling [hash_pair].
///
/// The pattern of hashing is as follows.  It is akin to [graph reduction][2]:
///
/// ```text
/// a b c d e f
/// |/  |/  |/
/// g   h   i
/// | /   /
/// |/   /
/// j   k
/// | /
/// |/
/// l
/// ```
///
/// Finally hashes the number of elements resulting hash. In the example above the final output
/// would be `hash_pair(u64_as_slice(6), l)`.
///
/// Returns [`SENTINEL2`] when the input is empty.
///
/// [1]: https://en.wikipedia.org/wiki/Merkle_tree
/// [2]: https://en.wikipedia.org/wiki/Graph_reduction
fn hash_merkle_tree<I>(leaves: I) -> Blake2bHash
where
    I: IntoIterator<Item = Blake2bHash>,
{
    let (leaf_count, raw_root) = leaves
        .into_iter()
        .map(|x| (1usize, x))
        .tree_fold1(|(count_x, hash_x), (count_y, hash_y)| {
            (count_x + count_y, hash_pair(&hash_x, &hash_y))
        })
        .unwrap_or((0, SENTINEL2));
    let leaf_count_bytes = u64_to_byte_slice(leaf_count as u64);
    hash_pair(leaf_count_bytes, raw_root)
}

struct ChunkAndMerkleProof {
    index: u64,
    count: u64,
    merkle_proof: Vec<Blake2bHash>,
}

impl ChunkAndMerkleProof {
    pub fn root_hash(&self) -> Result<Blake2bHash, ()> {
        let ChunkAndMerkleProof {
            index,
            count,
            merkle_proof,
        } = self;
        let raw_root = compute_raw_root_from_proof(*index as usize, *count as usize, merkle_proof)?;
        Ok(hash_pair(u64_to_byte_slice(*count as u64), raw_root))
    }
}

fn merkle_proof<I>(count: usize, leaves: I, index: usize) -> Result<ChunkAndMerkleProof, ()>
where
    I: IntoIterator<Item = Blake2bHash>,
{
    enum HashOrProof {
        Hash(Blake2bHash),
        Proof(Vec<Blake2bHash>),
    }
    use HashOrProof::{Hash, Proof};

    leaves
        .into_iter()
        .enumerate()
        .map(|(i, hash)| {
            if i == index {
                Proof(vec![hash])
            } else {
                Hash(hash)
            }
        })
        .tree_fold1(|x, y| match (x, y) {
            (Hash(mut hash_x), Hash(hash_y)) => {
                hash_x = hash_pair(&hash_x, &hash_y);
                Hash(hash_x)
            }
            (Hash(hash), Proof(mut proof)) | (Proof(mut proof), Hash(hash)) => {
                proof.push(hash);
                Proof(proof)
            }
            (Proof(_), Proof(_)) => unreachable!(),
        })
        .and_then(|x| match x {
            Proof(merkle_proof) => Some(ChunkAndMerkleProof {
                index: index as u64,
                count: count as u64,
                merkle_proof,
            }),
            _ => None,
        })
        .ok_or(())
}

fn compute_raw_root_from_proof(
    index: usize,
    leaf_count: usize,
    proof: &[Blake2bHash],
) -> Result<Blake2bHash, ()> {
    if leaf_count == 0 {
        if proof.is_empty() {
            return Ok(SENTINEL2);
        } else {
            return Err(());
        }
    }
    if proof.is_empty() {
        return Err(());
    }
    if leaf_count == 1 {
        if proof.len() == 1 {
            return Ok(proof[0].clone());
        } else {
            return Err(());
        }
    }
    let n = leaf_count.next_power_of_two();
    let half = n / 2;
    let last = proof.len() - 1;
    if index < half {
        let left = compute_raw_root_from_proof(index, half, &proof[..last])?;
        Ok(hash_pair(&left, &proof[last]))
    } else {
        let right = compute_raw_root_from_proof(index - half, leaf_count - half, &proof[..last])?;
        Ok(hash_pair(&proof[last], &right))
    }
}

/// Hashes a `&[Blake2bHash]` using a [right fold][1].
///
/// This pattern of hashing is as follows:
///
/// ```text
/// hash_pair(a, &hash_pair(b, &hash_pair(c, &SENTINEL)))
/// ```
///
/// Unlike Merkle trees, this is suited to hashing heterogeneous lists we may wish to extend in the
/// future (ie, hashes of data structures that may undergo revision).
///
/// Returns [`SENTINEL1`] when given an empty [`Vec`] as input.
///
/// [1]: https://en.wikipedia.org/wiki/Fold_(higher-order_function)#Linear_folds
fn hash_slice_rfold(slice: &[Blake2bHash]) -> Blake2bHash {
    hash_slice_with_proof(slice, SENTINEL1)
}

/// Hashes a `&[Blake2bHash]` using a [right fold][1]. Uses `proof` as a Merkle proof for the
/// missing tail of the slice.
///
/// [1]: https://en.wikipedia.org/wiki/Fold_(higher-order_function)#Linear_folds
fn hash_slice_with_proof(slice: &[Blake2bHash], proof: Blake2bHash) -> Blake2bHash {
    slice
        .iter()
        .rfold(proof, |prev, next| hash_pair(next, &prev))
}

#[cfg(test)]
mod test {
    use rand::Rng;

    use super::*;

    #[test]
    fn test_merkle_proofs() {
        let mut rng = rand::thread_rng();
        for _ in 0..20 {
            let leaf_count: usize = rng.gen_range(1..100);
            let i = rng.gen_range(0..leaf_count);
            let leaves: Vec<Blake2bHash> = (0..leaf_count)
                .map(|i| blake2b_hash(i.to_le_bytes()))
                .collect();
            let root = hash_merkle_tree(leaves.iter().cloned());
            let mut chunk_and_merkle_proof =
                merkle_proof(leaf_count, leaves.iter().cloned(), i).unwrap();
            chunk_and_merkle_proof.count = leaves.len() as u64;
            assert_eq!(leaves[i], chunk_and_merkle_proof.merkle_proof[0]);
            assert_eq!(
                root,
                chunk_and_merkle_proof.root_hash().unwrap() /* compute_root_from_proof(&
                                                             * chunk_and_merkle_proof, i,
                                                             * leaf_count).unwrap() */
            );
        }
    }
}
