use std::{collections::HashMap, io::{BufRead, BufReader, Error, Read, Write}, marker::PhantomData, net::TcpStream, thread::sleep, time::Duration};
use tungstenite::{handshake::{headers::MAX_HEADERS, server::Response}, http};
use url::Url;

use crate::{client::WebSocket, error::StreamError, transport::{Framed, Transport}};

pub struct HandshakeResponse {
    pub version: u8,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: HashMap<String, String>,
}

impl HandshakeResponse {
    pub fn from_raw(raw: &[u8]) -> Result<Self, StreamError> {
        let mut headers_storage = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut res = httparse::Response::new(&mut headers_storage);

        let status = res.parse(raw)
            .map_err(|_| StreamError::InvalidRequest)?;
        if !status.is_complete() {
            return Err(StreamError::InvalidRequest);
        }


        let status = res.code.unwrap();
        let version = res.version.unwrap();
        let reason = res.reason.unwrap().to_string();

        let mut headers = HashMap::new();

        for h in res.headers.iter() {
            let key = h.name.to_string();
            let val = String::from_utf8_lossy(h.value).into_owned();
            println!("Headers: {}:{}", key, val);

            headers.insert(key, val);
        }

        Ok(Self {
            version,
            status_code: status,
            reason_phrase: reason,
            headers
        })
    }
}


// TODO: We need to make this non-blocking
pub fn client_handshake<S: Transport>(mut stream: S, url: &Url) -> Result<WebSocket<S>, StreamError> {
    let mut headers = String::new();
    headers.push_str("GET / HTTP/1.1\n");
    headers.push_str("Host: localhost\n");
    headers.push_str("Upgrade: websocket\n");
    headers.push_str("Connection: Upgrade\n");   
    headers.push_str("Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\n");
    headers.push_str("Origin: 127.0.0.1\n");
    headers.push_str("Sec-WebSocket-Version: 13\n");
    headers.push_str("\r\n");

    // Lets do not blocking way
    stream.write_all(&headers.as_bytes()).unwrap();

    let mut reader = BufReader::new(&mut stream);
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];

    loop {
        match reader.read(&mut buf) {
            Ok(0) => return Err(StreamError::InvalidRequest),
            Ok(n) => raw.extend_from_slice(&buf[..n]),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                sleep(Duration::from_millis(50));
                continue;
            }
            Err(_) => return Err(StreamError::InvalidRequest),
        }

        if raw.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }


    let resp = HandshakeResponse::from_raw(&raw).unwrap();

    // TODO: Perform validation on response
    
    // let method = resp.method.unwrap();
    // if method != "GET" {
    //     println!("Response method is incorrect");
    // }

    let version = resp.version;
    println!("Got version: {}", version);

    let framed = Framed::new(stream);

    Ok(WebSocket::new(framed))
}
