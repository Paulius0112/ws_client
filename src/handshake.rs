use crate::{
    client::WebSocket,
    error::StreamError,
    transport::{Framed, Transport},
};
use bytes::BytesMut;
use log::info;
use std::{
    io::{ErrorKind, Read, Write},
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
}

impl HandshakeClient {
    pub fn new(url: &str) -> Self {
        let parsed = url::Url::parse(url)
            .expect("Invalid URL for WebSocket handshake");
        let host = parsed.host_str().unwrap_or("localhost");

        let mut request = Vec::new();
        let path = parsed.path();
        write!(request, "GET {} HTTP/1.1\r\n", path).unwrap();
        write!(request, "Host: {}\r\n", host).unwrap();
        write!(request, "Upgrade: websocket\r\n").unwrap();
        write!(request, "Connection: Upgrade\r\n").unwrap();
        write!(request, "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n").unwrap();
        write!(request, "Origin: {}\r\n", host).unwrap();
        write!(request, "Sec-WebSocket-Version: 13\r\n").unwrap();
        write!(request, "\r\n").unwrap();

        HandshakeClient {
            state: HandshakeState::Sending { request, offset: 0 },
        }
    }

    pub fn handshake<S: Transport>(mut self, mut stream: S) -> Result<WebSocket<S>, StreamError> {
        loop {
            match &mut self.state {
                HandshakeState::Sending { request, offset } => {
                    match stream.write(&request[*offset..]) {
                        Ok(0) => {
                            return Err(StreamError::ConnectionClosed);
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
                            sleep(Duration::from_millis(5));
                        }
                        Err(e) => return Err(StreamError::Io(e)),
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
                        Ok(0) => return Err(StreamError::ConnectionClosed),
                        Ok(n) => {
                            buf.extend_from_slice(&tmp[..n]);
                            info!("Received {} bytes", n);
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(e) => return Err(StreamError::Io(e)),
                    }

                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        info!("End of handshake response detected");
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