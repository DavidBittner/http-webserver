use std::net::{TcpStream, SocketAddr};
use std::io::{BufRead, BufReader, Write};
use std::io;

use std::fmt::{Display, Formatter};
use std::error::Error;
use std::path::{Path, PathBuf};

use log::*;

use crate::CONFIG;

use requests::*;
use responses::Response;
use shared::*;

type Result<T> = std::result::Result<T, SocketError>;

lazy_static::lazy_static! {
    static ref ROOT: PathBuf = {
        lazy_static::initialize(&CONFIG);

        let root = CONFIG.get_str("root")
            .expect("root not defined (shouldn't happen)");

        PathBuf::from(root)
    };
}

pub struct SocketHandler {
    stream: TcpStream,
    addr:   SocketAddr,
}

#[derive(Debug)]
pub enum SocketError {
    IoError(std::io::Error),
    RequestError(RequestParsingError),
}

impl Display for SocketError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use SocketError::*;

        match self {
            IoError(err) => write!(f, "IoError: {}", err),
            RequestError(err) => write!(f, "{}", err)
        }
    }
}

impl Error for SocketError {}

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
        let mut conn = Connection::Close;

        loop {
            let req = self.parse_request()?;

            match req.method {
                Method::Get => {
                    let resp = self.get(&req)?;

                    conn = resp.headers.connection
                        .clone()
                        .unwrap_or(Connection::Close);

                    write!(self.stream, "{}", resp)?;
                    trace!("response written to '{}'", self.addr);
                },
                _ => ()
            };

            match conn {
                Connection::Close =>
                    break
            }
        }

        Ok(())
    }

    fn parse_request(&mut self) -> Result<Request> {
        let reader = BufReader::new(&mut self.stream);
        let mut buff = String::new();

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                buff.push_str("\r\n");
                break;
            }else{
                buff.push_str(&line);
                buff.push_str("\r\n");
            }
        }

        let request: Request = buff.parse()?;
        trace!("received request from '{}'", self.addr);

        Ok(request)
    }

    fn get(&mut self, req: &Request) -> io::Result<Response> {
        let url = if req.url.starts_with("/") {
            req.url.strip_prefix("/")
                .unwrap()
                .to_owned()
        }else{
            req.url.to_owned()
        };

        let path = ROOT.clone().join(&url);
        Ok(Response::file_response(&path))
    }
}
