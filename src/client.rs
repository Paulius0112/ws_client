use std::{net::TcpStream, time::Duration};

use crate::{frame::Frame, handshake::HandshakeClient};
use thiserror::Error;
use tungstenite::http::HeaderMap;
use url::Url;
use crate::{
    error::StreamError,
    message::Message,
    transport::{Framed, Transport},
};
use log::info;

#[allow(dead_code)]
pub enum SocketState {
    Init,
    Connecting,
    Connected,
    Close,
}

#[allow(dead_code)]
pub struct WebSocket<T: Transport> {
    inner: Framed<T>,
    state: SocketState,
}

#[allow(dead_code)]
impl<T: Transport> WebSocket<T> {
    pub fn new(framed: Framed<T>) -> Self {
        Self {
            inner: framed,
            state: SocketState::Init,
        }
    }

    // receive and close funcs
    pub fn send(&mut self, msg: Message) {
        let frame = match msg {
            Message::Text(string) => Frame::text(string),
            Message::Binary(binary) => Frame::binary(binary),
        };

        info!("Sending Frame: {:?}", frame);

        self.inner.send_frame(frame).unwrap();
    }

    pub fn recv(&mut self) -> Result<Option<Message>, StreamError> {
        let next_frame = self.inner.next_frame().unwrap();

        if let Some(frame) = next_frame {
            let msg = match frame.opcode {
                crate::frame::OpCode::Text => Message::text(frame.payload),
                crate::frame::OpCode::Binary => Message::binary(frame.payload),
                _ => unimplemented!(),
            };

            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
}

#[allow(dead_code)]
pub struct ClientBuilder {
    url: Url,
    timeout: Duration,
    tls: bool,
    headers: HeaderMap,
}

#[allow(dead_code)]
impl ClientBuilder {
    pub fn new(url: &str) -> Result<Self, ParseError> {
        let url = Url::parse(url)?;

        match url.scheme() {
            "ws" | "wss" => {}
            _ => return Err(ParseError::UnsupportedScheme(url.scheme().into())),
        }

        let headers = HeaderMap::new();

        Ok(Self {
            url: url.clone(),
            timeout: Duration::from_secs(5),
            tls: url.scheme() == "wss",
            headers,
        })
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }

    pub fn header(self, name: &str, value: &str) -> Self {
        // TOPO
        self
    }

    pub fn connect(self) -> Result<WebSocket<impl Transport>, StreamError> {
        let host = self.url.host().unwrap();
        let port = self.url.port().unwrap();

        let endpoint = format!("{}:{}", host, port);
        info!("Endpoint to connect: {}", endpoint);

        let stream = TcpStream::connect(endpoint.clone()).unwrap();
        info!("Setting stream as non blocking...");
        stream.set_nonblocking(true).unwrap();

        let machine = HandshakeClient::new(&endpoint);
        return machine.handshake(stream)
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Unsupported scheme: {0}")]
    UnsupportedScheme(String),
}
