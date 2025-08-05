use thiserror::Error;

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("Handshake response was incomplete")]
    Incomplete,

    #[error("Bad status code from the server")]
    BadStatus,

    #[error("Server closed the connection")]
    ConnectionClosed,

    #[error("I/O error: {0}")]
    Io(#[source] std::io::Error),

    #[error("Missing accept header in handshake response")]
    MissingAcceptHeader,

    #[error("Handshake response contains invalid security key")]
    BadAccept
}

impl From<std::io::Error> for HandshakeError {
    fn from(e: std::io::Error) -> Self {
        HandshakeError::Io(e)
    }
}