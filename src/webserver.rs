mod socket_handler;

pub mod responses;
pub mod shared;
pub mod requests;
mod     clf;

use async_std::{
    net::TcpListener,
    task
};

use std::io;

use log::*;

use crate::CONFIG;
use socket_handler::SocketHandler;

pub struct WebServer {
    listener: TcpListener,
}

impl WebServer {
    pub async fn new() -> io::Result<Self> {
        info!("creating new webserver...");
        let addr = format!(
            "{}:{}",
            CONFIG.get_str("addr").unwrap(),
            CONFIG.get_int("port").unwrap()
        );

        let listener = TcpListener::bind(&addr).await?;
        info!("bound to addr '{}' successfully", addr);

        Ok(WebServer{
            listener: listener,
        })
    }

    pub async fn listen(&mut self) -> io::Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    trace!("new connection received: '{}'", addr);
                    let handler = SocketHandler::new(
                        stream,
                    )?;

                    task::spawn(handler.dispatch());
                },
                Err(err) => {
                    use io::ErrorKind;

                    match err.kind() {
                        ErrorKind::WouldBlock => (),
                        _ => error!(
                            "error occured while accepting connection: '{}'",
                            err
                        ),
                    }
                }
            }
        }
    }
}
