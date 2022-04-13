use std::{collections::BTreeMap, convert::TryInto};

use casper_types::{AsymmetricType, PublicKey};

use crate::{
    capnp::{types::public_key::public_key_capnp, Error, FromCapnpBytes, ToCapnpBytes},
    components::consensus::EraReport,
};

#[allow(dead_code)]
mod era_report_capnp {
    include!(concat!(
        env!("OUT_DIR"),
        "/src/capnp/schemas/era_report_capnp.rs"
    ));
}

#[allow(dead_code)]
mod map_capnp {
    include!(concat!(env!("OUT_DIR"), "/src/capnp/schemas/map_capnp.rs"));
}

impl ToCapnpBytes for EraReport<PublicKey> {
    fn try_to_capnp_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut builder = capnp::message::Builder::new_default();
        let mut root = builder.init_root::<era_report_capnp::era_report::Builder>();

        {
            let inactive_validators_count: u32 = self
                .inactive_validators
                .len()
                .try_into()
                .map_err(|_| Error::TooManyItems)?;

            let mut inactive_validators = root
                .reborrow()
                .init_inactive_validators(inactive_validators_count);
            for (index, inactive_validator) in self.inactive_validators.iter().enumerate() {
                let mut validator_builder = inactive_validators.reborrow().get(index as u32);
                // TODO[RC]: Deduplicate the code for writing PublicKey
                match inactive_validator {
                    PublicKey::Ed25519(key) => {
                        let bytes = key.as_bytes();
                        let mut msg = validator_builder.init_ed25519();
                        msg.set_byte0(bytes[0]);
                        msg.set_byte1(bytes[1]);
                        msg.set_byte2(bytes[2]);
                        msg.set_byte3(bytes[3]);
                        msg.set_byte4(bytes[4]);
                        msg.set_byte5(bytes[5]);
                        msg.set_byte6(bytes[6]);
                        msg.set_byte7(bytes[7]);
                        msg.set_byte8(bytes[8]);
                        msg.set_byte9(bytes[9]);
                        msg.set_byte10(bytes[10]);
                        msg.set_byte11(bytes[11]);
                        msg.set_byte12(bytes[12]);
                        msg.set_byte13(bytes[13]);
                        msg.set_byte14(bytes[14]);
                        msg.set_byte15(bytes[15]);
                        msg.set_byte16(bytes[16]);
                        msg.set_byte17(bytes[17]);
                        msg.set_byte18(bytes[18]);
                        msg.set_byte19(bytes[19]);
                        msg.set_byte20(bytes[20]);
                        msg.set_byte21(bytes[20]);
                        msg.set_byte21(bytes[21]);
                        msg.set_byte22(bytes[22]);
                        msg.set_byte23(bytes[23]);
                        msg.set_byte24(bytes[24]);
                        msg.set_byte25(bytes[25]);
                        msg.set_byte26(bytes[26]);
                        msg.set_byte27(bytes[27]);
                        msg.set_byte28(bytes[28]);
                        msg.set_byte29(bytes[29]);
                        msg.set_byte30(bytes[30]);
                        msg.set_byte31(bytes[31]);
                    }
                    PublicKey::Secp256k1(key) => {
                        let bytes = key.to_bytes();
                        let mut msg = validator_builder.init_secp256k1();
                        msg.set_byte0(bytes[0]);
                        msg.set_byte1(bytes[1]);
                        msg.set_byte2(bytes[2]);
                        msg.set_byte3(bytes[3]);
                        msg.set_byte4(bytes[4]);
                        msg.set_byte5(bytes[5]);
                        msg.set_byte6(bytes[6]);
                        msg.set_byte7(bytes[7]);
                        msg.set_byte8(bytes[8]);
                        msg.set_byte9(bytes[9]);
                        msg.set_byte10(bytes[10]);
                        msg.set_byte11(bytes[11]);
                        msg.set_byte12(bytes[12]);
                        msg.set_byte13(bytes[13]);
                        msg.set_byte14(bytes[14]);
                        msg.set_byte15(bytes[15]);
                        msg.set_byte16(bytes[16]);
                        msg.set_byte17(bytes[17]);
                        msg.set_byte18(bytes[18]);
                        msg.set_byte19(bytes[19]);
                        msg.set_byte20(bytes[20]);
                        msg.set_byte21(bytes[20]);
                        msg.set_byte21(bytes[21]);
                        msg.set_byte22(bytes[22]);
                        msg.set_byte23(bytes[23]);
                        msg.set_byte24(bytes[24]);
                        msg.set_byte25(bytes[25]);
                        msg.set_byte26(bytes[26]);
                        msg.set_byte27(bytes[27]);
                        msg.set_byte28(bytes[28]);
                        msg.set_byte29(bytes[29]);
                        msg.set_byte30(bytes[30]);
                        msg.set_byte31(bytes[31]);
                        msg.set_byte32(bytes[32]);
                    }
                    PublicKey::System => {
                        let _ = validator_builder.set_system(());
                    }
                }
            }
        }
        {
            let equivocators_count: u32 = self
                .equivocators
                .len()
                .try_into()
                .map_err(|_| Error::TooManyItems)?;

            let mut equivocators = root.reborrow().init_equivocators(equivocators_count);
            for (index, equivocator) in self.equivocators.iter().enumerate() {
                let mut equivocator_builder = equivocators.reborrow().get(index as u32);
                // TODO[RC]: Deduplicate the code for writing PublicKey
                match equivocator {
                    PublicKey::Ed25519(key) => {
                        let bytes = key.as_bytes();
                        let mut msg = equivocator_builder.init_ed25519();
                        msg.set_byte0(bytes[0]);
                        msg.set_byte1(bytes[1]);
                        msg.set_byte2(bytes[2]);
                        msg.set_byte3(bytes[3]);
                        msg.set_byte4(bytes[4]);
                        msg.set_byte5(bytes[5]);
                        msg.set_byte6(bytes[6]);
                        msg.set_byte7(bytes[7]);
                        msg.set_byte8(bytes[8]);
                        msg.set_byte9(bytes[9]);
                        msg.set_byte10(bytes[10]);
                        msg.set_byte11(bytes[11]);
                        msg.set_byte12(bytes[12]);
                        msg.set_byte13(bytes[13]);
                        msg.set_byte14(bytes[14]);
                        msg.set_byte15(bytes[15]);
                        msg.set_byte16(bytes[16]);
                        msg.set_byte17(bytes[17]);
                        msg.set_byte18(bytes[18]);
                        msg.set_byte19(bytes[19]);
                        msg.set_byte20(bytes[20]);
                        msg.set_byte21(bytes[20]);
                        msg.set_byte21(bytes[21]);
                        msg.set_byte22(bytes[22]);
                        msg.set_byte23(bytes[23]);
                        msg.set_byte24(bytes[24]);
                        msg.set_byte25(bytes[25]);
                        msg.set_byte26(bytes[26]);
                        msg.set_byte27(bytes[27]);
                        msg.set_byte28(bytes[28]);
                        msg.set_byte29(bytes[29]);
                        msg.set_byte30(bytes[30]);
                        msg.set_byte31(bytes[31]);
                    }
                    PublicKey::Secp256k1(key) => {
                        let bytes = key.to_bytes();
                        let mut msg = equivocator_builder.init_secp256k1();
                        msg.set_byte0(bytes[0]);
                        msg.set_byte1(bytes[1]);
                        msg.set_byte2(bytes[2]);
                        msg.set_byte3(bytes[3]);
                        msg.set_byte4(bytes[4]);
                        msg.set_byte5(bytes[5]);
                        msg.set_byte6(bytes[6]);
                        msg.set_byte7(bytes[7]);
                        msg.set_byte8(bytes[8]);
                        msg.set_byte9(bytes[9]);
                        msg.set_byte10(bytes[10]);
                        msg.set_byte11(bytes[11]);
                        msg.set_byte12(bytes[12]);
                        msg.set_byte13(bytes[13]);
                        msg.set_byte14(bytes[14]);
                        msg.set_byte15(bytes[15]);
                        msg.set_byte16(bytes[16]);
                        msg.set_byte17(bytes[17]);
                        msg.set_byte18(bytes[18]);
                        msg.set_byte19(bytes[19]);
                        msg.set_byte20(bytes[20]);
                        msg.set_byte21(bytes[20]);
                        msg.set_byte21(bytes[21]);
                        msg.set_byte22(bytes[22]);
                        msg.set_byte23(bytes[23]);
                        msg.set_byte24(bytes[24]);
                        msg.set_byte25(bytes[25]);
                        msg.set_byte26(bytes[26]);
                        msg.set_byte27(bytes[27]);
                        msg.set_byte28(bytes[28]);
                        msg.set_byte29(bytes[29]);
                        msg.set_byte30(bytes[30]);
                        msg.set_byte31(bytes[31]);
                        msg.set_byte32(bytes[32]);
                    }
                    PublicKey::System => {
                        let _ = equivocator_builder.set_system(());
                    }
                }
            }
        }

