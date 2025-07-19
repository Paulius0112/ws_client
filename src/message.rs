#[derive(Debug)]
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
}

impl Message {
    pub fn text(buf: Vec<u8>) -> Self {
        let msg = String::from_utf8_lossy(&buf).to_string();

        Message::Text(msg)
    }

    pub fn binary(buf: Vec<u8>) -> Self {
        Message::Binary(buf)
    }
}
