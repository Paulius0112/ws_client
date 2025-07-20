use thiserror::Error;
use log::info;

#[allow(dead_code)]
#[derive(Debug)]
pub enum OpCode {
    Continuation = 0x00,
    Text = 0x01,
    Binary = 0x02,
    Reserved = 0x03,
    Close = 0x08,
    Ping = 0x09,
    Pong = 0x0A,
    Control = 0x0B,
    NotImplemented,
}

#[allow(dead_code)]
impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        match value {
            0x01 => OpCode::Text,
            0x02 => OpCode::Binary,
            0x08 => OpCode::Close,
            0x09 => OpCode::Ping,
            0x0A => OpCode::Pong,
            0x03..=0x07 => OpCode::Reserved,
            0x0B..=0x0F => OpCode::Control,
            _ => OpCode::NotImplemented,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Frame {
    pub fin: bool,
    pub opcode: OpCode,
    pub mask: bool,
    pub payload: Vec<u8>,
}

// TODO: We might not need this anymore
#[allow(dead_code)]
impl From<&Vec<u8>> for Frame {
    fn from(bytes: &Vec<u8>) -> Self {
        let b0 = bytes[0];
        let b1 = bytes[1];

        let fin = ((0b1000_0000 & b0) >> 7) == 1;
        let opcode: OpCode = (0b0000_1111 & b0).into();
        let mask = (0b1000_0000 & b1) >> 7 == 1;
        let payload = bytes[2..bytes.len()].to_vec();

        Frame {
            fin,
            opcode,
            mask,
            payload,
        }
    }
}

#[allow(dead_code)]
impl Frame {
    pub fn try_parse(bytes: &Vec<u8>) -> Result<Frame, FrameError> {
        if bytes.len() < 2 {
            //println!("Frame atleast should have a header...");
            return Err(FrameError::InvalidHeader);
        }

        let b0 = bytes[0];
        let b1 = bytes[1];
        let fin = ((0b1000_0000 & b0) >> 7) == 1;
        let opcode: OpCode = (0b0000_1111 & b0).into();
        let mask = (0b1000_0000 & b1) >> 7 == 1;
        let len = (0b0111_1111 & b1) as usize;

        let payload = bytes[2..bytes.len()].to_vec();

        if len > payload.len() {
            info!("Frame cannot be parsed yet. Need additional payload");
            return Err(FrameError::IncompleteFramePayload);
        }

        Ok(Frame {
            fin,
            opcode,
            mask,
            payload,
        })
    }
    pub fn text(msg: String) -> Frame {
        Frame {
            fin: true,
            opcode: OpCode::Text,
            mask: false,
            payload: msg.as_bytes().to_vec(),
        }
    }

    pub fn binary(bytes: Vec<u8>) -> Frame {
        Frame {
            fin: true,
            opcode: OpCode::Binary,
            mask: false,
            payload: bytes,
        }
    }
}

#[allow(dead_code)]
impl Frame {
    pub fn encode(&self, mask: [u8; 4]) -> Vec<u8> {
        let n: usize = self.payload.len();
        let mut masked: Vec<u8> = Vec::with_capacity(n);
        let mut headers = Vec::with_capacity(6);

        let fin_bit = 0x80;
        let masked_bit = 0x80;

        info!("Encoding the payload of lenght: {}", self.payload.len());
        for i in 0..self.payload.len() {
            let encoded = self.payload[i] ^ mask[i % 4];
            masked.push(encoded);
        }

        // Replace with enum
        headers.push(fin_bit | 0x01);
        headers.push(masked_bit | (n as u8));
        headers.extend_from_slice(&mask);
        headers.extend_from_slice(&masked);

        headers
    }

    pub fn decode(bytes: &Vec<u8>) -> Result<Frame, FrameError> {
        info!("Got bytes in decode: {}", bytes.len());
        let frame: Frame = bytes.into();

        if frame.mask {
            info!("Frame is masked. Wrong");
            return Err(FrameError::MaskedResponse);
        }

        Ok(frame)
    }
}

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum FrameError {
    #[error("Response is masked. Invalid")]
    MaskedResponse,

    #[error("Frame cannot be parsed yet")]
    CannotParse,

    #[error("Frame contains invalid header")]
    InvalidHeader,

    #[error("Frame payload is not complete")]
    IncompleteFramePayload,
}
