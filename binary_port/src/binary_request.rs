use core::convert::TryFrom;

use casper_types::{
    bytesrepr::{self, Bytes, FromBytes, ToBytes},
    ProtocolVersion, Transaction,
};
use tokio_util::{
    bytes::{self, Buf},
    codec,
};

use crate::{get_request::GetRequest, Error};

#[cfg(test)]
use casper_types::testing::TestRng;
#[cfg(test)]
use rand::Rng;

type LengthEncoding = u32;
const LENGTH_ENCODING_SIZE_BYTES: usize = std::mem::size_of::<LengthEncoding>();

// TODO: Move to binary port
pub const SUPPORTED_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::from_parts(2, 0, 0);

pub struct BinaryRequestCodec {}

impl codec::Encoder<BinaryRequest> for BinaryRequestCodec {
    type Error = Error;

    fn encode(
        &mut self,
        item: BinaryRequest,
        dst: &mut bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        let header = BinaryRequestHeader::new(SUPPORTED_PROTOCOL_VERSION, item.tag());
        let length = (header.serialized_length() + item.serialized_length()) as LengthEncoding;

        // TODO: Can we write directly to 'dst'?
        let mut bytes = Vec::with_capacity(length as usize);
        header.write_bytes(&mut bytes)?;
        item.write_bytes(&mut bytes)?;

        let length_bytes = length.to_le_bytes();

        dst.extend(length_bytes.iter().chain(bytes.iter()));
        Ok(())
    }
}
impl codec::Decoder for BinaryRequestCodec {
    type Item = BinaryRequest;

    type Error = Error;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < LENGTH_ENCODING_SIZE_BYTES {
            // Not enough bytes to read the length.
            return Ok(None);
        }
        let length = LengthEncoding::from_le_bytes([src[0], src[1], src[2], src[3]]) as usize;
        if src.len() < length {
            // Not enough bytes to read the whole message.
            return Ok(None);
        }

        // TODO: Remove the 'handle_payload' function as it is only used in juliet-based implementation.
        let serialized = &src[LENGTH_ENCODING_SIZE_BYTES..LENGTH_ENCODING_SIZE_BYTES + length];
        let (header, remainder) =
            BinaryRequestHeader::from_bytes(serialized).map_err(Error::BytesRepr)?;
        let tag = BinaryRequestTag::try_from(header.type_tag())
            .map_err(|err| Error::InvalidBinaryRequestTag(err.0))?;
        let deserialized = BinaryRequest::try_from((tag, remainder)).map_err(Error::BytesRepr)?;
        src.advance(LENGTH_ENCODING_SIZE_BYTES + length);
        Ok(Some(deserialized))
    }
}

/// The header of a binary request.
#[derive(Debug, PartialEq)]
pub struct BinaryRequestHeader {
    protocol_version: ProtocolVersion,
    type_tag: u8,
}

impl BinaryRequestHeader {
    /// Creates new binary request header.
    pub fn new(protocol_version: ProtocolVersion, type_tag: BinaryRequestTag) -> Self {
        Self {
            protocol_version,
            type_tag: type_tag.into(),
        }
    }

    /// Returns the protocol version of the request.
    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    /// Returns the type tag of the request.
    pub fn type_tag(&self) -> u8 {
        self.type_tag
    }

    #[cfg(test)]
    pub(crate) fn random(rng: &mut TestRng) -> Self {
        Self {
            protocol_version: ProtocolVersion::from_parts(rng.gen(), rng.gen(), rng.gen()),
            type_tag: BinaryRequestTag::random(rng).into(),
        }
    }
}

impl ToBytes for BinaryRequestHeader {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut buffer = bytesrepr::allocate_buffer(self)?;
        self.write_bytes(&mut buffer)?;
        Ok(buffer)
    }

    fn write_bytes(&self, writer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        self.protocol_version.write_bytes(writer)?;
        self.type_tag.write_bytes(writer)
    }

    fn serialized_length(&self) -> usize {
        self.protocol_version.serialized_length() + self.type_tag.serialized_length()
    }
}

impl FromBytes for BinaryRequestHeader {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (protocol_version, remainder) = FromBytes::from_bytes(bytes)?;
        let (type_tag, remainder) = u8::from_bytes(remainder)?;
        Ok((
            BinaryRequestHeader {
                protocol_version,
                type_tag,
            },
            remainder,
        ))
    }
}

/// A request to the binary access interface.
#[derive(Clone, Debug, PartialEq)]
pub enum BinaryRequest {
    /// Request to get data from the node
    Get(GetRequest),
    /// Request to add a transaction into a blockchain.
    TryAcceptTransaction {
        /// Transaction to be handled.
        transaction: Transaction,
    },
    /// Request to execute a transaction speculatively.
    TrySpeculativeExec {
        /// Transaction to execute.
        transaction: Transaction,
    },
}

impl BinaryRequest {
    /// Returns the type tag of the request.
    pub fn tag(&self) -> BinaryRequestTag {
        match self {
            BinaryRequest::Get(_) => BinaryRequestTag::Get,
            BinaryRequest::TryAcceptTransaction { .. } => BinaryRequestTag::TryAcceptTransaction,
            BinaryRequest::TrySpeculativeExec { .. } => BinaryRequestTag::TrySpeculativeExec,
        }
    }

