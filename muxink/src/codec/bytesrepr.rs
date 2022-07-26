//! Bytesrepr encoding/decoding
//!
use std::{
    error, fmt,
    fmt::{Debug, Display},
    io::{self, Cursor},
    marker::PhantomData,
};

use bytes::{Buf, Bytes, BytesMut};
use casper_types::bytesrepr::{self, FromBytes, ToBytes};
use serde::{de::DeserializeOwned, Serialize};

use super::{DecodeResult, FrameDecoder, Transcoder};
use crate::codec::DecodeResult::Failed;

#[derive(Debug)]
enum TranscoderError {
    BufferNotExhausted { left: usize },
    BytesreprError(bytesrepr::Error),
}
impl error::Error for TranscoderError {}
impl Display for TranscoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TranscoderError::BufferNotExhausted { left } => {
                write!(f, "{} bytes still in the buffer after decoding", left)
            }
            TranscoderError::BytesreprError(inner) => {
                write!(f, "bytesrepr error: {}", inner)
            }
        }
    }
}

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
        let bytes = input
            .to_bytes()
            .map_err(|e| TranscoderError::BytesreprError(e))?;

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
    R: AsRef<[u8]> + Debug,
{
    type Error = Box<dyn error::Error + Send + Sync + 'static>;

    type Output = T;

    fn transcode(&mut self, input: R) -> Result<Self::Output, Self::Error> {
        let (data, rem) = FromBytes::from_bytes(input.as_ref())
            .map_err(|e| TranscoderError::BytesreprError(e))?;

        if !rem.is_empty() {
            return Err(TranscoderError::BufferNotExhausted { left: rem.len() }.into());
        }

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
        match transcoded {
            Ok((data, rem)) => {
                let _ = buffer.split_to(buffer.remaining() - rem.len());
                DecodeResult::Item(data)
            }
            Err(err) => match &err {
                bytesrepr::Error::EarlyEndOfStream => DecodeResult::Incomplete,
                bytesrepr::Error::Formatting
                | bytesrepr::Error::LeftOverBytes
                | bytesrepr::Error::NotRepresentable
                | bytesrepr::Error::ExceededRecursionDepth
                | bytesrepr::Error::OutOfMemory => {
                    Failed(TranscoderError::BytesreprError(err).into())
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DecodeResult;
    use crate::codec::{
        bytesrepr::{
            BytesreprDecoder, BytesreprEncoder,
            TranscoderError::{BufferNotExhausted, BytesreprError},
        },
        BytesMut, FrameDecoder, Transcoder,
    };
    use casper_types::bytesrepr;

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

    #[test]
    fn error_when_buffer_not_exhausted() {
        let data = b"\x03\0\0\0abc\x04\0\0\0defg";

        let mut decoder = BytesreprDecoder::<String>::new();
        let actual_error = decoder.transcode(data).unwrap_err();

        let expected_error = Box::new(BufferNotExhausted { left: 8 });
        assert_eq!(expected_error.to_string(), actual_error.to_string())
    }

    #[test]
    fn error_when_data_incomplete() {
        let data = b"\x03\0\0\0ab";

        let mut decoder = BytesreprDecoder::<String>::new();
        let actual_error = decoder.transcode(data).unwrap_err();

        let expected_error = Box::new(BytesreprError(bytesrepr::Error::EarlyEndOfStream));
        assert_eq!(expected_error.to_string(), actual_error.to_string())
    }
}
