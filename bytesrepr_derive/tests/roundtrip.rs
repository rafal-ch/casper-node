extern crate alloc;

use core::ptr::NonNull;
use core::alloc::Layout;
use core::any;
use core::mem;
use std::alloc::alloc;
use alloc::vec::Vec;
use casper_types::bytesrepr::Error;

use bytesrepr_derive::{BytesreprDeserialize, BytesreprSerialize};
use casper_types::bytesrepr::{FromBytes, ToBytes};

#[test]
fn struct_simple() {
    #[derive(BytesreprSerialize, BytesreprDeserialize, Eq, PartialEq, Debug)]
    struct Simple {
        unsigned_int: u16,
        int: i64,
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

// #[test]
// fn struct_with_vector() {
//     #[derive(BytesreprSerialize, BytesreprDeserialize, Eq, PartialEq, Debug)]
//     struct WithVector {
//         data: Vec<i64>,
//     }
//     let original = Simple {
//         data: vec![-123, 0, 1234],
//     };

//     let serialized = original.to_bytes().expect("should serialize");
//     let (deserialized, rem) = Simple::from_bytes(&serialized).expect("should deserialize");

//     assert_eq!(original, deserialized);
//     assert!(rem.is_empty());
// }
