use std::{
    net::TcpListener,
    thread::{sleep, spawn},
    time::Duration,
};

use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
    Message,
};

fn main() {
    let server = TcpListener::bind("127.0.0.1:3012").unwrap();
    for stream in server.incoming() {
        spawn(move || {
            println!("New thread spawned");
            let callback = |req: &Request, mut response: Response| {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.uri().path());
                println!("The request's headers are:");
                for (header, _value) in req.headers() {
                    println!("* {header}");
                }

                let headers = response.headers_mut();
                headers.append("MyCustomHeader", ":)".parse().unwrap());
                headers.append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());

                Ok(response)
            };

            let stream = stream.unwrap();

            let mut websocket = accept_hdr(stream, callback).unwrap();
            println!("Stream initiated");
            println!("Continue sending messages..");

            loop {
                let message: Message = Message::text("gdgdfgdf".to_string());
                websocket.send(message).unwrap();
                sleep(Duration::from_secs(3));
            }
        });
    }
}