        let mut serialized = Vec::new();
        capnp::serialize::write_message(&mut serialized, &builder)
            .map_err(|_| Error::UnableToSerialize)?;
        Ok(serialized)
    }
}

impl FromCapnpBytes for EraReport<PublicKey> {
    fn try_from_capnp_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let deserialized =
            capnp::serialize::read_message(bytes, capnp::message::ReaderOptions::new())
                .expect("unable to deserialize struct");

        let reader = deserialized
            .get_root::<era_report_capnp::era_report::Reader>()
            .map_err(|_| Error::UnableToDeserialize)?;

        let mut target_inactive_validators = vec![];
        {
            if reader.has_inactive_validators() {
                for inactive_validator in reader
                    .get_inactive_validators()
                    .map_err(|_| Error::UnableToDeserialize)?
                    .iter()
                {
                    let deserialized_validator = match inactive_validator
                        .which()
                        .map_err(|_| Error::UnableToDeserialize)?
                    {
                        public_key_capnp::public_key::Which::Ed25519(reader) => match reader {
                            Ok(reader) => {
                                let bytes: [u8; PublicKey::ED25519_LENGTH] = [
                                    reader.get_byte0(),
                                    reader.get_byte1(),
                                    reader.get_byte2(),
                                    reader.get_byte3(),
                                    reader.get_byte4(),
                                    reader.get_byte5(),
                                    reader.get_byte6(),
                                    reader.get_byte7(),
                                    reader.get_byte8(),
                                    reader.get_byte9(),
                                    reader.get_byte10(),
                                    reader.get_byte11(),
                                    reader.get_byte12(),
                                    reader.get_byte13(),
                                    reader.get_byte14(),
                                    reader.get_byte15(),
                                    reader.get_byte16(),
                                    reader.get_byte17(),
                                    reader.get_byte18(),
                                    reader.get_byte19(),
                                    reader.get_byte20(),
                                    reader.get_byte21(),
                                    reader.get_byte22(),
                                    reader.get_byte23(),
                                    reader.get_byte24(),
                                    reader.get_byte25(),
                                    reader.get_byte26(),
                                    reader.get_byte27(),
                                    reader.get_byte28(),
                                    reader.get_byte29(),
                                    reader.get_byte30(),
                                    reader.get_byte31(),
                                ];

                                Ok(PublicKey::ed25519_from_bytes(bytes)
                                    .map_err(|_| Error::UnableToDeserialize)?)
                            }
                            Err(_) => Err(Error::UnableToDeserialize),
                        },
                        public_key_capnp::public_key::Which::Secp256k1(reader) => match reader {
                            Ok(reader) => {
                                let bytes: [u8; PublicKey::SECP256K1_LENGTH] = [
                                    reader.get_byte0(),
                                    reader.get_byte1(),
                                    reader.get_byte2(),
                                    reader.get_byte3(),
                                    reader.get_byte4(),
                                    reader.get_byte5(),
                                    reader.get_byte6(),
                                    reader.get_byte7(),
                                    reader.get_byte8(),
                                    reader.get_byte9(),
                                    reader.get_byte10(),
                                    reader.get_byte11(),
                                    reader.get_byte12(),
                                    reader.get_byte13(),
                                    reader.get_byte14(),
                                    reader.get_byte15(),
                                    reader.get_byte16(),
                                    reader.get_byte17(),
                                    reader.get_byte18(),
                                    reader.get_byte19(),
                                    reader.get_byte20(),
                                    reader.get_byte21(),
                                    reader.get_byte22(),
                                    reader.get_byte23(),
                                    reader.get_byte24(),
                                    reader.get_byte25(),
                                    reader.get_byte26(),
                                    reader.get_byte27(),
                                    reader.get_byte28(),
                                    reader.get_byte29(),
                                    reader.get_byte30(),
                                    reader.get_byte31(),
                                    reader.get_byte32(),
                                ];
                                Ok(PublicKey::secp256k1_from_bytes(bytes)
                                    .map_err(|_| Error::UnableToDeserialize)?)
                            }
                            Err(_) => Err(Error::UnableToDeserialize),
                        },
                        public_key_capnp::public_key::Which::System(_) => Ok(PublicKey::System),
                    };
                    target_inactive_validators.push(deserialized_validator?);
                }
            }
        }

