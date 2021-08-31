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

use crate::{Digest, MerkleConstructionError, MerkleVerificationError};

/// The output of the [BLAKE2b][1] hashing algorithm.
///
/// [1]: https://datatracker.ietf.org/doc/html/rfc7693
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Blake2bHash([u8; Digest::LENGTH]);

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
pub(super) fn blake2b_hash<T: AsRef<[u8]>>(data: T) -> Blake2bHash {
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
        .map(|x| (1u64, x))
        .tree_fold1(|(count_x, hash_x), (count_y, hash_y)| {
            (count_x + count_y, hash_pair(&hash_x, &hash_y))
        })
        .unwrap_or((0, SENTINEL2));
    let leaf_count_bytes = leaf_count.to_le_bytes();
    hash_pair(leaf_count_bytes, raw_root)
}

pub struct IndexedMerkleProof {
    index: u64,
    count: u64,
    merkle_proof: Vec<Blake2bHash>,
}

impl IndexedMerkleProof {
    pub(super) fn new<I>(
        leaves: I,
        index: u64,
    ) -> Result<IndexedMerkleProof, MerkleConstructionError>
    where
        I: IntoIterator<Item = Blake2bHash>,
    {
        enum HashOrProof {
            Hash(Blake2bHash),
            Proof(Vec<Blake2bHash>),
        }
        use HashOrProof::{Hash, Proof};

        let maybe_count_and_proof = leaves
            .into_iter()
            .enumerate()
            .map(|(i, hash)| {
                if i as u64 == index {
                    (1u64, Proof(vec![hash]))
                } else {
                    (1u64, Hash(hash))
                }
            })
            .tree_fold1(|(count_x, x), (count_y, y)| match (x, y) {
                (Hash(hash_x), Hash(hash_y)) => {
                    (count_x + count_y, Hash(hash_pair(&hash_x, &hash_y)))
                }
                (Hash(hash), Proof(mut proof)) | (Proof(mut proof), Hash(hash)) => {
                    proof.push(hash);
                    (count_x + count_y, Proof(proof))
                }
                (Proof(_), Proof(_)) => unreachable!(),
            });
        match maybe_count_and_proof {
            None => {
                if index != 0 {
                    Err(MerkleConstructionError::EmptyProofMustHaveIndex { index })
                } else {
                    Ok(IndexedMerkleProof {
                        index: 0,
                        count: 0,
                        merkle_proof: Vec::new(),
                    })
                }
            }
            Some((count, Hash(_))) => {
                Err(MerkleConstructionError::IndexOutOfBounds { count, index })
            }
            Some((count, Proof(merkle_proof))) => Ok(IndexedMerkleProof {
                index,
                count,
                merkle_proof,
            }),
        }
    }

    pub(crate) fn root_hash(&self) -> Blake2bHash {
        // Recursion depth can not exceed 64
        fn compute_raw_root_from_proof(
            index: usize,
            leaf_count: usize,
            proof: &[Blake2bHash],
        ) -> Blake2bHash {
            if leaf_count == 0 {
                return SENTINEL2;
            }
            if leaf_count == 1 {
                return proof[0].clone();
            }
            let n = leaf_count.next_power_of_two();
            let half = n / 2;
            let last = proof.len() - 1;
            if index < half {
                let left = compute_raw_root_from_proof(index, half, &proof[..last]);
                hash_pair(&left, &proof[last])
            } else {
                let right =
                    compute_raw_root_from_proof(index - half, leaf_count - half, &proof[..last]);
                hash_pair(&proof[last], &right)
            }
        }

        let IndexedMerkleProof {
            index,
            count,
            merkle_proof,
        } = self;
        let raw_root = compute_raw_root_from_proof(*index as usize, *count as usize, merkle_proof);
        hash_pair(count.to_le_bytes(), raw_root)
    }

    pub fn index(&self) -> u64 {
        self.index
    }
    pub fn count(&self) -> u64 {
        self.count
    }

    pub(crate) fn merkle_proof(&self) -> &[Blake2bHash] {
        &self.merkle_proof
    }

    // Proof lengths are never bigger than 65, so we can use a u8 here
    // The reason they are never bigger than 65 is because we are using 64 bit counts
    fn compute_expected_proof_length(&self) -> u64 {
        if self.count == 0 {
            return 0;
        }
        let mut l = 1;
        let mut n = self.count;
        let mut i = self.index;
        while n > 1 {
            let half = 1u64 << (63 - (n - 1).leading_zeros());
            if i < half {
                n = half;
            } else {
                n -= half;
                i -= half;
            }
            l += 1;
        }
        l
    }

