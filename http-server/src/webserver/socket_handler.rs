use std::net::{TcpStream, SocketAddr};
use std::rc::Rc;
use std::io::{Read, Write, BufRead};
use std::io;

use crate::CONFIG;

pub struct SocketHandler {
    stream: TcpStream,
    addr:   SocketAddr,
}

impl SocketHandler {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        Ok(SocketHandler {
            addr:   stream.peer_addr()?,
            stream: stream,
        })
    }

    pub fn dispatch(self) -> io::Result<()> {
        Ok(())
    }

    fn parse_request(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn respond(&mut self) -> io::Result<()> {
        Ok(())
    }
}
