use proptest::proptest;
use proptest::prelude::any;

use casper_types::{
    bytesrepr::{FromBytes, ToBytes},
    NamedKey,
};

proptest! {
    #[test]
    fn named_key_prop(name in any::<String>(), key in any::<String>()) {
        let original = NamedKey {
            name,
            key
        };

        let serialized = original.to_bytes().expect("should serialize");
        let (deserialized, rem) = NamedKey::from_bytes(&serialized).expect("should deserialize");

        assert_eq!(original, deserialized);
        assert!(rem.is_empty());
    }
}
