use std::net::{TcpStream, SocketAddr};
use std::io::{BufRead, BufReader};
use std::io;

use crate::CONFIG;
use requests::*;
use std::error::Error;

use log::*;

type Result<T> = std::result::Result<T, SocketError>;

pub struct SocketHandler {
    stream: TcpStream,
    addr:   SocketAddr,
}

pub enum SocketError {
    IoError(std::io::Error),
    RequestError(RequestParsingError),
}

impl From<RequestParsingError> for SocketError {
    fn from(err: RequestParsingError) -> Self {
        SocketError::RequestError(err)
    }
}

impl From<std::io::Error> for SocketError {
    fn from(err: std::io::Error) -> Self {
        SocketError::IoError(err)
    }
}

impl SocketHandler {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        Ok(SocketHandler {
            addr:   stream.peer_addr()?,
            stream: stream,
        })
    }

    pub fn dispatch(mut self) -> Result<()> {
        let req = self.parse_request()?;
        self.respond(req)?;

        Ok(())
    }

    fn parse_request(&mut self) -> Result<Request> {
        let reader = BufReader::new(&mut self.stream);
        
        let mut buff = String::new();

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                break;
            }else{
                buff.push_str(&line);
            }
        }

        let request: Request = buff.parse()?;
        trace!("received request from '{}'\n{:#?}", self.addr, request);

        Ok(request)
    }

    fn respond(&mut self, req: Request) -> io::Result<()> {
        Ok(())
    }
}
