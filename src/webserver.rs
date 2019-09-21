mod socket_handler;

use crate::CONFIG;
use std::sync::mpsc::channel;
use std::net::TcpListener;
use std::time::Duration;
use std::collections::HashMap;
use std::io;
use std::io::Write;

use log::*;

use socket_handler::SocketHandler;

pub struct WebServer {
    listener: TcpListener
}

impl WebServer {
    pub fn new() -> io::Result<Self> {
        info!("creating new webserver...");
        let addr = format!(
            "{}:{}",
            CONFIG.addr,
            CONFIG.port
        );

        let listener = TcpListener::bind(&addr)?;
        info!("bound to addr '{}' successfully", addr);

        listener.set_nonblocking(true)?;
        Ok(WebServer{
            listener: listener
        })
    }

    pub fn listen(&mut self) -> io::Result<()> {
        let mut conn_map = HashMap::new();
        let (tx, rx)     = channel();

        loop {
            io::stdout().flush()?;
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    trace!("new connection received: '{}'", addr);

                    let handler = SocketHandler::new(
                        stream
                    )?;

                    let other_tx = tx.clone();
                    let handle = std::thread::spawn(move || {
                        let res = handler.dispatch();
                        other_tx.send(addr)
                            .expect("failed to send addr");

                        res
                    });

                    conn_map.insert(addr, handle);
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

            let del = rx.recv_timeout(Duration::from_millis(10));
            match del {
                Ok(addr) => {
                    let thread = conn_map.remove(&addr)
                        .expect("attempted to unwrap a connection that did not exist");

                    match thread.join() {
                        Err(err) => {
                            error!("connection ended with error: '{:?}'", err);
                        },
                        _ => ()
                    }

                    trace!("connection '{}' terminated successfully", addr);
                },
                Err(_) => continue
            }
        }
    }
}
