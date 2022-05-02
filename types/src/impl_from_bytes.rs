use alloc::{
    alloc::{alloc, Layout},
    string::String,
    vec::Vec,
};
use core::{
    any,
    convert::TryInto,
    fmt::{self, Display, Formatter},
    mem,
    ptr::NonNull,
};

use crate::{
    bytesrepr::{Error, FromBytes},
    NamedKey,
};

impl FromBytes for NamedKey {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), crate::bytesrepr::Error> {
        let (name, remainder) = String::from_bytes(bytes)?;
        let (key, remainder) = String::from_bytes(remainder)?;
        let named_key = NamedKey { name, key };
        Ok((named_key, remainder))
    }
}

impl FromBytes for Account {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (account_hash, rem) = AccountHash::from_bytes(bytes)?;
        let (named_keys, rem) = NamedKeys::from_bytes(rem)?;
        let (main_purse, rem) = URef::from_bytes(rem)?;
        let (associated_keys, rem) = AssociatedKeys::from_bytes(rem)?;
        let (action_thresholds, rem) = ActionThresholds::from_bytes(rem)?;
        Ok((
            Account {
                account_hash,
                named_keys,
                main_purse,
                associated_keys,
                action_thresholds,
            },
            rem,
        ))
    }
}


// TODO[RC]: Deduplicate this and adjacent functions
fn ensure_efficient_serialization THIS IS HERE TO MAKE SURE THIS FILE IS NOT COMPILED <T>() {
    #[cfg(debug_assertions)]
    debug_assert_ne!(
        any::type_name::<T>(),
        any::type_name::<u8>(),
        "You should use Bytes newtype wrapper for efficiency"
    );
}

// TODO Replace `try_vec_with_capacity` with `Vec::try_reserve_exact` once it's in stable.
fn try_vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, Error> {
    // see https://doc.rust-lang.org/src/alloc/raw_vec.rs.html#75-98
    let elem_size = mem::size_of::<T>();
    let alloc_size = capacity.checked_mul(elem_size).ok_or(Error::OutOfMemory)?;

    let ptr = if alloc_size == 0 {
        NonNull::<T>::dangling()
    } else {
        let align = mem::align_of::<T>();
        let layout = Layout::from_size_align(alloc_size, align).map_err(|_| Error::OutOfMemory)?;
        let raw_ptr = unsafe { alloc(layout) };
        let non_null_ptr = NonNull::<u8>::new(raw_ptr).ok_or(Error::OutOfMemory)?;
        non_null_ptr.cast()
    };
    unsafe { Ok(Vec::from_raw_parts(ptr.as_ptr(), 0, capacity)) }
}

fn vec_from_vec<T: FromBytes>(bytes: Vec<u8>) -> Result<(Vec<T>, Vec<u8>), Error> {
    ensure_efficient_serialization::<T>();

    Vec::<T>::from_bytes(bytes.as_slice()).map(|(x, remainder)| (x, Vec::from(remainder)))
}

impl<T: FromBytes> FromBytes for Vec<T> {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        ensure_efficient_serialization::<T>();

        let (mut count, mut stream) = u32::from_bytes(bytes)?;

        let mut result = try_vec_with_capacity(count as usize)?;
        for _ in 0..count {
            let (value, remainder) = T::from_bytes(stream)?;
            result.push(value);
            stream = remainder;
        }

        Ok((result, stream))
    }

    fn from_vec(bytes: Vec<u8>) -> Result<(Self, Vec<u8>), Error> {
        vec_from_vec(bytes)
    }
}
