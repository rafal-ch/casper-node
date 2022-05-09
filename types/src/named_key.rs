// TODO - remove once schemars stops causing warning.
#![allow(clippy::field_reassign_with_default)]

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bytesrepr::{self, Bytes, FromBytes, ToBytes, MAGIC_BYTES};

/// A named key.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Default, Debug)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NamedKey {
    /// The name of the entry.
    pub name: String,
    /// The value of the entry: a casper `Key` type.
    pub key: String,
    /// The brand new, additional field
    pub id: u64,
}

impl ToBytes for NamedKey {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut ret = MAGIC_BYTES.to_vec();
        let mut field_map = BTreeMap::<String, Bytes>::new(); // TODO: If we have implemented `bytesrepr` for `HashMap` we could leverage the `with_capacity()` call
        field_map.insert("name".to_string(), Bytes::from(self.name.to_bytes()?));
        field_map.insert("key".to_string(), Bytes::from(self.key.to_bytes()?));
        field_map.insert("id".to_string(), Bytes::from(self.id.to_bytes()?));
        let bytes = field_map.to_bytes()?;
        ret.extend(bytes);
        Ok(ret)
    }

    fn serialized_length(&self) -> usize {
        unreachable!("should not be invoked")
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

        if Self::is_legacy(bytes) {
            let (legacy_named_key, remainder) = NamedKeyLegacy::from_bytes(&bytes)?;
            Ok((
                crate::NamedKey {
                    name: legacy_named_key.name,
                    key: legacy_named_key.key,
                    ..Default::default()
                },
                remainder,
            ))
        } else {
            let (field_map, remainder) =
                BTreeMap::<String, Bytes>::from_bytes(&bytes[MAGIC_BYTES.len()..])?;
            let name_bytes = field_map.get("name").unwrap();
            let (name, _) = String::from_bytes(name_bytes)?;
            let key_bytes = field_map.get("key").unwrap();
            let (key, _) = String::from_bytes(key_bytes)?;
            let id_bytes = field_map.get("id").unwrap();
            let (id, _) = u64::from_bytes(id_bytes)?;
            Ok((NamedKey { name, key, id }, remainder))
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{distributions::Alphanumeric, Rng};

    use crate::{
        bytesrepr::{FromBytes, ToBytes},
        NamedKey,
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
        assert_eq!(de.name, "legacy_key_name");
        assert_eq!(de.key, "legacy_key_value");
        assert_eq!(de.id, u64::default());
        assert!(remainder.is_empty());
    }

    #[test]
    fn roundtrip() {
        let mut rng = rand::thread_rng();
        let obj = crate::NamedKey {
            name: random_string(&mut rng),
            key: random_string(&mut rng),
            id: rng.gen::<u64>().into(),
        };

        let ser = NamedKey::to_bytes(&obj).expect("serialization");
        let (de, remainder) = NamedKey::from_bytes(&ser).expect("deserialization");

        assert_eq!(obj, de);
        assert_eq!(remainder.len(), 0);
    }
}
