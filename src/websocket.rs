use std::{io::{Read, Write}, net::TcpStream};

use native_tls::TlsStream;
use thiserror::Error;

use crate::{error::StreamError, frame::Frame, message::Message, transport::Framed};



#[allow(dead_code)]
pub enum SocketState {
    Init,
    Connecting,
    Connected,
    Close,
}

#[allow(dead_code)]
pub struct WebSocket {
    inner: Framed<MaybeTlsStream>,
    state: SocketState,
}

#[allow(dead_code)]
impl WebSocket {
    pub fn new(framed: Framed<MaybeTlsStream>) -> Self {
        Self {
            inner: framed,
            state: SocketState::Init,
        }
    }

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

pub enum MaybeTlsStream 
{
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>)
}


impl Read for MaybeTlsStream
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            MaybeTlsStream::Plain(plain) => plain.read(buf),
            MaybeTlsStream::Tls(tls) => tls.read(buf)
        }
    }   
}

impl Write for MaybeTlsStream
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


#[derive(Error, Debug)]
pub enum ParseError {
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
}
