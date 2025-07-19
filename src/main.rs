use crate::{client::ClientBuilder, message::Message};
use log::info;

mod client;
mod error;
mod frame;
mod handshake;
mod message;
mod transport;

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
            info!("Received: {:?}", val);
        }
    }
}
