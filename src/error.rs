use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Codec error: {0}")]
    Codec(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Timeout")]
    Timeout,

    #[error("Not supported: {0}")]
    NotSupported(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Out of range: {0}")]
    OutOfRange(String),
}

pub type Result<T> = std::result::Result<T, Error>;
