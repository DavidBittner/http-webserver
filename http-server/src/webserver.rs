use crate::config::Config;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io;

use log::*;

pub struct WebServer<'a> {
    config: &'a Config,
    listener: TcpListener
}

impl<'a> WebServer<'a> {
    pub fn new(config: &'a Config) -> io::Result<Self> {
        info!("creating new webserver...");
        let addr = format!(
            "{}:{}",
            config.addr,
            config.port
        );

        let listener = TcpListener::bind(&addr)?;
        info!("bound to addr '{}' successfully", addr);

        Ok(WebServer{
            config: config,
            listener: listener
        })
    }

    pub fn listen(&mut self) -> io::Result<()> {
        info!("listening on addr '{}'...", self.listener.local_addr()?);

        loop {
            match self.listener.accept() {
                Ok(client) => {
                    self.handle_stream(client)?;
                },
                Err(err) => {
                    error!("error accepting connection from client: '{}'", err);
                }
            }
        }
    }

    fn handle_stream(&self, (mut stream, addr): (TcpStream, SocketAddr)) -> io::Result<()> {
        info!("new connection from client: '{}'", addr);
        let req = self.parse_request(&mut stream)?;
        let res = self.create_response(&mut stream, req)?;

        //stream.write(res);

        Ok(())
    }

    fn parse_request(&self, stream: &mut TcpStream) -> io::Result<()> {
        match stream.peer_addr() {
            Ok(addr) => trace!("parsing request from: {}", addr),
            _ => ()
        }

        Ok(())
    }

    fn create_response(&self, stream: &mut TcpStream, req: ()) -> io::Result<()> {
        Ok(())
    }
}
