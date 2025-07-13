use std::io::{Read, Write};

use rand::random;
use crate::{error::StreamError, frame::{Frame, FrameError}, message::Message};



pub trait Transport: Write + Read {}

impl<T: Write + Read> Transport for T {}


pub struct Framed<S: Transport> {
    stream: S,
    read_buf: Vec<u8>,
    raw_buf: Vec<u8>,
}

impl<T: Transport> Framed<T> {
    pub fn send_frame(&mut self, frame: Frame) -> Result<(), StreamError> {
        let mask: [u8; 4] = random();

        let encoded = frame.encode(mask);

        println!("Sending frame");
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
                    println!("Buffer is empty");
                },
                Ok(n) => {
                    println!("{} bytes were read", n);

                    self.raw_buf.extend_from_slice(&tmp[..n]);
                },
                Err(e) => {
                    //println!("Error reading from stream: {}", e);
                }
            }

            match Frame::try_parse(&self.raw_buf) {
                Ok(frame) => {
                    println!("Frame was successfully parsed!");
                    self.read_buf.extend_from_slice(&frame.payload);
                    self.raw_buf.clear();
                    
                    if !frame.fin {
                        println!("FIN=0, this is not the last frame");
                        return Ok(None)
                    }

                    let len = self.read_buf.len();

                    // We have full frame. Return upstream
                    let full_frame = Frame {
                        fin: true,
                        opcode: frame.opcode,
                        mask: false,
                        payload: self.read_buf[..len].to_vec()
                    };

                    self.read_buf.clear();

                    return Ok(Some(full_frame))
                },
                Err(e) => {
                    //println!("Error parsing frame: {}", e);
                    //return Err(e);
                }
            }
        }
    }
}

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
