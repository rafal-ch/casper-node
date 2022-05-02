use alloc::{
    alloc::{alloc, Layout},
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
    bytesrepr::{Error, ToBytes, U32_SERIALIZED_LENGTH},
    NamedKey,
};
fn iterator_serialized_length<'a, T: 'a + ToBytes>(ts: impl Iterator<Item = &'a T>) -> usize {
    U32_SERIALIZED_LENGTH + ts.map(ToBytes::serialized_length).sum::<usize>()
}

fn ensure_efficient_serialization<T>() {
    #[cfg(debug_assertions)]
    debug_assert_ne!(
        any::type_name::<T>(),
        any::type_name::<u8>(),
        "You should use Bytes newtype wrapper for efficiency"
    );
}

impl ToBytes for NamedKey {
    fn to_bytes(&self) -> Result<Vec<u8>, crate::bytesrepr::Error> {
        let mut buffer = crate::bytesrepr::allocate_buffer(self)?;
        buffer.extend(self.name.to_bytes()?);
        buffer.extend(self.key.to_bytes()?);
        Ok(buffer)
    }

    fn serialized_length(&self) -> usize {
        self.name.serialized_length() + self.key.serialized_length()
    }
}

/// Returns a `Vec<u8>` initialized with sufficient capacity to hold `to_be_serialized` after
/// serialization, or an error if the capacity would exceed `u32::max_value()`.
pub fn allocate_buffer<T: ToBytes>(to_be_serialized: &T) -> Result<Vec<u8>, Error> {
    let serialized_length = to_be_serialized.serialized_length();
    if serialized_length > u32::max_value() as usize {
        return Err(Error::OutOfMemory);
    }
    Ok(Vec::with_capacity(serialized_length))
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

impl<T: ToBytes> ToBytes for Vec<T> {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        ensure_efficient_serialization::<T>();

        let mut result = try_vec_with_capacity(self.serialized_length())?;
        let length_32: u32 = self.len().try_into().map_err(|_| Error::NotRepresentable)?;
        result.append(&mut length_32.to_bytes()?);

        for item in self.iter() {
            result.append(&mut item.to_bytes()?);
        }

        Ok(result)
    }

    fn into_bytes(self) -> Result<Vec<u8>, Error> {
        ensure_efficient_serialization::<T>();

        let mut result = allocate_buffer(&self)?;
        let length_32: u32 = self.len().try_into().map_err(|_| Error::NotRepresentable)?;
        result.append(&mut length_32.to_bytes()?);

        for item in self {
            result.append(&mut item.into_bytes()?);
        }

        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        iterator_serialized_length(self.iter())
    }

    fn write_bytes(&self, writer: &mut Vec<u8>) -> Result<(), Error> {
        let length_32: u32 = self.len().try_into().map_err(|_| Error::NotRepresentable)?;
        writer.extend_from_slice(&length_32.to_le_bytes());
        for item in self.iter() {
            item.write_bytes(writer)?;
        }
        Ok(())
    }
}
