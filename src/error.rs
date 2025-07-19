use thiserror::Error;
use std::io::ErrorKind;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum StreamError {
    #[error("Invalid request")]
    InvalidRequest,

    #[error("Invalid url scheme")]
    InvalidScheme,

    #[error("No hostname")]
    NoHostname,

    #[error("Emtpy buff")]
    EmptyBuff,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("I/O error: {0}")]
    Io(#[source] std::io::Error),

    #[error("Operation would block")]
    WouldBlock,
}


impl From<std::io::Error> for StreamError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            ErrorKind::WouldBlock => StreamError::WouldBlock,
            _ => StreamError::Io(err),
        }
    }
}