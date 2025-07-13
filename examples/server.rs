use std::{net::TcpListener, thread::{sleep, spawn}, time::Duration};

use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response}, Message,
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

                // Let's add an additional header to our response to the client.
                let headers = response.headers_mut();
                headers.append("MyCustomHeader", ":)".parse().unwrap());
                headers.append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());

                Ok(response)
            };

            let stream = stream.unwrap();
            
            let mut websocket = accept_hdr(stream, callback).unwrap();
            println!("Stream initiated");
            loop {
                let msg = websocket.read().unwrap();
                println!("Got msg: {}", msg);
                if msg.is_binary() || msg.is_text() {
                    websocket.send(msg).unwrap();
                }

                break;
            }

            println!("Continue sending messages..");

            loop {

                let message: Message = Message::text("Testing message".to_string());
                websocket.send(message).unwrap();
                sleep(Duration::from_secs(3));
            }

        });
    }
}
