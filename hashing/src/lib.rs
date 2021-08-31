#![cfg_attr(not(feature = "std"), no_std)]

mod chunk;
mod error;
pub mod primitives;

/// The number of bytes in a Blake2b hash
use core::{array::TryFromSliceError, convert::TryFrom};

#[cfg(all(feature = "no-std", no_std))]
use alloc::vec::Vec;
#[cfg(all(features = "std", not(no_std)))]
use std::vec::Vec;

use blake2::{
    digest::{Update, VariableOutput},
    VarBlake2b,
};

pub use error::{MerkleConstructionError, MerkleVerificationError};

#[cfg(feature = "std")]
use hex_buffer_serde::{Hex, HexForm};

/// The hash digest; a wrapped `u8` array.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Default)]
#[cfg_attr(
    feature = "std",
    derive(
        schemars::JsonSchema,
        serde::Serialize,
        serde::Deserialize,
        datasize::DataSize
    ),
    schemars(with = "String", description = "Hex-encoded hash digest."),
    serde(deny_unknown_fields)
)]
pub struct Digest(
    #[cfg_attr(
        feature = "std",
        schemars(skip, with = "String"),
        serde(with = "HexForm::<[u8; Digest::LENGTH]>")
    )]
    [u8; Digest::LENGTH],
);

impl Digest {
    pub const LENGTH: usize = 32;

    /// Creates a 32-byte hash digest from a given a piece of data
    pub fn hash<T: AsRef<[u8]>>(data: T) -> Digest {
        let mut result = [0; Digest::LENGTH];

        let mut hasher = VarBlake2b::new(Digest::LENGTH).expect("should create hasher");
        hasher.update(data);
        hasher.finalize_variable(|slice| {
            result.copy_from_slice(slice);
        });
        Digest(result)
    }

    /// Hashes a pair of byte slices into a single [`Digest`]
    pub fn hash_pair<T: AsRef<[u8]>, U: AsRef<[u8]>>(data1: T, data2: U) -> Digest {
        let mut result = [0; Digest::LENGTH];
        let mut hasher = VarBlake2b::new(Digest::LENGTH).unwrap();
        hasher.update(data1);
        hasher.update(data2);
        hasher.finalize_variable(|slice| {
            result.copy_from_slice(slice);
        });
        Digest(result)
    }

    /// Returns a reference to the underlying value.
    pub fn inner(&self) -> &[u8; Digest::LENGTH] {
        &self.0
    }

    /// Returns the underlying value.
    pub fn value(&self) -> [u8; Digest::LENGTH] {
        self.0
    }

    /// Converts the underlying hash digest array to a `Vec`
    #[cfg(feature = "std")]
    pub fn into_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Returns a `Digest` parsed from a hex-encoded `Digest`.
    #[cfg(feature = "std")]
    pub fn from_hex<T: AsRef<[u8]>>(hex_input: T) -> Result<Self, hex::FromHexError> {
        let mut inner = [0; Digest::LENGTH];
        hex::decode_to_slice(hex_input, &mut inner)?;
        Ok(Digest(inner))
    }
}

#[cfg(feature = "std")]
impl std::fmt::LowerHex for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let hex_string = base16::encode_lower(self.inner());
        if f.alternate() {
            write!(f, "0x{}", hex_string)
        } else {
            write!(f, "{}", hex_string)
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::UpperHex for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let hex_string = base16::encode_upper(self.inner());
        if f.alternate() {
            write!(f, "0x{}", hex_string)
        } else {
            write!(f, "{}", hex_string)
        }
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:10}", base16::encode_lower(&self.0))
    }
}

#[cfg(feature = "std")]
impl std::fmt::Debug for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<[u8; Digest::LENGTH]> for Digest {
    fn from(arr: [u8; Digest::LENGTH]) -> Self {
        Digest(arr)
    }
}

impl<'a> TryFrom<&'a [u8]> for Digest {
    type Error = TryFromSliceError;

    fn try_from(slice: &[u8]) -> Result<Digest, Self::Error> {
        <[u8; Digest::LENGTH]>::try_from(slice).map(Digest)
    }
}

impl AsRef<[u8]> for Digest {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<Digest> for [u8; Digest::LENGTH] {
    fn from(hash: Digest) -> Self {
        hash.0
    }
}

#[cfg(feature = "std")]
impl hex::FromHex for Digest {
    type Error = hex::FromHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        Ok(Digest(hex::FromHex::from_hex(hex)?))
    }
}

#[cfg(test)]
mod test {
    use std::iter;

    use super::*;

    #[test]
    fn blake2b_hash_known() {
        let inputs_and_digests = [
            (
                "",
                "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8",
            ),
            (
                "abc",
                "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319",
            ),
            (
                "The quick brown fox jumps over the lazy dog",
                "01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9",
            ),
        ];
        for (known_input, expected_digest) in &inputs_and_digests {
            let known_input: &[u8] = known_input.as_ref();
            assert_eq!(*expected_digest, format!("{:?}", Digest::hash(known_input)));
        }
    }

    #[test]
    fn from_valid_hex_should_succeed() {
        for char in "abcdefABCDEF0123456789".chars() {
            let input: String = iter::repeat(char).take(64).collect();
            assert!(Digest::from_hex(input).is_ok());
        }
    }

    #[test]
    fn from_hex_invalid_length_should_fail() {
        for len in &[2_usize, 62, 63, 65, 66] {
            let input: String = "f".repeat(*len);
            assert!(Digest::from_hex(input).is_err());
        }
    }

    #[test]
    fn from_hex_invalid_char_should_fail() {
        for char in "g %-".chars() {
            let input: String = iter::repeat('f').take(63).chain(iter::once(char)).collect();
            assert!(Digest::from_hex(input).is_err());
        }
    }

    #[test]
    fn should_display_digest_in_hex() {
        let hash = Digest([0u8; 32]);
        let hash_hex = format!("{:?}", hash);
        assert_eq!(
            hash_hex,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn should_print_digest_lower_hex() {
        let hash = Digest([10u8; 32]);
        let hash_lower_hex = format!("{:x}", hash);
        assert_eq!(
            hash_lower_hex,
            "0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a"
        )
    }

    #[test]
    fn should_print_digest_upper_hex() {
        let hash = Digest([10u8; 32]);
        let hash_lower_hex = format!("{:X}", hash);
        assert_eq!(
            hash_lower_hex,
            "0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A"
        )
    }

    #[test]
    fn alternate_should_prepend_0x() {
        let hash = Digest([0u8; 32]);
        let hash_hex_alt = format!("{:#x}", hash);
        assert_eq!(
            hash_hex_alt,
            "0x0000000000000000000000000000000000000000000000000000000000000000"
        )
    }
}
