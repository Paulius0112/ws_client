use std::io::{Read, Write};

use crate::{
    error::StreamError,
    frame::{Frame, FrameError},
};
use rand::random;
use log::info;
use std::io::ErrorKind;


pub trait Transport: Write + Read {}

impl<T: Write + Read> Transport for T {}

const MIN_HEADER_SIZE: usize = 2;

#[allow(dead_code)]
pub struct Framed<S: Transport> {
    stream: S,
    read_buf: Vec<u8>,
    raw_buf: Vec<u8>,
}

#[allow(dead_code)]
impl<T: Transport> Framed<T> {
    pub fn send_frame(&mut self, frame: Frame) -> Result<(), StreamError> {
        let mask: [u8; 4] = random();

        let encoded = frame.encode(mask);

        info!("Transmiting frame...");
        self.stream.write_all(&encoded).unwrap();
        self.stream.flush().unwrap();

        Ok(())
    }

    pub fn next_frame(&mut self) -> Result<Option<Frame>, FrameError> {
        loop {
            // We might use a ring buffer
            let mut tmp = [0u8; 1024];
            match self.stream.read(&mut tmp) {
                Ok(0) => {
                    return Ok(None)
                }
                Ok(n) => {
                    info!("Read {} bytes from socket", n);

                    self.raw_buf.extend_from_slice(&tmp[..n]);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // Socket is just empty
                    return Ok(None)
                }
                Err(e) => {
                    info!("Error reading next frame: {}", e);
                }
            }

            // Only parse if we have enough data
            if self.raw_buf.len() >= MIN_HEADER_SIZE {
                match Frame::try_parse(&self.raw_buf) {
                    Ok(frame) => {
                        info!("Frame was successfully parsed!");
                        self.read_buf.extend_from_slice(&frame.payload);
                        self.raw_buf.clear();

                        if !frame.fin {
                            info!("FIN=0, this is not the last frame");
                            return Ok(None);
                        }

                        let len = self.read_buf.len();

                        // We have full frame. Return upstream
                        let full_frame = Frame {
                            fin: true,
                            opcode: frame.opcode,
                            mask: false,
                            payload: self.read_buf[..len].to_vec(),
                        };

                        self.read_buf.clear();

                        return Ok(Some(full_frame));
                    }
                    Err(e) => {
                        info!("Error parsing frame: {}", e);
                        return Err(e);
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
impl<S: Transport> Framed<S> {
    pub fn new(stream: S) -> Self {
        // lets set non blocking stream
        Self {
            stream,
            read_buf: Vec::new(),
            raw_buf: Vec::new(),
        }
    }
}
