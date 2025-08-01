use std::str::FromStr;

use crate::{message::Message};
use log::info;
use url::Url;
use crate::client::connect;

mod client;
mod error;
mod frame;
mod handshake;
mod message;
mod transport;

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting Websocket client");
    //let client = ClientBuilder::new("ws://localhost:3012").unwrap();
    //let client = ClientBuilder::new("ws://54.92.63.182:80/ws/bnbusdt@aggTrade").unwrap();
    //let url = Url::from_str("ws://localhost:3012/tesitng@/test").unwrap();
    let url = Url::from_str("wss://fstream.binance.com/ws/bnbusdt@aggTrade").unwrap();
    let mut stream = connect(url).unwrap();

    let msg = "Hello from custom client".to_string();

    let msg = Message::Text(msg);

    info!("Sending initial message");
    stream.send(msg);

    loop {
        let msg = stream.recv().unwrap();
        if let Some(val) = msg {
            info!("Received: {:?}", val);
        }
    }
}
