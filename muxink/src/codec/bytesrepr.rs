//! Bytesrepr encoding/decoding
//!
use std::{
    error,
    io::{self, Cursor},
    marker::PhantomData,
};

use bytes::{Buf, Bytes, BytesMut};
use casper_types::bytesrepr::{self, FromBytes, ToBytes};
use serde::{de::DeserializeOwned, Serialize};

use super::{DecodeResult, FrameDecoder, Transcoder};

/// A bytesrepr encoder.
#[derive(Default)]
pub struct BytesreprEncoder<T> {
    /// Item type processed by this encoder.
    ///
    /// We restrict encoders to a single message type to make decoding on the other end easier.
    item_type: PhantomData<T>,
}

impl<T> BytesreprEncoder<T> {
    /// Creates a new bytesrepr encoder.
    pub fn new() -> Self {
        BytesreprEncoder {
            item_type: PhantomData,
        }
    }
}

impl<T> Transcoder<T> for BytesreprEncoder<T>
where
    T: ToBytes,
{
    type Error = Box<dyn error::Error + Send + Sync + 'static>;

    type Output = Bytes;

    fn transcode(&mut self, input: T) -> Result<Self::Output, Self::Error> {
        let bytes = input.to_bytes().expect("TODO[RC]: handle failure");

        Ok(bytes.into())
    }
}

/// Bytesrepr decoder.
#[derive(Default)]
pub struct BytesreprDecoder<T> {
    item_type: PhantomData<T>,
}

impl<T> BytesreprDecoder<T> {
    /// Creates a new bytesrepr decoder.
    pub fn new() -> Self {
        BytesreprDecoder {
            item_type: PhantomData,
        }
    }
}

impl<R, T> Transcoder<R> for BytesreprDecoder<T>
where
    T: FromBytes + Send + Sync + 'static,
    R: AsRef<[u8]>,
{
    type Error = Box<dyn error::Error + Send + Sync + 'static>;

    type Output = T;

    fn transcode(&mut self, input: R) -> Result<Self::Output, Self::Error> {
        let transcoded = FromBytes::from_bytes(input.as_ref());
        let (data, rem) = transcoded.expect("TODO[RC]: handle failure");

        // TODO[RC]: Fail, if rem != 0

        Ok(data)
    }
}

impl<T> FrameDecoder for BytesreprDecoder<T>
where
    T: FromBytes + Send + Sync + 'static,
{
    type Error = Box<dyn error::Error + Send + Sync + 'static>;
    type Output = T;

    fn decode_frame(&mut self, buffer: &mut BytesMut) -> DecodeResult<Self::Output, Self::Error> {
        let transcoded = FromBytes::from_bytes(buffer.as_ref());
        let (data, rem) = transcoded.expect("TODO[RC]: handle failure");

        buffer.split_to(buffer.remaining() - rem.len());

        DecodeResult::Item(data)
    }
}

#[cfg(test)]
mod tests {
    use super::DecodeResult;
    use crate::codec::{
        bytesrepr::{BytesreprDecoder, BytesreprEncoder},
        BytesMut, FrameDecoder, Transcoder,
    };

    #[test]
    fn roundtrip() {
        let data = "abc";

        let mut encoder = BytesreprEncoder::new();
        let value: String = String::from(data);
        let encoded = encoder.transcode(value).expect("should encode");

        let mut decoder = BytesreprDecoder::<String>::new();
        let decoded = decoder.transcode(encoded).expect("should decode");

        assert_eq!(data, decoded);
    }

    #[test]
    fn decodes_frame() {
        let data = b"\x03\0\0\0abc\x04\0\0\0defg";

        let mut bytes: BytesMut = BytesMut::new();
        bytes.extend(data);

        let mut decoder = BytesreprDecoder::<String>::new();

        assert!(matches!(decoder.decode_frame(&mut bytes), DecodeResult::Item(i) if i == "abc"));
        assert!(matches!(decoder.decode_frame(&mut bytes), DecodeResult::Item(i) if i == "defg"));
    }

    // #[test]
    // fn error_when_decoding_incorrect_data() {
    //     let data = "abc";

    //     let mut decoder = BytesreprDecoder::<String>::new();
    //     let decoded = decoder.transcode(data).expect_err("should not decode");
    // }
}
