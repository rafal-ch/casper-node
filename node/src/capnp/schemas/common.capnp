@0x9e126c05ee94717e;

using Rust = import "rust.capnp";
$Rust.parentModule("capnp::types::common");

struct Option(T) {
    union {
        some @0 :T;
        none @1 :Void;
    }
}

struct Map(Key, Value) {
  entries @0 :List(Entry);
  struct Entry {
    key @0 :Key;
    value @1 :Value;
  }
}

struct U512 {
  bytes0 @0 :UInt64;
  bytes1 @1 :UInt64;
  bytes2 @2 :UInt64;
  bytes3 @3 :UInt64;
  bytes4 @4 :UInt64;
  bytes5 @5 :UInt64;
  bytes6 @6 :UInt64;
  bytes7 @7 :UInt64;
}