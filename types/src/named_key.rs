// TODO - remove once schemars stops causing warning.
#![allow(clippy::field_reassign_with_default)]

use alloc::{string::String, vec::Vec};

#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bytesrepr::{self, FromBytes, ToBytes};

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
}

impl ToBytes for NamedKey {
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

impl FromBytes for NamedKey {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (name, remainder) = String::from_bytes(bytes)?;
        let (key, remainder) = String::from_bytes(remainder)?;
        let named_key = NamedKey { name, key };
        Ok((named_key, remainder))
    }
}

mod key_value {
    const MAGIC_BYTES: &[u8] = &[0xff, 0xfe, 0xbb, 0xaa];

    #[cfg(feature = "datasize")]
    use datasize::DataSize;
    #[cfg(feature = "json-schema")]
    use schemars::JsonSchema;

    use alloc::collections::BTreeMap;
    use serde::{Deserialize, Serialize};

    use crate::{
        bytesrepr::{self, Bytes},
        bytesrepr::{FromBytes, ToBytes},
    };
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

    fn to_bytes(obj: &NamedKey) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut ret = MAGIC_BYTES.to_vec();
        let mut field_map = BTreeMap::<String, Bytes>::new(); // TODO: If we have implemented `bytesrepr` for `HashMap` we could leverage the `with_capacity()` call
        field_map.insert("name".to_string(), Bytes::from(obj.name.to_bytes()?));
        field_map.insert("key".to_string(), Bytes::from(obj.key.to_bytes()?));
        field_map.insert("id".to_string(), Bytes::from(obj.id.to_bytes()?));
        let bytes = field_map.to_bytes()?;
        ret.extend(bytes);
        Ok(ret)
    }

    fn is_legacy(bytes: &[u8]) -> bool {
        !bytes.starts_with(MAGIC_BYTES)
    }

    fn from_bytes(
        bytes: &[u8],
    ) -> Result<(crate::named_key::key_value::NamedKey, &[u8]), bytesrepr::Error> {
        if is_legacy(bytes) {
            let (legacy_named_key, remainder) = crate::NamedKey::from_bytes(&bytes)?;
            Ok((
                crate::named_key::key_value::NamedKey {
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
            Ok((
                crate::named_key::key_value::NamedKey { name, key, id },
                remainder,
            ))
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::bytesrepr::ToBytes;
        use rand::{distributions::Alphanumeric, Rng};

        fn random_string<R: Rng>(rng: &mut R) -> String {
            rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(rng.gen::<u8>().into())
                .map(char::from)
                .collect()
        }

        #[test]
        fn key_value_roundtrip() {
            let mut rng = rand::thread_rng();
            let obj = crate::named_key::key_value::NamedKey {
                name: random_string(&mut rng),
                key: random_string(&mut rng),
                id: rng.gen::<u64>().into(),
            };

            let ser = crate::named_key::key_value::to_bytes(&obj).expect("serialization");
            let (de, remainder) =
                crate::named_key::key_value::from_bytes(&ser).expect("deserialization");

            assert_eq!(obj, de);
            assert_eq!(remainder.len(), 0);
        }

        #[test]
        fn upgradeability() {
            let mut rng = rand::thread_rng();

            let obj = crate::NamedKey {
                name: random_string(&mut rng),
                key: random_string(&mut rng),
            };

            let ser_legacy = obj.to_bytes().unwrap();
            let (de, remainder) =
                crate::named_key::key_value::from_bytes(&ser_legacy).expect("deserialization");

            assert_eq!(de.name, obj.name);
            assert_eq!(de.key, obj.key);
            assert_eq!(de.id, u64::default());
            assert_eq!(remainder.len(), 0);
        }
    }
}
