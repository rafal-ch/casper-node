//! Types used to allow creation of Wasm contracts and tests for use on the Casper Platform.
 
// TODO[RC]: Describe
#![feature(specialization)]

#![cfg_attr(
    not(any(
        feature = "json-schema",
        feature = "datasize",
        feature = "crypto-std",
        feature = "std",
        feature = "testing",
        test,
    )),
    no_std
)]
#![doc(html_root_url = "https://docs.rs/casper-types/1.5.0")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/CasperLabs/casper-node/master/images/CasperLabs_Logo_Favicon_RGB_50px.png",
    html_logo_url = "https://raw.githubusercontent.com/CasperLabs/casper-node/master/images/CasperLabs_Logo_Symbol_RGB.png",
    test(attr(forbid(warnings)))
)]
#![warn(missing_docs)]

#[cfg_attr(not(test), macro_use)]
extern crate alloc;

mod access_rights;
pub mod account;
pub mod api_error;
mod block_time;
pub mod bytesrepr;
pub mod checksummed_hex;
mod cl_type;
mod cl_value;
mod contract_wasm;
pub mod contracts;
pub mod crypto;
mod deploy_info;
mod era_id;
mod execution_result;
#[cfg(any(feature = "crypto-std", test))]
pub mod file_utils;
mod gas;
#[cfg(any(feature = "testing", test))]
pub mod gens;
mod json_pretty_printer;
mod key;
mod motes;
mod named_key;
mod phase;
mod protocol_version;
pub mod runtime_args;
mod semver;
mod stored_value;
pub mod system;
mod tagged;
#[cfg(any(feature = "testing", test))]
pub mod testing;
mod transfer;
mod transfer_result;
mod uint;
mod uref;

#[cfg(feature = "impl-from-bytes")]
mod impl_from_bytes;
#[cfg(feature = "impl-to-bytes")]
mod impl_to_bytes;

use alloc::alloc::alloc;
use bytesrepr::FromBytes;
use core::{alloc::Layout, any, mem, ptr::NonNull};

pub use access_rights::{
    AccessRights, ContextAccessRights, GrantedAccess, ACCESS_RIGHTS_SERIALIZED_LENGTH,
};
#[doc(inline)]
pub use api_error::ApiError;
pub use block_time::{BlockTime, BLOCKTIME_SERIALIZED_LENGTH};
pub use cl_type::{named_key_type, CLType, CLTyped};
pub use cl_value::{CLTypeMismatch, CLValue, CLValueError};
pub use contract_wasm::{ContractWasm, ContractWasmHash};
#[doc(inline)]
pub use contracts::{
    Contract, ContractHash, ContractPackage, ContractPackageHash, ContractVersion,
    ContractVersionKey, EntryPoint, EntryPointAccess, EntryPointType, EntryPoints, Group,
    Parameter,
};
pub use crypto::*;
pub use deploy_info::DeployInfo;
pub use execution_result::{
    ExecutionEffect, ExecutionResult, OpKind, Operation, Transform, TransformEntry,
};
pub use gas::Gas;
pub use json_pretty_printer::json_pretty_print;
#[doc(inline)]
pub use key::{
    DictionaryAddr, FromStrError as KeyFromStrError, HashAddr, Key, KeyTag, BLAKE2B_DIGEST_LENGTH,
    DICTIONARY_ITEM_KEY_MAX_LENGTH, KEY_DICTIONARY_LENGTH, KEY_HASH_LENGTH,
};
pub use motes::Motes;
pub use named_key::NamedKey;
pub use phase::{Phase, PHASE_SERIALIZED_LENGTH};
pub use protocol_version::{ProtocolVersion, VersionCheckResult};
#[doc(inline)]
pub use runtime_args::{NamedArg, RuntimeArgs};
pub use semver::{ParseSemVerError, SemVer, SEM_VER_SERIALIZED_LENGTH};
pub use stored_value::{StoredValue, TypeMismatch as StoredValueTypeMismatch};
pub use tagged::Tagged;
pub use transfer::{
    DeployHash, FromStrError as TransferFromStrError, Transfer, TransferAddr, DEPLOY_HASH_LENGTH,
    TRANSFER_ADDR_LENGTH,
};
pub use transfer_result::{TransferResult, TransferredTo};
pub use uref::{
    FromStrError as URefFromStrError, URef, URefAddr, UREF_ADDR_LENGTH, UREF_SERIALIZED_LENGTH,
};

pub use crate::{
    era_id::EraId,
    uint::{UIntParseError, U128, U256, U512},
};




use alloc::vec::Vec;


// TODO[RC]: Deduplicate this and adjacent functions
fn ensure_efficient_serialization<T>() {
    #[cfg(debug_assertions)]
    debug_assert_ne!(
        any::type_name::<T>(),
        any::type_name::<u8>(),
        "You should use Bytes newtype wrapper for efficiency"
    );
}

// TODO Replace `try_vec_with_capacity` with `Vec::try_reserve_exact` once it's in stable.
fn try_vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, bytesrepr::Error> {
    // see https://doc.rust-lang.org/src/alloc/raw_vec.rs.html#75-98
    let elem_size = mem::size_of::<T>();
    let alloc_size = capacity.checked_mul(elem_size).ok_or(bytesrepr::Error::OutOfMemory)?;

    let ptr = if alloc_size == 0 {
        NonNull::<T>::dangling()
    } else {
        let align = mem::align_of::<T>();
        let layout = Layout::from_size_align(alloc_size, align).map_err(|_| bytesrepr::Error::OutOfMemory)?;
        let raw_ptr = unsafe { alloc(layout) };
        let non_null_ptr = NonNull::<u8>::new(raw_ptr).ok_or(bytesrepr::Error::OutOfMemory)?;
        non_null_ptr.cast()
    };
    unsafe { Ok(Vec::from_raw_parts(ptr.as_ptr(), 0, capacity)) }
}

fn vec_from_vec<T: FromBytes>(bytes: Vec<u8>) -> Result<(Vec<T>, Vec<u8>), bytesrepr::Error> {
    ensure_efficient_serialization::<T>();

    Vec::<T>::from_bytes(bytes.as_slice()).map(|(x, remainder)| (x, Vec::from(remainder)))
}

impl<T: FromBytes> FromBytes for Vec<T> {
    default fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        ensure_efficient_serialization::<T>();

        let (count, mut stream) = u32::from_bytes(bytes)?;

        let mut result = try_vec_with_capacity(count as usize)?;
        for _ in 0..count {
            let (value, remainder) = T::from_bytes(stream)?;
            result.push(value);
            stream = remainder;
        }

        Ok((result, stream))
    }

    default fn from_vec(bytes: Vec<u8>) -> Result<(Self, Vec<u8>), bytesrepr::Error> {
        vec_from_vec(bytes)
    }
}
