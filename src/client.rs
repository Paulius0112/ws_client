use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use crate::{handshake::handshake::{HandshakeClient, HandshakeProgress}, websocket::{MaybeTlsStream, WebSocket}};
use native_tls::{HandshakeError, MidHandshakeTlsStream, TlsConnector};
use url::{Host, Url};
use crate::error::StreamError;
use log::{info, debug};

pub struct ConnectionClient {
    url: Url,
    socket: SocketAddr,
    state: ConnectionPhase,
    host_str: String,
    connector: TlsConnector,
    pending_tls: Option<MidHandshakeTlsStream<TcpStream>>,
    pending_handshake: Option<HandshakeClient>,
    scheme: SchemeType
}
enum ConnectionPhase {
    TcpConnecting,
    TlsHandshaking,
    WebSocketHandshaking,
    Done,
}

#[allow(dead_code)]
pub fn connect_blocking(url: Url) -> Result<WebSocket, StreamError> {
    connect(url)?.connect_blocking()
}

#[allow(dead_code)]
pub fn connect(url: Url) -> Result<ConnectionClient, StreamError> {
    let host = url.host().expect("Host not found");

    let host_str = match host {
        Host::Domain(domain) => domain.to_string(),
        Host::Ipv4(ip) => ip.to_string(),
        Host::Ipv6(ip) => ip.to_string(),
    };

    let scheme = match url.scheme() {
        "wss" => SchemeType::WSS,
        "ws" => SchemeType::WS,
        _ => return Err(StreamError::InvalidScheme),
    };
    
    let port = url.port().unwrap_or_else(|| match scheme {
        SchemeType::WS => 80,
        SchemeType::WSS => 443,
    });

    let socket = {
        let mut addr_buf = String::with_capacity(host_str.len() + 6);
        use std::fmt::Write;
        write!(addr_buf, "{}:{}", host_str, port).unwrap();

        addr_buf
            .to_socket_addrs()?
            .next()
            .ok_or(StreamError::DnsResolve)?
    };

    let connector = TlsConnector::new().unwrap();

    let client = ConnectionClient {
        url: url,
        host_str: host_str,
        socket,
        connector,
        state: ConnectionPhase::TcpConnecting,
        pending_tls: None,
        pending_handshake: None,
        scheme,
    };

    return Ok(client);
}

#[derive(PartialEq)]
pub enum SchemeType {
    WSS,
    WS
}

impl ConnectionClient {
    pub fn connect_blocking(mut self) -> Result<WebSocket, StreamError> {
        loop {
            match self.poll_once() {
                Some(Ok(ws)) => return Ok(ws),
                Some(Err(e)) => return Err(e),
                None => {},
            }
        }
    }

    pub fn poll_once(&mut self) -> Option<Result<WebSocket, StreamError>> {
        match self.state {
            ConnectionPhase::TcpConnecting => {
                debug!("TcpConnecting");
                let stream = TcpStream::connect(self.socket).expect("Failed to initialise stream");
                stream.set_nonblocking(true).expect("Failed to set stream to nonblocking mode");

                if self.scheme == SchemeType::WSS {
                    info!("Opening secure websocket stream");

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
                            return Some(Err(StreamError::TcpConnection(e.to_string())));
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
                debug!("TlsConnecting");
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
                            return Some(Err(StreamError::TlsHandshake(e.to_string())));
                        }
                    }
                }
                None
            }

            ConnectionPhase::WebSocketHandshaking => {
                debug!("WebSocketHandshaking");
                if let Some(handshaker) = &mut self.pending_handshake {
                    match handshaker.poll_once() {
                        HandshakeProgress::Pending => None,
                        HandshakeProgress::Complete(ws) => {
                            self.state = ConnectionPhase::Done;
                            Some(Ok(ws))
                        }
                        HandshakeProgress::Error(e) => {
                            return Some(Err(StreamError::Handshake(e)))
                        }
                    }
                } else {
                    None
                }
            },
            ConnectionPhase::Done => None,
        }
    }

}
