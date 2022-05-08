//! LMDB extensions.
//!
//! Various traits and helper functions to extend the lower level LMDB functions. Unifies
//! lower-level storage errors from lmdb and serialization issues.
//!
//! ## Serialization
//!
//! The module also centralizes settings and methods for serialization for all parts of storage.
//!
//! Serialization errors are unified into a generic, type erased `std` error to allow for easy
//! interchange of the serialization format if desired.

use std::any::TypeId;

use lmdb::{Database, RwTransaction, Transaction, WriteFlags};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

use casper_types::{
    bytesrepr::{self, FromBytes, ToBytes},
    system::auction::UnbondingPurse,
    UREF_SERIALIZED_LENGTH,
};

/// Error wrapper for lower-level storage errors.
///
/// Used to classify storage errors, allowing more accurate reporting on potential issues and
/// crashes. Indicates how to proceed (clearing storage entirely or just restarting) in most cases.
///
/// Note that accessing a storage with an incompatible version of this software is also considered a
/// case of corruption.
#[derive(Debug, Error)]
pub enum LmdbExtError {
    /// The internal database is corrupted and can probably not be salvaged.
    #[error("internal storage corrupted: {0}")]
    LmdbCorrupted(lmdb::Error),
    /// The data stored inside the internal database is corrupted or formatted wrong.
    #[error("internal data corrupted: {0}")]
    DataCorrupted(Box<dyn std::error::Error + Send + Sync>),
    /// A resource has been exhausted at runtime, restarting (potentially with different settings)
    /// might fix the problem. Storage integrity is still intact.
    #[error("storage exhausted resource (but still intact): {0}")]
    ResourceExhausted(lmdb::Error),
    /// Error neither corruption nor resource exhaustion occurred, likely a programming error.
    #[error("unknown LMDB or serialization error, likely from a bug: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct BytesreprError(pub bytesrepr::Error);

// Classifies an `lmdb::Error` according to our scheme. This one of the rare cases where we accept a
// blanked `From<>` implementation for error type conversion.
impl From<lmdb::Error> for LmdbExtError {
    fn from(lmdb_error: lmdb::Error) -> Self {
        match lmdb_error {
            lmdb::Error::PageNotFound
            | lmdb::Error::Corrupted
            | lmdb::Error::Panic
            | lmdb::Error::VersionMismatch
            | lmdb::Error::Invalid
            | lmdb::Error::Incompatible => LmdbExtError::LmdbCorrupted(lmdb_error),

            lmdb::Error::MapFull
            | lmdb::Error::DbsFull
            | lmdb::Error::ReadersFull
            | lmdb::Error::TlsFull
            | lmdb::Error::TxnFull
            | lmdb::Error::CursorFull
            | lmdb::Error::PageFull
            | lmdb::Error::MapResized => LmdbExtError::ResourceExhausted(lmdb_error),

            lmdb::Error::NotFound
            | lmdb::Error::BadRslot
            | lmdb::Error::BadTxn
            | lmdb::Error::BadValSize
            | lmdb::Error::BadDbi
            | lmdb::Error::KeyExist
            | lmdb::Error::Other(_) => LmdbExtError::Other(Box::new(lmdb_error)),
        }
    }
}

/// Additional methods on transaction.
pub(super) trait TransactionExt {
    /// Helper function to load a value from a database.
    fn get_value<K: AsRef<[u8]>, V: 'static + DeserializeOwned>(
        &mut self,
        db: Database,
        key: &K,
    ) -> Result<Option<V>, LmdbExtError>;

    /// Helper function to load a value from a database using the `bytesrepr` `ToBytes`/`FromBytes`
    /// serialization.
    fn get_value_bytesrepr<K: AsRef<[u8]>, V: FromBytes>(
        &mut self,
        db: Database,
        key: &K,
    ) -> Result<Option<V>, LmdbExtError>;
}

