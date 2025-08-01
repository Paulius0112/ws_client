use std::{io::{Read, Write}, net::TcpStream};

use crate::{frame::Frame, handshake::HandshakeClient};
use native_tls::{TlsConnector, TlsStream};
use thiserror::Error;
use url::{Host, Url};
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
    inner: Framed<MaybeTlsStream<T>>,
    state: SocketState,
}

// impl<Stream> Transport for MaybeTlsStream<Stream> {}

#[allow(dead_code)]
impl<T: Transport> WebSocket<T> {
    pub fn new(framed: Framed<MaybeTlsStream<T>>) -> Self {
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

pub enum MaybeTlsStream<Stream> 
where 
    Stream: Transport
{
    Plain(Stream),
    Tls(TlsStream<Stream>)
}


impl<Stream> Read for MaybeTlsStream<Stream>
where 
    Stream: Transport
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            MaybeTlsStream::Plain(plain) => plain.read(buf),
            MaybeTlsStream::Tls(tls) => tls.read(buf)
        }
    }   
}

impl<Stream> Write for MaybeTlsStream<Stream>
where 
    Stream: Transport
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            MaybeTlsStream::Plain(plain) => plain.write(buf),
            MaybeTlsStream::Tls(tls) => tls.write(buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            MaybeTlsStream::Plain(plain) => plain.flush(),
            MaybeTlsStream::Tls(tls) => tls.flush(),
        }
    }
}

// impl<T: Read + Write> Transport for MaybeTlsStream<T> {}

pub fn connect(url: Url) -> Result<WebSocket<impl Transport>, StreamError> {
    println!("Endpoint: {}", url);

    let host = url.host().expect("Host not found");

    let host_str = match host {
        Host::Domain(domain) => domain.to_string(),
        Host::Ipv4(ip) => ip.to_string(),
        Host::Ipv6(ip) => ip.to_string(),
    };

    let scheme = url.scheme();
    
    let port = if let Some(p) = url.port() {
        p
    } else {
        match scheme {
            "ws" => 80,
            "wss" => 443,
            _ => unimplemented!(),
        }
    };

    let query = match url.query() {
        Some(q) => q,
        None => ""
    };
   
    let endpoint = format!("{}:{}{}", host, port, query);
    info!("Endpoint to connect: {}", endpoint);

    //let stream = TcpStream::connect(endpoint.clone()).unwrap();

    let stream = match scheme {
        "ws" => {
            let stream = TcpStream::connect(endpoint.clone()).unwrap();
            let _ = stream.set_nonblocking(true);
            info!("Starting Plain stream");
            MaybeTlsStream::Plain(stream)
        },
        "wss" => {
            let connector = TlsConnector::new().unwrap();
            let stream = TcpStream::connect(endpoint.clone()).unwrap();
            let _ = stream.set_nonblocking(true);
            let tls_stream = connector.connect(host_str.as_str(), stream).unwrap();
            info!("Starting Tls stream");
            MaybeTlsStream::Tls(tls_stream)
        },
        &_ => todo!(),
    };

    // info!("Setting stream as non blocking...");
    // stream.set_nonblocking(true).unwrap();

    let machine = HandshakeClient::new(&endpoint);

    info!("Starting handshake");
    return Ok(machine.handshake(stream)?)
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("Unsupported scheme: {0}")]
    UnsupportedScheme(String),
}
