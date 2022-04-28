use bytesrepr_derive::{BytesreprDeserialize, BytesreprSerialize};
use casper_types::bytesrepr::{FromBytes, ToBytes};

// TODO: Into prop test
#[test]
fn struct_simple() {
    #[derive(BytesreprSerialize, BytesreprDeserialize, Eq, PartialEq, Debug)]
    struct Simple {
        unsigned_int: u16,
        int: i64,
        // float: f64, // TODO: No support for float in bytesrepr?
        string: String,
    }
    let original = Simple {
        unsigned_int: 1234,
        int: -98765,
        string: "Hello".to_string(),
    };

    let serialized = original.to_bytes().expect("should serialize");
    let (deserialized, rem) = Simple::from_bytes(&serialized).expect("should deserialize");

    assert_eq!(original, deserialized);
    assert!(rem.is_empty());
}
