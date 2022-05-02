// TODO - remove once schemars stops causing warning.
#![allow(clippy::field_reassign_with_default)]

use alloc::string::String;
use alloc::alloc::alloc;
use alloc::alloc::Layout;
use alloc::vec::Vec;
use core::ptr::NonNull;
use crate::bytesrepr::Error;
use core::any;
use core::mem;

#[cfg(feature = "derive-from-bytes")]
use bytesrepr_derive::BytesreprDeserialize;
#[cfg(feature = "derive-to-bytes")]
use bytesrepr_derive::BytesreprSerialize;
#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bytesrepr::{FromBytes, ToBytes};

/// A named key.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Default, Debug)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[cfg_attr(feature = "derive-to-bytes", derive(BytesreprSerialize))]
#[cfg_attr(feature = "derive-from-bytes", derive(BytesreprDeserialize))]
#[serde(deny_unknown_fields)]
pub struct NamedKey {
    /// The name of the entry.
    pub name: String,
    /// The value of the entry: a casper `Key` type.
    pub key: String,
}
