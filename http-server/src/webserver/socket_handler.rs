use std::net::{TcpStream, SocketAddr};
use std::rc::Rc;
use std::io::{Read, Write, BufRead, BufReader};
use std::io;

use crate::CONFIG;
use requests::Request;

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
        let req = self.parse_request()?;
        self.respond(req);

        Ok(())
    }

    fn parse_request(&mut self) -> io::Result<Request> {
        let reader = BufReader::new(self.stream);
        
        let mut buff = String::new();
        for line in reader.lines() {
            let line = line?;
        }

        Ok(Default::default())
    }

    fn respond(&mut self, req: Request) -> io::Result<()> {
        Ok(())
    }
}
