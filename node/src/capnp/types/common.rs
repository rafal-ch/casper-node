use std::convert::TryInto;

use casper_types::U512;

use super::{FromCapnpReader, ToCapnpBuilder};
use crate::capnp::{DeserializeError, FromCapnpBytes, SerializeError, ToCapnpBytes};
//use casper_node_macros::make_capnp_byte_setter_functions;

#[allow(dead_code)]
pub(super) mod common_capnp {
    include!(concat!(
        env!("OUT_DIR"),
        "/src/capnp/schemas/common_capnp.rs"
    ));
}

// We cannot use the const literals directly
// const U512_LENGTH_BYTES = 64;
// make_capnp_byte_setter_functions!(64, "u512", "common_capnp::u512");

impl ToCapnpBuilder<U512> for common_capnp::u512::Builder<'_> {
    fn try_to_builder(&mut self, x: &U512) -> Result<(), SerializeError> {
        let mut le_bytes = [0u8; 64];
        x.to_little_endian(&mut le_bytes[..]);
        let mut msg = self.reborrow();

        msg.set_bytes0(u64::from_le_bytes(le_bytes[0..=7].try_into().unwrap()));
        msg.set_bytes1(u64::from_le_bytes(le_bytes[8..=15].try_into().unwrap()));
        msg.set_bytes2(u64::from_le_bytes(le_bytes[16..=23].try_into().unwrap()));
        msg.set_bytes3(u64::from_le_bytes(le_bytes[24..=31].try_into().unwrap()));
        msg.set_bytes4(u64::from_le_bytes(le_bytes[32..=39].try_into().unwrap()));
        msg.set_bytes5(u64::from_le_bytes(le_bytes[40..=47].try_into().unwrap()));
        msg.set_bytes6(u64::from_le_bytes(le_bytes[48..=55].try_into().unwrap()));
        msg.set_bytes7(u64::from_le_bytes(le_bytes[56..=63].try_into().unwrap()));
        //set_u512(&mut msg, &le_bytes);
        Ok(())
    }
}

impl FromCapnpReader<U512> for common_capnp::u512::Reader<'_> {
    fn try_from_reader(&self) -> Result<U512, DeserializeError> {
        //let le_bytes = get_u512(*self);
        let mut le_bytes = [0u8; 64];
        le_bytes[0..=7].copy_from_slice(&self.get_bytes0().to_le_bytes());
        le_bytes[8..=15].copy_from_slice(&self.get_bytes1().to_le_bytes());
        le_bytes[16..=23].copy_from_slice(&self.get_bytes2().to_le_bytes());
        le_bytes[24..=31].copy_from_slice(&self.get_bytes3().to_le_bytes());
        le_bytes[32..=39].copy_from_slice(&self.get_bytes4().to_le_bytes());
        le_bytes[40..=47].copy_from_slice(&self.get_bytes5().to_le_bytes());
        le_bytes[48..=55].copy_from_slice(&self.get_bytes6().to_le_bytes());
        le_bytes[56..=63].copy_from_slice(&self.get_bytes7().to_le_bytes());
        Ok(U512::from_little_endian(&le_bytes))
    }
}

impl ToCapnpBytes for U512 {
    fn try_to_capnp_bytes(&self) -> Result<Vec<u8>, SerializeError> {
        let mut builder = capnp::message::Builder::new_default();
        let mut msg = builder.init_root::<common_capnp::u512::Builder>();
        msg.try_to_builder(self)?;

        let mut serialized = Vec::new();
        capnp::serialize::write_message(&mut serialized, &builder)?;
        Ok(serialized)
    }
}

impl FromCapnpBytes for U512 {
    fn try_from_capnp_bytes(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let deserialized =
            capnp::serialize::read_message(bytes, capnp::message::ReaderOptions::new())?;

        let reader = deserialized.get_root::<common_capnp::u512::Reader>()?;
        reader.try_from_reader()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{super::random_bytes, *};

    pub(crate) fn random_u512() -> U512 {
        let bytes = random_bytes(64);
        U512::from_little_endian(&bytes)
    }

    #[test]
    fn u512_capnp() {
        let original = random_u512();
        let serialized = original.try_to_capnp_bytes().expect("serialization");
        let deserialized = U512::try_from_capnp_bytes(&serialized).expect("deserialization");
        assert_eq!(original, deserialized);
    }
}
