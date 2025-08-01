use crate::{
    client::{MaybeTlsStream, WebSocket},
    transport::{Framed, Transport},
};
use base64::Engine;
use bytes::BytesMut;
use httparse::{Response, Status};
use log::info;
use rand::Rng;
use sha1::Digest;
use thiserror::Error;
use std::{
    io::{ErrorKind, Write, Read},
    thread::sleep,
    time::Duration,
};

pub enum HandshakeState {
    Sending { request: Vec<u8>, offset: usize },
    Flushing,
    Receiving { buf: BytesMut },
    Done,
}

pub struct HandshakeClient {
    state: HandshakeState,
    sec_key: String
}

impl HandshakeClient {
    pub fn new(url: &str) -> Self {
        let parsed = url::Url::parse(url)
            .expect("Invalid URL for WebSocket handshake");
        let host = parsed.host_str().unwrap_or("localhost");

        let raw: [u8; 16] = rand::rng().random();
        let sec_key = base64::engine::general_purpose::STANDARD.encode(&raw);

        let mut request = Vec::new();
        let path = parsed.path();
        write!(request, "GET {} HTTP/1.1\r\n", path).unwrap();
        write!(request, "Host: {}\r\n", host).unwrap();
        write!(request, "Upgrade: websocket\r\n").unwrap();
        write!(request, "Connection: Upgrade\r\n").unwrap();
        write!(request, "Sec-WebSocket-Key: {}\r\n", sec_key).unwrap();
        write!(request, "Origin: {}\r\n", host).unwrap();
        write!(request, "Sec-WebSocket-Version: 13\r\n").unwrap();
        write!(request, "\r\n").unwrap();

        HandshakeClient {
            state: HandshakeState::Sending { request, offset: 0 },
            sec_key
        }
    }

    pub fn handshake<Stream: Transport>(mut self, mut stream: MaybeTlsStream<Stream>) -> Result<WebSocket<Stream>, HandshakeError> {
        loop {
            match &mut self.state {
                HandshakeState::Sending { request, offset } => {
                    match stream.write(&request[*offset..]) {
                        Ok(0) => {
                            return Err(HandshakeError::ConnectionClosed);
                        }
                        Ok(n) => {
                            *offset += n;
                            info!("Sent {} bytes of {}", *offset, request.len());
                            if *offset >= request.len() {
                                self.state = HandshakeState::Flushing;
                            }
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            // Can't write now, retry shortly
                            // TODO: Update with non blocking way
                            sleep(Duration::from_millis(5));
                        }
                        Err(e) => return Err(HandshakeError::Io(e)),
                    }
                }

                HandshakeState::Flushing => {
                    stream.flush()?;
                    info!("Flushed handshake request");
                    self.state = HandshakeState::Receiving { buf: BytesMut::with_capacity(1024) };
                }

                HandshakeState::Receiving { buf } => {
                    let mut tmp = [0_u8; 1024];
                    match stream.read(&mut tmp) {
                        Ok(0) => return Err(HandshakeError::ConnectionClosed),
                        Ok(n) => {
                            buf.extend_from_slice(&tmp[..n]);
                            info!("Received {} bytes", n);
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(e) => return Err(HandshakeError::Io(e)),
                    }

                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        info!("End of handshake response detected");
                        const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

                        let mut headers = [httparse::EMPTY_HEADER; 32];
                        let mut parsed = Response::new(
                            &mut headers
                        );

                        let status = parsed.parse(&buf).unwrap();
                        let _body_start = match status {
                            Status::Complete(id) => id,
                            Status::Partial => return Err(HandshakeError::Incomplete),
                        };

                        // Check for Connection and Upgrade header
                        let code = parsed.code.unwrap_or(0);
                        if code != 101 {
                            return Err(HandshakeError::BadStatus)
                        }

                        let accept_hdr = parsed.headers
                            .iter()
                            .find(|k| k.name == "sec-websocket-accept")
                            .ok_or(HandshakeError::MissingAcceptHeader)?;

                        let mut hasher = sha1::Sha1::new();
                        hasher.update(self.sec_key.as_bytes());
                        hasher.update(GUID.as_bytes());

                        let expected = base64::engine::general_purpose::STANDARD.encode(&hasher.finalize());

                        let actual = std::str::from_utf8(accept_hdr.value)
                            .map_err(|_| HandshakeError::BadAccept)?;
                        if actual != expected {
                            return Err(HandshakeError::BadAccept);
                        }

                        self.state = HandshakeState::Done;
                    }
                }

                HandshakeState::Done => {
                    let framed = Framed::new(stream);
                    return Ok(WebSocket::new(framed));
                }
            }
        }
    }
}



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