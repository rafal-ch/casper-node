use bytesrepr_derive::BytesreprSerialize;
use casper_types::bytesrepr::ToBytes;

// TODO: Into prop test
#[test]
fn struct_simple() {
    #[derive(BytesreprSerialize)]
    struct Simple {
        unsigned_int: u16,
        int: i64,
        // float: f64, // TODO: No support for float in bytesrepr?
        string: String,
    }
    let s = Simple {
        unsigned_int: 1234,
        int: -98765,
        string: "Hello".to_string(),
    };

    let v1 = s.to_bytes();
    dbg!(&v1);
}
