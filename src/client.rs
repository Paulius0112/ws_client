use std::{io::{Read, Write}, net::{SocketAddr, TcpStream, ToSocketAddrs}};

use crate::{frame::Frame, handshake::{HandshakeClient, HandshakeProgress}};
use native_tls::{HandshakeError, MidHandshakeTlsStream, TlsConnector, TlsStream};
use thiserror::Error;
use url::{Host, Url};
use crate::{
    error::StreamError,
    message::Message,
    transport::Framed,
};
use log::{info, warn};

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

pub struct ConnectionClient {
    url: Url,
    socket: SocketAddr,
    state: ConnectionPhase,
    host_str: String,
    connector: TlsConnector,
    pending_tls: Option<MidHandshakeTlsStream<TcpStream>>,
    pending_handshake: Option<HandshakeClient>,
}
enum ConnectionPhase {
    TcpConnecting,
    TlsHandshaking,
    WebSocketHandshaking,
    Done,
    Failed(StreamError),
}


pub fn connect(url: Url) -> Result<WebSocket, StreamError> {
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

    let _query = match url.query() {
        Some(q) => q,
        None => ""
    };
   
    let socket = format!("{}:{}", host, port)
        .to_socket_addrs()?
        .next()
        .ok_or(StreamError::DnsResolve)?;

    let connector = TlsConnector::new().unwrap();

    let mut client = ConnectionClient {
        url: url,
        host_str: host_str,
        socket,
        connector,
        state: ConnectionPhase::TcpConnecting,
        pending_tls: None,
        pending_handshake: None,
    };

    loop {
        match client.poll_once() {
            Some(Ok(ws)) => return Ok(ws),
            Some(Err(e)) => return Err(e),
            None => {
                // we shouldn't sleep here in prod. Just for simulation
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }
}

// TODO:
// Return maybe status instead of none?
// Update error types

impl ConnectionClient {
    pub fn poll_once(&mut self) -> Option<Result<WebSocket, StreamError>> {
        match self.state {
            ConnectionPhase::TcpConnecting => {
                let stream = TcpStream::connect(self.socket).expect("Failed to initialise stream");
                stream.set_nonblocking(true).expect("Failed to set stream to nonblocking mode");

                if self.url.scheme() == "wss" {
                    info!("Opening secury websocket stream");

                    match self.connector.connect(&self.host_str, stream) {
                        Ok(tls) => {
                            self.pending_handshake = Some(HandshakeClient::new(&self.url, MaybeTlsStream::Tls(tls)));
                            self.state = ConnectionPhase::WebSocketHandshaking;
                        }
                        Err(HandshakeError::WouldBlock(mid)) => {
                            self.pending_tls = Some(mid);
                            self.state = ConnectionPhase::TlsHandshaking;
                        }
                        Err(e) => {
                            warn!("Tcp connection failed: {}", e);
                            self.state = ConnectionPhase::Failed(StreamError::TcpConnection);
                        }
                    }
                } else {
                    info!("Opening plain websocket stream");
                    self.pending_handshake = Some(HandshakeClient::new(&self.url, MaybeTlsStream::Plain(stream)));
                    self.state = ConnectionPhase::WebSocketHandshaking;
                }

                None
            }

            ConnectionPhase::TlsHandshaking => {
                if let Some(mid) = self.pending_tls.take() {
                    match mid.handshake() {
                        Ok(tls) => {
                            self.pending_handshake = Some(HandshakeClient::new(&self.url, MaybeTlsStream::Tls(tls)));
                            self.state = ConnectionPhase::WebSocketHandshaking;
                        }
                        Err(HandshakeError::WouldBlock(mh)) => {
                            self.pending_tls = Some(mh);
                        }
                        Err(e) => {
                            warn!("Failed TLS handshake: {}", e);
                            self.state = ConnectionPhase::Failed(StreamError::TlsHandshake);
                        }
                    }
                }
                None
            }

            ConnectionPhase::WebSocketHandshaking => {
                if let Some(handshaker) = &mut self.pending_handshake {
                    match handshaker.poll_once() {
                        HandshakeProgress::Pending => None,
                        HandshakeProgress::Complete(ws) => {
                            self.state = ConnectionPhase::Done;
                            Some(Ok(ws))
                        }
                        HandshakeProgress::Error(e) => {
                            warn!("Failed handshake: {}", e);
                            self.state = ConnectionPhase::Failed(StreamError::Handshake(e));
                            
                            Some(Err(StreamError::TlsHandshake))
                        }
                    }
                } else {
                    None
                }
            }

            ConnectionPhase::Failed(ref e) => Some(Err(StreamError::TlsHandshake)),
            ConnectionPhase::Done => None,
        }
    }

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