    fn verify(&self) -> Result<(), MerkleVerificationError> {
        if !((self.count == 0 && self.index == 0) || self.index < self.count) {
            return Err(MerkleVerificationError::IndexOutOfBounds {
                count: self.count,
                index: self.index,
            });
        }
        let expected_proof_length = self.compute_expected_proof_length();
        if self.merkle_proof.len() != expected_proof_length as usize {
            return Err(MerkleVerificationError::UnexpectedProofLength {
                count: self.count,
                index: self.index,
                expected_proof_length,
                actual_proof_length: self.merkle_proof.len(),
            });
        }
        Ok(())
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
    use proptest::prelude::prop_assert;
    use proptest_attr_macro::proptest;
    use rand::Rng;

    use super::*;

    #[test]
    fn test_merkle_proofs() {
        let mut rng = rand::thread_rng();
        for _ in 0..20 {
            let leaf_count: u64 = rng.gen_range(1..100);
            let index = rng.gen_range(0..leaf_count);
            let leaves: Vec<Blake2bHash> = (0..leaf_count)
                .map(|i| blake2b_hash(i.to_le_bytes()))
                .collect();
            let root = hash_merkle_tree(leaves.iter().cloned());
            let indexed_merkle_proof =
                IndexedMerkleProof::new(leaves.iter().cloned(), index).unwrap();
            assert_eq!(
                indexed_merkle_proof.compute_expected_proof_length(),
                indexed_merkle_proof.merkle_proof().len() as u64
            );
            assert_eq!(indexed_merkle_proof.verify(), Ok(()));
            assert_eq!(leaf_count, indexed_merkle_proof.count);
            assert_eq!(leaves[index as usize], indexed_merkle_proof.merkle_proof[0]);
            assert_eq!(root, indexed_merkle_proof.root_hash());
        }
    }

    #[test]
    fn out_of_bounds_index() {
        let out_of_bounds_indexed_merkle_proof = IndexedMerkleProof {
            index: 23,
            count: 4,
            merkle_proof: vec![Blake2bHash([0u8; 32]); 3],
        };
        assert_eq!(
            out_of_bounds_indexed_merkle_proof.verify(),
            Err(MerkleVerificationError::IndexOutOfBounds {
                count: 4,
                index: 23
            })
        )
    }

    #[test]
    fn unexpected_proof_length() {
        let out_of_bounds_indexed_merkle_proof = IndexedMerkleProof {
            index: 1235,
            count: 5647,
            merkle_proof: vec![Blake2bHash([0u8; 32]); 13],
        };
        assert_eq!(
            out_of_bounds_indexed_merkle_proof.verify(),
            Err(MerkleVerificationError::UnexpectedProofLength {
                count: 5647,
                index: 1235,
                expected_proof_length: 14,
                actual_proof_length: 13
            })
        )
    }

    #[test]
    fn empty_unexpected_proof_length() {
        let out_of_bounds_indexed_merkle_proof = IndexedMerkleProof {
            index: 0,
            count: 0,
            merkle_proof: vec![Blake2bHash([0u8; 32]); 3],
        };
        assert_eq!(
            out_of_bounds_indexed_merkle_proof.verify(),
            Err(MerkleVerificationError::UnexpectedProofLength {
                count: 0,
                index: 0,
                expected_proof_length: 0,
                actual_proof_length: 3
            })
        )
    }

    #[test]
    fn empty_out_of_bounds_index() {
        let out_of_bounds_indexed_merkle_proof = IndexedMerkleProof {
            index: 23,
            count: 0,
            merkle_proof: vec![],
        };
        assert_eq!(
            out_of_bounds_indexed_merkle_proof.verify(),
            Err(MerkleVerificationError::IndexOutOfBounds {
                count: 0,
                index: 23
            })
        )
    }

    #[test]
    fn deep_proof_doesnt_kill_stack() {
        const PROOF_LENGTH: usize = 63;
        let indexed_merkle_proof = IndexedMerkleProof {
            index: 42,
            count: 1 << (PROOF_LENGTH - 1),
            merkle_proof: vec![Blake2bHash([0u8; Digest::LENGTH]); PROOF_LENGTH],
        };
        let _hash = indexed_merkle_proof.root_hash();
    }

    #[test]
    fn empty_proof() {
        let empty_merkle_root = hash_merkle_tree(vec![]);
        assert_eq!(empty_merkle_root, hash_pair(0u64.to_le_bytes(), SENTINEL2));
        let indexed_merkle_proof = IndexedMerkleProof {
            index: 0,
            count: 0,
            merkle_proof: vec![],
        };
        assert_eq!(indexed_merkle_proof.verify(), Ok(()));
        assert_eq!(indexed_merkle_proof.root_hash(), empty_merkle_root);
    }

    #[proptest]
    fn expected_proof_length_le_65(index: u64, count: u64) {
        let indexed_merkle_proof = IndexedMerkleProof {
            index,
            count,
            merkle_proof: vec![],
        };
        prop_assert!(indexed_merkle_proof.compute_expected_proof_length() <= 65);
    }
}
