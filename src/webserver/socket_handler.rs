use super::requests::*;
use super::responses::*;
use super::shared::headers::*;
use super::shared::*;

use std::time::Duration;
use std::net::{TcpStream, SocketAddr};
use std::io::{BufRead, BufReader};
use std::io;

use std::fmt::{Display, Formatter};
use std::error::Error;

use log::*;

use crate::CONFIG;
use std::path::{PathBuf};

type Result<T> = std::result::Result<T, SocketError>;

lazy_static::lazy_static! {
    static ref ROOT: PathBuf = {
        lazy_static::initialize(&CONFIG);

        let root = CONFIG.get_str("root")
            .expect("root not defined (shouldn't happen)");

        PathBuf::from(root)
    };

    static ref READ_TIMEOUT: Duration = {
        lazy_static::initialize(&CONFIG);

        let ms: u64 = CONFIG.get("read_timeout")
            .expect("read_timeout not defined, shouldn't happen.");

        Duration::from_millis(ms)
    };

    static ref WRITE_TIMEOUT: Duration = {
        lazy_static::initialize(&CONFIG);

        let ms: u64 = CONFIG.get("write_timeout")
            .expect("write_timeout not defined, shouldn't happen.");

        Duration::from_millis(ms)
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
        stream.set_read_timeout(Some(*READ_TIMEOUT))?;
        stream.set_write_timeout(Some(*WRITE_TIMEOUT))?;

        Ok(SocketHandler {
            addr:   stream.peer_addr()?,
            stream: stream,
        })
    }

    pub fn dispatch(mut self) -> Result<()> {
        let conn;

        loop {
            let req = self.parse_request();

            let resp_headers = HeaderList::response_headers();
            //If the response failed to be parsed, send a bad request
            let mut resp = match req {
                Ok(req) => {
                    if req.ver != "HTTP/1.1" {
                        Response::unsupported_version(resp_headers)
                    }else{
                        match req.method {
                            Method::Get => {
                                self.get(&req)
                            },
                            Method::Head => {
                                let mut resp = self.get(&req);
                                resp.data = None;
                                resp
                            },
                            Method::Options => {
                                self.options(&req)
                            },
                            Method::Trace => {
                                self.trace(&req)
                            },
                            _ =>{
                                Response::not_implemented(resp_headers)
                            }

                        }
                    }
                },
                Err(err) => {
                    error!("{}", err);
                    Response::bad_request(resp_headers)
                }
            };

            conn = resp.headers.connection
                .get_or_insert(Connection::Close)
                .clone();

            resp.write_self(&mut self.stream)?;
            trace!("response written to '{}'", self.addr);

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

        let request = buff.parse()?;

        trace!("received request from '{}'", self.addr);

        Ok(request)
    }

    fn sterilize_path(path: &PathBuf) -> PathBuf {
        let rel_path = if path.starts_with("/") {
            path.strip_prefix("/")
                .unwrap()
        }else{
            &path
        };

        ROOT
            .join(rel_path)
    }

    fn get(&mut self, req: &Request) -> Response {
        let url = SocketHandler::sterilize_path(&req.path);

        if url.starts_with(&*ROOT) {
            Response::file_response(&url)
        }else{
            Response::forbidden(HeaderList::response_headers())
        }
    }

    fn options(&mut self, req: &Request) -> Response {
        let url = SocketHandler::sterilize_path(&req.path);

        if url.starts_with(&*ROOT) {
            Response::options_response(&url)
        }else{
            Response::forbidden(HeaderList::response_headers())
        }
    }

    fn trace(&mut self, req: &Request) -> Response {
        Response::trace_response(req)
    }
}