/// Additional methods on write transactions.
pub(super) trait WriteTransactionExt {
    /// Helper function to write a value to a database.
    ///
    /// Returns `true` if the value has actually been written, `false` if the key already existed.
    ///
    /// Setting `overwrite` to true will cause the value to always be written instead.
    fn put_value<K: AsRef<[u8]>, V: 'static + Serialize>(
        &mut self,
        db: Database,
        key: &K,
        value: &V,
        overwrite: bool,
    ) -> Result<bool, LmdbExtError>;

    /// Helper function to write a value to a database using the `bytesrepr` `ToBytes`/`FromBytes`
    /// serialization.
    ///
    /// Returns `true` if the value has actually been written, `false` if the key already existed.
    ///
    /// Setting `overwrite` to true will cause the value to always be written instead.
    fn put_value_bytesrepr<K: AsRef<[u8]>, V: ToBytes>(
        &mut self,
        db: Database,
        key: &K,
        value: &V,
        overwrite: bool,
    ) -> Result<bool, LmdbExtError>;
}

impl<T> TransactionExt for T
where
    T: Transaction,
{
    #[inline]
    fn get_value<K: AsRef<[u8]>, V: 'static + DeserializeOwned>(
        &mut self,
        db: Database,
        key: &K,
    ) -> Result<Option<V>, LmdbExtError> {
        match self.get(db, key) {
            // Deserialization failures are likely due to storage corruption.
            Ok(raw) => deserialize_internal(raw),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    #[inline]
    fn get_value_bytesrepr<K: AsRef<[u8]>, V: FromBytes>(
        &mut self,
        db: Database,
        key: &K,
    ) -> Result<Option<V>, LmdbExtError> {
        match self.get(db, key) {
            // Deserialization failures are likely due to storage corruption.
            Ok(raw) => deserialize_bytesrepr(raw).map(Some),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}

/// Serializes `value` into the buffer.
/// In case the `value` is of the `UnbondingPurse` type it uses the specialized
/// function to provide compatibility with the legacy version of the `UnbondingPurse` struct.
/// See [`serialize_unbonding_purse`] for more details.
pub(crate) fn serialize_internal<V: 'static + Serialize>(
    value: &V,
) -> Result<Vec<u8>, LmdbExtError> {
    let buffer = if TypeId::of::<UnbondingPurse>() == TypeId::of::<V>() {
        serialize_unbonding_purse(value)?
    } else {
        serialize(value)?
    };
    Ok(buffer)
}

/// Deserializes an object from the raw bytes.
/// In case the expected object is of the `UnbondingPurse` type it uses the specialized
/// function to provide compatibility with the legacy version of the `UnbondingPurse` struct.
/// See [`deserialize_unbonding_purse`] for more details.
pub(crate) fn deserialize_internal<V: 'static + DeserializeOwned>(
    raw: &[u8],
) -> Result<Option<V>, LmdbExtError> {
    if TypeId::of::<UnbondingPurse>() == TypeId::of::<V>() {
        deserialize_unbonding_purse(raw).map(Some)
    } else {
        deserialize(raw).map(Some)
    }
}

impl WriteTransactionExt for RwTransaction<'_> {
    fn put_value<K: AsRef<[u8]>, V: 'static + Serialize>(
        &mut self,
        db: Database,
        key: &K,
        value: &V,
        overwrite: bool,
    ) -> Result<bool, LmdbExtError> {
        let buffer = serialize_internal(value)?;

        let flags = if overwrite {
            WriteFlags::empty()
        } else {
            WriteFlags::NO_OVERWRITE
        };

        match self.put(db, key, &buffer, flags) {
            Ok(()) => Ok(true),
            // If we did not add the value due to it already existing, just return `false`.
            Err(lmdb::Error::KeyExist) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    fn put_value_bytesrepr<K: AsRef<[u8]>, V: ToBytes>(
        &mut self,
        db: Database,
        key: &K,
        value: &V,
        overwrite: bool,
    ) -> Result<bool, LmdbExtError> {
        let buffer = serialize_bytesrepr(value)?;

        let flags = if overwrite {
            WriteFlags::empty()
        } else {
            WriteFlags::NO_OVERWRITE
        };

        match self.put(db, key, &buffer, flags) {
            Ok(()) => Ok(true),
            // If we did not add the value due to it already existing, just return `false`.
            Err(lmdb::Error::KeyExist) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }
}

/// Deserializes from a buffer.
#[inline(always)]
pub(super) fn deserialize<T: DeserializeOwned>(raw: &[u8]) -> Result<T, LmdbExtError> {
    bincode::deserialize(raw).map_err(|err| LmdbExtError::DataCorrupted(Box::new(err)))
}

/// Returns `true` if the specified bytes represent the legacy version of `UnbondingPurse`.
fn is_legacy(raw: &[u8]) -> bool {
    if raw.len() <= UREF_SERIALIZED_LENGTH {
        // TODO[RC]: Error here?
        return true;
    }

    !(raw[UREF_SERIALIZED_LENGTH] & 0b10000000 == 0b10000000)
}

/// Deserializes `UnbondingPurse` from a buffer.
/// TODO[RC]: Proper comment
pub(super) fn deserialize_unbonding_purse<T: DeserializeOwned>(
    raw: &[u8],
) -> Result<T, LmdbExtError> {
    const BINCODE_ENCODED_NONE: [u8; 4] = [0; 4];
    if is_legacy(raw) {
        deserialize(&[raw, &BINCODE_ENCODED_NONE].concat())
    } else {
        deserialize(&eject_magic_byte(raw)?)
    }
}

/// Serializes into a buffer.
#[inline(always)]
pub(super) fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, LmdbExtError> {
    bincode::serialize(value).map_err(|err| LmdbExtError::Other(Box::new(err)))
}

/// Serializes `UnbondingPurse` into a buffer.
/// TODO[RC]: Proper comment
#[inline(always)]
pub(super) fn serialize_unbonding_purse<T: Serialize>(value: &T) -> Result<Vec<u8>, LmdbExtError> {
    let mut raw_bytes =
        bincode::serialize(value).map_err(|err| LmdbExtError::Other(Box::new(err)))?;
    inject_magic_byte(&mut raw_bytes)?;
    Ok(raw_bytes)
}

fn eject_magic_byte(raw: &[u8]) -> Result<Vec<u8>, LmdbExtError> {
    if raw.len() <= UREF_SERIALIZED_LENGTH {
        return Err(LmdbExtError::Other(
            "The raw bytes don't represent `UnbondingPurse`, can't eject magic byte".into(),
        ));
    }

    let mut bytes: Vec<_> = raw.iter().copied().collect();
    bytes[UREF_SERIALIZED_LENGTH] ^= 0b10000000;
    Ok(bytes)
}

// TODO[RC]: Add comment, UTs...
fn inject_magic_byte(raw: &mut [u8]) -> Result<(), LmdbExtError> {
    if raw.len() <= UREF_SERIALIZED_LENGTH {
        return Err(LmdbExtError::Other(
            "The raw bytes don't represent `UnbondingPurse`, can't inject magic byte".into(),
        ));
    }

    raw[UREF_SERIALIZED_LENGTH] |= 0b10000000;
    Ok(())
}

/// Deserializes from a buffer.
#[inline(always)]
pub(super) fn deserialize_bytesrepr<T: FromBytes>(raw: &[u8]) -> Result<T, LmdbExtError> {
    T::from_bytes(raw)
        .map(|val| val.0)
        .map_err(|err| LmdbExtError::DataCorrupted(Box::new(BytesreprError(err))))
}

/// Serializes into a buffer.
#[inline(always)]
pub(super) fn serialize_bytesrepr<T: ToBytes>(value: &T) -> Result<Vec<u8>, LmdbExtError> {
    value
        .to_bytes()
        .map_err(|err| LmdbExtError::Other(Box::new(BytesreprError(err))))
}