        let mut target_equivocator = vec![];
        {
            if reader.has_equivocators() {
                for equivocator in reader
                    .get_equivocators()
                    .map_err(|_| Error::UnableToDeserialize)?
                    .iter()
                {
                    let deserialized_equivocator = match equivocator
                        .which()
                        .map_err(|_| Error::UnableToDeserialize)?
                    {
                        public_key_capnp::public_key::Which::Ed25519(reader) => match reader {
                            Ok(reader) => {
                                let bytes: [u8; PublicKey::ED25519_LENGTH] = [
                                    reader.get_byte0(),
                                    reader.get_byte1(),
                                    reader.get_byte2(),
                                    reader.get_byte3(),
                                    reader.get_byte4(),
                                    reader.get_byte5(),
                                    reader.get_byte6(),
                                    reader.get_byte7(),
                                    reader.get_byte8(),
                                    reader.get_byte9(),
                                    reader.get_byte10(),
                                    reader.get_byte11(),
                                    reader.get_byte12(),
                                    reader.get_byte13(),
                                    reader.get_byte14(),
                                    reader.get_byte15(),
                                    reader.get_byte16(),
                                    reader.get_byte17(),
                                    reader.get_byte18(),
                                    reader.get_byte19(),
                                    reader.get_byte20(),
                                    reader.get_byte21(),
                                    reader.get_byte22(),
                                    reader.get_byte23(),
                                    reader.get_byte24(),
                                    reader.get_byte25(),
                                    reader.get_byte26(),
                                    reader.get_byte27(),
                                    reader.get_byte28(),
                                    reader.get_byte29(),
                                    reader.get_byte30(),
                                    reader.get_byte31(),
                                ];

                                Ok(PublicKey::ed25519_from_bytes(bytes)
                                    .map_err(|_| Error::UnableToDeserialize)?)
                            }
                            Err(_) => Err(Error::UnableToDeserialize),
                        },
                        public_key_capnp::public_key::Which::Secp256k1(reader) => match reader {
                            Ok(reader) => {
                                let bytes: [u8; PublicKey::SECP256K1_LENGTH] = [
                                    reader.get_byte0(),
                                    reader.get_byte1(),
                                    reader.get_byte2(),
                                    reader.get_byte3(),
                                    reader.get_byte4(),
                                    reader.get_byte5(),
                                    reader.get_byte6(),
                                    reader.get_byte7(),
                                    reader.get_byte8(),
                                    reader.get_byte9(),
                                    reader.get_byte10(),
                                    reader.get_byte11(),
                                    reader.get_byte12(),
                                    reader.get_byte13(),
                                    reader.get_byte14(),
                                    reader.get_byte15(),
                                    reader.get_byte16(),
                                    reader.get_byte17(),
                                    reader.get_byte18(),
                                    reader.get_byte19(),
                                    reader.get_byte20(),
                                    reader.get_byte21(),
                                    reader.get_byte22(),
                                    reader.get_byte23(),
                                    reader.get_byte24(),
                                    reader.get_byte25(),
                                    reader.get_byte26(),
                                    reader.get_byte27(),
                                    reader.get_byte28(),
                                    reader.get_byte29(),
                                    reader.get_byte30(),
                                    reader.get_byte31(),
                                    reader.get_byte32(),
                                ];
                                Ok(PublicKey::secp256k1_from_bytes(bytes)
                                    .map_err(|_| Error::UnableToDeserialize)?)
                            }
                            Err(_) => Err(Error::UnableToDeserialize),
                        },
                        public_key_capnp::public_key::Which::System(_) => Ok(PublicKey::System),
                    };
                    target_equivocator.push(deserialized_equivocator?);
                }
            }
        }

