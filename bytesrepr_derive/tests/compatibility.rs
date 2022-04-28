use bytesrepr_derive::{BytesreprDeserialize, BytesreprSerialize};
use casper_types::{bytesrepr::{FromBytes, ToBytes}, NamedKey};

// TODO: Into prop test
#[test]
fn named_key() {
    let original = NamedKey {
        name: "named_key_name".to_string(),
        key: "named_key_key".to_string(),
    };

    let serialized = original.to_bytes().expect("should serialize");
    let (deserialized, rem) = NamedKey::from_bytes(&serialized).expect("should deserialize");

    assert_eq!(original, deserialized);
    assert!(rem.is_empty());
}
