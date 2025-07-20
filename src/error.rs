use thiserror::Error;

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

    #[error("Operation would block")]
    WouldBlock,

    #[error("Handshake failed: {0}")]
    Handshake(#[from] crate::handshake::HandshakeError),
}