        Ok(EraReport {
            equivocators: target_equivocator,
            rewards: BTreeMap::new(),
            inactive_validators: target_inactive_validators,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use casper_types::{PublicKey, SecretKey};

    use crate::{
        capnp::{FromCapnpBytes, ToCapnpBytes},
        components::consensus::EraReport,
    };

    // TODO[RC]: Deduplicate
    fn random_bytes(len: usize) -> Vec<u8> {
        let mut buf = vec![0; len];
        getrandom::getrandom(&mut buf).expect("should get random");
        buf
    }

    #[test]
    fn era_report_capnp() {
        let bytes = random_bytes(SecretKey::SECP256K1_LENGTH);
        let secret_key =
            SecretKey::secp256k1_from_bytes(bytes.as_slice()).expect("should create secret key");
        let equivocator_1: PublicKey = (&secret_key).into();

        let bytes = random_bytes(SecretKey::SECP256K1_LENGTH);
        let secret_key =
            SecretKey::secp256k1_from_bytes(bytes.as_slice()).expect("should create secret key");
        let equivocator_2: PublicKey = (&secret_key).into();

        let bytes = random_bytes(SecretKey::SECP256K1_LENGTH);
        let secret_key =
            SecretKey::secp256k1_from_bytes(bytes.as_slice()).expect("should create secret key");
        let inactive_validator_1: PublicKey = (&secret_key).into();

        let bytes = random_bytes(SecretKey::SECP256K1_LENGTH);
        let secret_key =
            SecretKey::secp256k1_from_bytes(bytes.as_slice()).expect("should create secret key");
        let inactive_validator_2: PublicKey = (&secret_key).into();

        let bytes = random_bytes(SecretKey::SECP256K1_LENGTH);
        let secret_key =
            SecretKey::secp256k1_from_bytes(bytes.as_slice()).expect("should create secret key");
        let inactive_validator_3: PublicKey = (&secret_key).into();

        let bytes = random_bytes(SecretKey::SECP256K1_LENGTH);
        let secret_key =
            SecretKey::secp256k1_from_bytes(bytes.as_slice()).expect("should create secret key");
        let _got_reward_1: PublicKey = (&secret_key).into();

        let bytes = random_bytes(SecretKey::SECP256K1_LENGTH);
        let secret_key =
            SecretKey::secp256k1_from_bytes(bytes.as_slice()).expect("should create secret key");
        let _got_reward_2: PublicKey = (&secret_key).into();

        let rewards: BTreeMap<PublicKey, u64> = BTreeMap::new();
        //rewards.insert(got_reward_1, 0);
        //rewards.insert(got_reward_2, u64::MAX);

        let original = EraReport {
            equivocators: vec![equivocator_1, equivocator_2],
            rewards,
            inactive_validators: vec![
                inactive_validator_1,
                inactive_validator_2,
                inactive_validator_3,
            ],
        };

        let serialized = original.try_to_capnp_bytes().expect("serialization");
        let deserialized = EraReport::try_from_capnp_bytes(&serialized).expect("deserialization");

        assert_eq!(original, deserialized);
    }
}