    #[cfg(test)]
    pub(crate) fn random(rng: &mut TestRng) -> Self {
        match BinaryRequestTag::random(rng) {
            BinaryRequestTag::Get => Self::Get(GetRequest::random(rng)),
            BinaryRequestTag::TryAcceptTransaction => Self::TryAcceptTransaction {
                transaction: Transaction::random(rng),
            },
            BinaryRequestTag::TrySpeculativeExec => Self::TrySpeculativeExec {
                transaction: Transaction::random(rng),
            },
        }
    }
}

impl ToBytes for BinaryRequest {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut buffer = bytesrepr::allocate_buffer(self)?;
        self.write_bytes(&mut buffer)?;
        Ok(buffer)
    }

    fn write_bytes(&self, writer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        match self {
            BinaryRequest::Get(inner) => inner.write_bytes(writer),
            BinaryRequest::TryAcceptTransaction { transaction } => transaction.write_bytes(writer),
            BinaryRequest::TrySpeculativeExec { transaction } => transaction.write_bytes(writer),
        }
    }

    fn serialized_length(&self) -> usize {
        match self {
            BinaryRequest::Get(inner) => inner.serialized_length(),
            BinaryRequest::TryAcceptTransaction { transaction } => transaction.serialized_length(),
            BinaryRequest::TrySpeculativeExec { transaction } => transaction.serialized_length(),
        }
    }
}

impl TryFrom<(BinaryRequestTag, &[u8])> for BinaryRequest {
    type Error = bytesrepr::Error;

    fn try_from((tag, bytes): (BinaryRequestTag, &[u8])) -> Result<Self, Self::Error> {
        let (req, remainder) = match tag {
            BinaryRequestTag::Get => {
                let (get_request, remainder) = FromBytes::from_bytes(bytes)?;
                (BinaryRequest::Get(get_request), remainder)
            }
            BinaryRequestTag::TryAcceptTransaction => {
                let (transaction, remainder) = FromBytes::from_bytes(bytes)?;
                (
                    BinaryRequest::TryAcceptTransaction { transaction },
                    remainder,
                )
            }
            BinaryRequestTag::TrySpeculativeExec => {
                let (transaction, remainder) = FromBytes::from_bytes(bytes)?;
                (BinaryRequest::TrySpeculativeExec { transaction }, remainder)
            }
        };
        if !remainder.is_empty() {
            return Err(bytesrepr::Error::LeftOverBytes);
        }
        Ok(req)
    }
}

/// The type tag of a binary request.
#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum BinaryRequestTag {
    /// Request to get data from the node
    Get = 0,
    /// Request to add a transaction into a blockchain.
    TryAcceptTransaction = 1,
    /// Request to execute a transaction speculatively.
    TrySpeculativeExec = 2,
}

impl BinaryRequestTag {
    /// Creates a random `BinaryRequestTag`.
    #[cfg(test)]
    pub fn random(rng: &mut TestRng) -> Self {
        match rng.gen_range(0..3) {
            0 => BinaryRequestTag::Get,
            1 => BinaryRequestTag::TryAcceptTransaction,
            2 => BinaryRequestTag::TrySpeculativeExec,
            _ => unreachable!(),
        }
    }
}

impl TryFrom<u8> for BinaryRequestTag {
    type Error = InvalidBinaryRequestTag;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BinaryRequestTag::Get),
            1 => Ok(BinaryRequestTag::TryAcceptTransaction),
            2 => Ok(BinaryRequestTag::TrySpeculativeExec),
            _ => Err(InvalidBinaryRequestTag(value)),
        }
    }
}

impl From<BinaryRequestTag> for u8 {
    fn from(value: BinaryRequestTag) -> Self {
        value as u8
    }
}

/// Error raised when trying to convert an invalid u8 into a `BinaryRequestTag`.
// Move to Error?
pub struct InvalidBinaryRequestTag(u8);

#[cfg(test)]
mod tests {
    use super::*;
    use casper_types::testing::TestRng;
    use tokio_util::codec::{Decoder, Encoder};

    #[test]
    fn header_bytesrepr_roundtrip() {
        let rng = &mut TestRng::new();

        let val = BinaryRequestHeader::random(rng);
        bytesrepr::test_serialization_roundtrip(&val);
    }

    #[test]
    fn request_bytesrepr_roundtrip() {
        let rng = &mut TestRng::new();

        let val = BinaryRequest::random(rng);
        let bytes = val.to_bytes().expect("should serialize");
        assert_eq!(BinaryRequest::try_from((val.tag(), &bytes[..])), Ok(val));
    }

    #[test]
    fn binary_request_codec() {
        let rng = &mut TestRng::new();
        let val = BinaryRequest::random(rng);

        let mut codec = BinaryRequestCodec {};

        let mut bytes = bytes::BytesMut::new();
        codec
            .encode(val.clone(), &mut bytes)
            .expect("should encode");

        let decoded = codec
            .decode(&mut bytes)
            .expect("should decode")
            .expect("should be Some");

        assert_eq!(val, decoded);
    }
}
