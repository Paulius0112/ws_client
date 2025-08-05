use thiserror::Error;
use std::io;

use crate::handshake::error::HandshakeError;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum StreamError {
    #[error("Invalid request")]
    InvalidRequest,

    #[error("Invalid URL scheme")]
    InvalidScheme,

    #[error("No hostname in URL")]
    NoHostname,

    #[error("Empty buffer")]
    EmptyBuff,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Operation would block")]
    WouldBlock,

    #[error("Handshake failed: {0}")]
    Handshake(#[from] HandshakeError),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Tls handshake failed: {0}")]
    TlsHandshake(String),

    #[error("Tcp handshake failed: {0}")]
    TcpConnection(String),

    #[error("Failed to resolve DNS name")]
    DnsResolve
}
