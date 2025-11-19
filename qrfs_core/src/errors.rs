use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QrfsError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("serialization error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("QRFS not formatted: {0}")]
    NotFormatted(String),

    #[error("unimplemented feature: {0}")]
    Unimplemented(String),

    #[error("other error: {0}")]
    Other(String),
}
