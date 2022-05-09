// TODO - remove once schemars stops causing warning.
#![allow(clippy::field_reassign_with_default)]

use alloc::{collections::BTreeMap, string::String, vec::Vec};

#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bytesrepr::{self, Bytes, FromBytes, ToBytes};

const MAGIC_BYTES: &[u8] = &[121, 17, 133, 179, 91, 63, 69, 222];

/// A named key.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Default, Debug)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NamedKeyV1 {
    /// The name of the entry.
    pub name: String,
    /// The value of the entry: a casper `Key` type.
    pub key: String,
}

/// A named key.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Default, Debug)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NamedKeyV2 {
    /// The name of the entry.
    pub name: String,
    /// The value of the entry: a casper `Key` type.
    pub key: String,
    /// The brand new, additional field
    pub id: u64,
}

#[derive(Debug, PartialEq)]
pub enum NamedKey {
    V1(NamedKeyV1),
    V2(NamedKeyV2),
}

impl ToBytes for NamedKeyV1 {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut buffer = bytesrepr::allocate_buffer(self)?;
        buffer.extend(self.name.to_bytes()?);
        buffer.extend(self.key.to_bytes()?);
        Ok(buffer)
    }

    fn serialized_length(&self) -> usize {
        self.name.serialized_length() + self.key.serialized_length()
    }
}

impl FromBytes for NamedKeyV1 {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (name, remainder) = String::from_bytes(bytes)?;
        let (key, remainder) = String::from_bytes(remainder)?;
        let named_key = NamedKeyV1 { name, key };
        Ok((named_key, remainder))
    }
}

impl FromBytes for NamedKeyV2 {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (name, remainder) = String::from_bytes(bytes)?;
        let (key, remainder) = String::from_bytes(remainder)?;
        let (id, remainder) = u64::from_bytes(remainder)?;
        let named_key = NamedKeyV2 { name, key, id };
        Ok((named_key, remainder))
    }
}

impl ToBytes for NamedKey {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let obj = match self {
            NamedKey::V1(v1) => NamedKeyV2 {
                name: v1.name.clone(),
                key: v1.key.clone(),
                id: Default::default(),
            },
            NamedKey::V2(v2) => v2.clone(),
        };
        let mut buffer = bytesrepr::allocate_buffer(self)?;
        buffer.extend(MAGIC_BYTES);
        buffer.extend(obj.name.to_bytes()?);
        buffer.extend(obj.key.to_bytes()?);
        buffer.extend(obj.id.to_bytes()?);
        Ok(buffer)
    }

    fn serialized_length(&self) -> usize {
        match self {
            NamedKey::V1(_) => unreachable!("should not be invoked"),
            NamedKey::V2(obj) => {
                obj.name.serialized_length()
                    + obj.key.serialized_length()
                    + obj.id.serialized_length()
                    + MAGIC_BYTES.len()
            }
        }
    }
}

impl FromBytes for NamedKey {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        #[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Default, Debug)]
        #[cfg_attr(feature = "datasize", derive(DataSize))]
        #[cfg_attr(feature = "json-schema", derive(JsonSchema))]
        #[serde(deny_unknown_fields)]
        pub struct NamedKeyLegacy {
            /// The name of the entry.
            pub name: String,
            /// The value of the entry: a casper `Key` type.
            pub key: String,
        }
        impl FromBytes for NamedKeyLegacy {
            fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
                let (name, remainder) = String::from_bytes(bytes)?;
                let (key, remainder) = String::from_bytes(remainder)?;
                let named_key_legacy = NamedKeyLegacy { name, key };
                Ok((named_key_legacy, remainder))
            }
        }

        if is_legacy(bytes) {
            let (legacy_named_key, remainder) = NamedKeyLegacy::from_bytes(&bytes)?;
            Ok((
                NamedKey::V2(NamedKeyV2 {
                    name: legacy_named_key.name,
                    key: legacy_named_key.key,
                    ..Default::default()
                }),
                remainder,
            ))
        } else {
            let (v2, remainder) = NamedKeyV2::from_bytes(&bytes[MAGIC_BYTES.len()..])?;
            Ok((NamedKey::V2(v2), remainder))
        }
    }
}

fn is_legacy(raw: &[u8]) -> bool {
    !raw.starts_with(MAGIC_BYTES)
}
#[cfg(test)]
mod tests {
    use rand::{distributions::Alphanumeric, Rng};

    use crate::{
        bytesrepr::{FromBytes, ToBytes},
        named_key::{NamedKey, NamedKeyV2},
        NamedKeyV1,
    };

    const HEX_ENCODED_LEGACY_NAMED_KEY: &str =
        "0f0000006c65676163795f6b65795f6e616d65100000006c65676163795f6b65795f76616c7565";

    fn random_string<R: Rng>(rng: &mut R) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(rng.gen::<u8>().into())
            .map(char::from)
            .collect()
    }

    #[test]
    fn upgradeability() {
        let legacy_key_bytes = hex::decode(HEX_ENCODED_LEGACY_NAMED_KEY).unwrap();
        let (de, remainder) = NamedKey::from_bytes(&legacy_key_bytes).expect("deserialization");
        let de = match de {
            NamedKey::V1(_) => unreachable!("should not be invoked"),
            NamedKey::V2(v2) => v2,
        };
        assert_eq!(de.name, "legacy_key_name");
        assert_eq!(de.key, "legacy_key_value");
        assert_eq!(de.id, u64::default());
        assert!(remainder.is_empty());
    }

    #[test]
    fn roundtrip() {
        let mut rng = rand::thread_rng();
        let obj = NamedKey::V2(NamedKeyV2 {
            name: random_string(&mut rng),
            key: random_string(&mut rng),
            id: rng.gen::<u64>().into(),
        });

        let ser = NamedKey::to_bytes(&obj).expect("serialization");
        let (de, remainder) = NamedKey::from_bytes(&ser).expect("deserialization");

        assert_eq!(obj, de);
        assert_eq!(remainder.len(), 0);
    }
}
