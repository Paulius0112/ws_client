use crate::{
    handshake::error::HandshakeError, transport::Framed, websocket::{MaybeTlsStream, WebSocket}
};
use base64::Engine;
use bytes::BytesMut;
use httparse::{Response, Status};
use log::debug;
use rand::Rng;
use sha1::Digest;
use std::{
    io::{ErrorKind, Write, Read},
};
use url::Url;

pub enum HandshakeState {
    Sending { request: Vec<u8>, offset: usize },
    Flushing,
    Receiving { buf: BytesMut },
}

pub enum HandshakeProgress {
    Pending,
    Complete(WebSocket),
    Error(HandshakeError)
}

pub struct HandshakeClient {
    state: HandshakeState,
    sec_key: String,
    stream: Option<MaybeTlsStream>
}

impl HandshakeClient {
    pub fn new(url: &Url, stream: MaybeTlsStream) -> Self {
        // We should already have working instance of host, no extract unwraping
        let host = url.host_str().unwrap_or("localhost");

        let raw: [u8; 16] = rand::rng().random();
        let sec_key = base64::engine::general_purpose::STANDARD.encode(&raw);

        let mut request = Vec::new();
        let path = url.path();

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
            sec_key,
            stream: Some(stream)
        }
    }

    fn parse_response(buf: &[u8], sec_key: &str) -> Result<(), HandshakeError> {
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

        let code = parsed.code.unwrap_or(0);

        if code != 101 {
            return Err(HandshakeError::BadStatus)
        }

        let has_upgrade = parsed.headers.iter().any(|h| h.name.eq_ignore_ascii_case("upgrade") && h.value.eq_ignore_ascii_case(b"websocket"));
        let has_connection = parsed.headers.iter().any(|h| h.name.eq_ignore_ascii_case("connection") && h.value.eq_ignore_ascii_case(b"upgrade"));

        if !has_upgrade || !has_connection {
            return Err(HandshakeError::BadStatus);
        }

        let accept_hdr = parsed.headers
            .iter()
            .find(|k| k.name.to_lowercase() == "sec-websocket-accept")
            .ok_or(HandshakeError::MissingAcceptHeader)?;

        let mut hasher = sha1::Sha1::new();
        hasher.update(sec_key.as_bytes());
        hasher.update(GUID.as_bytes());

        let expected = base64::engine::general_purpose::STANDARD.encode(&hasher.finalize());

        let actual = std::str::from_utf8(accept_hdr.value)
            .map_err(|_| HandshakeError::BadAccept)?;
        if actual != expected {
            return Err(HandshakeError::BadAccept);
        }

        Ok(())

    }

    pub fn poll_once(&mut self) -> HandshakeProgress {
        let stream = self.stream.as_mut().unwrap();

        match &mut self.state {
            HandshakeState::Sending { request, offset } => {
                match stream.write(&request[*offset..]) {
                    Ok(0) => {
                        return HandshakeProgress::Error(HandshakeError::ConnectionClosed);
                    }
                    Ok(n) => {
                        *offset += n;
                        debug!("Sent {} bytes of {}", *offset, request.len());
                        if *offset >= request.len() {
                            self.state = HandshakeState::Flushing;
                        }
                        return HandshakeProgress::Pending;
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        return HandshakeProgress::Pending;
                    }
                    Err(e) => return HandshakeProgress::Error(HandshakeError::Io(e)),
                }
            },
            HandshakeState::Flushing => {
                match stream.flush() {
                    Ok(()) => {
                        self.state = HandshakeState::Receiving { buf: BytesMut::with_capacity(1024) };
                        HandshakeProgress::Pending
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => HandshakeProgress::Pending,
                    Err(e) => HandshakeProgress::Error(HandshakeError::Io(e)),
                }
            },
            HandshakeState::Receiving { buf } => {
                let mut tmp = [0_u8; 1024];
                match stream.read(&mut tmp) {
                    Ok(0) => return HandshakeProgress::Error(HandshakeError::ConnectionClosed),
                    Ok(n) => {
                        buf.extend_from_slice(&tmp[..n]);
                        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                            match Self::parse_response(buf, &self.sec_key) {
                                Ok(()) => {
                                    let stream = self.stream.take().unwrap();
                                    let framed: Framed<MaybeTlsStream> = Framed::new(stream);
                                    let socket = WebSocket::new(framed);
                                    return HandshakeProgress::Complete(socket);
                                }
                                Err(e) => return HandshakeProgress::Error(e),
                            }
                        }
                        HandshakeProgress::Pending
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        return HandshakeProgress::Pending
                    }
                    Err(e) => return HandshakeProgress::Error(HandshakeError::Io(e)),
                }
            }
        }
    }
}