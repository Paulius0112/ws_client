use std::{error::Error, net::TcpStream, time::Duration};

use thiserror::Error;
use tungstenite::http::HeaderMap;
use url::Url;
use crate::frame::Frame;

use crate::{error::StreamError, handshake::client_handshake, message::Message, transport::{Framed, Transport}};


pub enum SocketState {
    Init,
    Connecting,
    Connected,
    Close,
}

pub struct WebSocket<T: Transport>
{
    inner: Framed<T>,
    state: SocketState
}


impl<T: Transport> WebSocket<T> {
    pub fn new(framed: Framed<T>) -> Self {
        Self {
            inner: framed,
            state: SocketState::Init
        }
    }

    // receive and close funcs
    pub fn send(&mut self, msg: Message) {
        // conert to frame
        let frame = match msg {
            Message::Text(string) => Frame::text(string),
            Message::Binary(binary) => Frame::binary(binary),
        };

        println!("Created frame: {:?}", frame);

        self.inner.send_frame(frame).unwrap();
    }

    pub fn recv(&mut self) -> Result<Option<Message>, StreamError> {
        let next_frame = self.inner.next_frame().unwrap();

        if let Some(frame) = next_frame {

            let msg = match frame.opcode {
                crate::frame::OpCode::Text => Message::text(frame.payload),
                crate::frame::OpCode::Binary => Message::binary(frame.payload),
                _ => unimplemented!()
            };

            return Ok(Some(msg))
        } else {
            return Ok(None)
        }
    }
}



pub struct ClientBuilder {
    url: Url,
    timeout: Duration,
    tls: bool,
    headers: HeaderMap
}

impl ClientBuilder {
    pub fn new(url: &str) -> Result<Self, ParseError> {
        let url = Url::parse(url)?;

        match url.scheme() {
            "ws" | "wss" => {},
            _ => {
                return Err(ParseError::UnsupportedScheme(url.scheme().into()))
            }
        }

        let headers = HeaderMap::new();

        Ok(Self {
            url: url.clone(),
            timeout: Duration::from_secs(5),
            tls: url.scheme() == "wss",
            headers
        })
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }


    pub fn header(mut self, name: &str, value: &str) -> Self {
        // TOPO
        self
    }

    pub fn connect(self) -> Result<WebSocket<impl Transport>, StreamError> {
        let host = self.url.host().unwrap();
        let port = self.url.port().unwrap();

        let endpoint = format!("{}:{}", host, port);
        println!("Endpoint to connect: {}", endpoint);
        let stream = TcpStream::connect(endpoint).unwrap();
        println!("Setting stream as non blocking");
        let _ = stream.set_nonblocking(true).unwrap();

        // Check for tls
        let inner = client_handshake(stream, &self.url).unwrap();

        Ok(inner)

    }

    
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Unsupported scheme: {0}")]
    UnsupportedScheme(String)
}