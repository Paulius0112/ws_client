use tungstenite::client::IntoClientRequest;

use crate::{client::ClientBuilder, message::Message};
use log::{info};


mod stream;
mod error;
mod handshake;
mod client;
mod transport;
mod message;
mod frame;

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting Websocket client");
    let client = ClientBuilder::new("ws://localhost:3012").unwrap();

    let mut stream = client.connect().unwrap();

    let msg = "Hello from custom client".to_string();

    let msg = Message::Text(msg);

    stream.send(msg);

    loop {
        let msg = stream.recv().unwrap();
        if let Some(val) = msg {
            println!("Received: {:?}", val);
        } else {
            println!("Empty response");
        }
        
    }

    Ok(())
    
}

