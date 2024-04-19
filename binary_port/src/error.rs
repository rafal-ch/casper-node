use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid request tag ({0})")]
    InvalidBinaryRequestTag(u8),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    BytesRepr(#[from] casper_types::bytesrepr::Error),
}
