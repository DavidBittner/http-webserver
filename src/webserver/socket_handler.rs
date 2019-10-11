pub mod etag;

use super::requests::*;
use super::responses::*;
use super::shared::headers::*;
use super::shared::*;

use std::time::{Duration, SystemTime};
use std::net::{TcpStream, SocketAddr};
use std::io::{BufRead, BufReader};

use std::fmt::{Display, Formatter};
use std::error::Error;
use std::path::{PathBuf, Path};
use std::sync::RwLock;

use log::*;

use crate::CONFIG;
use super::clf::*;

type Result<T> = std::result::Result<T, SocketError>;

lazy_static::lazy_static! {
    pub static ref ROOT: PathBuf = {
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

    static ref LOG_LIST: RwLock<Vec<LogEntry>> = {
        Default::default()
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
    ConnectionClosed
}

impl Display for SocketError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use SocketError::*;

        match self {
            IoError(err) => write!(f, "IoError: {}", err),
            RequestError(err) => write!(f, "{}", err),
            ConnectionClosed  => write!(f, "connection closed by user")
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

use std::io::Result as ioResult;
impl SocketHandler {
    pub fn new(stream: TcpStream) -> ioResult<Self> {
        stream.set_read_timeout(Some(*READ_TIMEOUT))?;
        stream.set_write_timeout(Some(*WRITE_TIMEOUT))?;

        Ok(SocketHandler {
            addr:   stream.peer_addr()?,
            stream: stream,
        })
    }

    pub fn dispatch(mut self) -> Result<()> {
        loop {
            let req = self.parse_request();

            //If the response failed to be parsed, send a bad request
            let mut resp = match &req {
                Ok(req) => {
                    if req.ver != "HTTP/1.1" {
                        Response::unsupported_version()
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
                                Response::not_implemented()
                            }

                        }
                    }

                },
                Err(err) => {
                    use SocketError::*;
                    match err {
                        IoError(err) => {
                            use std::io::ErrorKind::*;
                            match err.kind() {
                                WouldBlock => {
                                    error!("request timed out: '{}'", err);
                                    Response::timed_out()
                                },
                                _ => {
                                    error!("io error occurred: '{}'", err);
                                    Response::bad_request()
                                }
                            }
                        },
                        ConnectionClosed => {
                            return Ok(());
                        },
                        _ => {
                            error!("{}", err);
                            Response::bad_request()
                        }
                    }
                }
            };

            let conn;
            match req {
                Ok(mut req) => {
                    let entry = LogEntry::new(&self.addr, &req, &resp);
                    let mut list = LOG_LIST.write().unwrap();
                    list.push(entry);

                    conn = req.headers.connection
                        .get_or_insert(Connection::LongLived)
                        .clone();
                },
                Err(_) =>
                    conn = resp.headers.connection
                        .get_or_insert(Connection::Close)
                        .clone()
            };

            resp.write_self(&mut self.stream)?;
            trace!("response written to '{}'", self.addr);

            match conn {
                Connection::Close =>
                    break,
                _ => ()
            };
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

        if buff.is_empty() {
            use std::net::Shutdown;
            self.stream.shutdown(Shutdown::Both)?;
            Err(SocketError::ConnectionClosed)
        }else{
            trace!("received request from '{}'", self.addr);

            let request = buff.parse()?;
            Ok(request)
        }


    }

    fn sterilize_path(path: &PathBuf) -> PathBuf {
        let has_slash = path.as_os_str()
            .to_string_lossy()
            .ends_with("/");

        let rel_path = if path.starts_with("/") {
            path.strip_prefix("/")
                .unwrap()
        }else{
            &path
        };

        if has_slash {
            PathBuf::from(
                format!("{}/", ROOT.join(rel_path).display())
            )
        }else{
            ROOT
                .join(rel_path)
        }
    }

    fn get(&mut self, req: &Request) -> Response {
        let url = SocketHandler::sterilize_path(&req.path);

        if url.starts_with(&*ROOT) {
            let not_mod = SocketHandler::check_if_match(req, &url)
                .or(SocketHandler::check_modified_since(req, &url))
                .or(SocketHandler::check_unmodified_since(req, &url))
                .or(SocketHandler::check_if_none_match(req, &url));

            if not_mod.is_some() {
                not_mod.unwrap()
            }else{
                let comp = ROOT.join(PathBuf::from(".well-known/access.log"));
                if url.clone() == comp {
                    SocketHandler::log_response()
                }else{
                    Response::file_response(&url)
                }
            }
        }else{
            Response::forbidden()
        }
    }

    fn check_modified_since(req: &Request, full_path: &Path) -> Option<Response> {
        match req.headers.if_modified {
            Some(date) => {
                let check_time: SystemTime = date.into();
                match full_path.metadata() {
                    Ok(meta) => {
                        match meta.modified() {
                            Ok(time) => {
                                if check_time < time {
                                    None
                                }else{
                                    return Some(
                                        Response::not_modified(full_path)
                                    );
                                }
                            },
                            Err(err) => {
                                warn!("couldn't retrieve last-modified date for file: '{}'", err);
                                Some(Response::precondition_failed())

                            }
                        }
                    },
                    Err(err) => {
                        warn!(
                            "couldn't retrieve metadata for file: '{}'",
                            err
                        );
                        Some(Response::precondition_failed())
                    }
                }
            },
            None =>
                None
        }
    }

    fn check_unmodified_since(req: &Request, full_path: &Path) -> Option<Response> {
        match req.headers.if_unmodified {
            Some(date) => {
                let check_time: SystemTime = date.into();
                match full_path.metadata() {
                    Ok(meta) => {
                        match meta.modified() {
                            Ok(time) => {
                                if check_time >= time {
                                    None
                                }else{
                                    Some(Response::precondition_failed())
                                }
                            },
                            Err(err) => {
                                warn!("couldn't retrieve last-modified date for file: '{}'", err);
                                Some(Response::precondition_failed())

                            }
                        }
                    },
                    Err(err) => {
                        warn!(
                            "couldn't retrieve metadata for file: '{}'",
                            err
                        );
                        Some(Response::precondition_failed())
                    }
                }
            },
            None =>
                None
        }
    }

    fn check_if_match(req: &Request, full_path: &Path) -> Option<Response> {
        use etag::*;

        match &req.headers.if_match {
            Some(etag) => {
                let comp_etag = file_etag(full_path).ok()?;
                if comp_etag == *etag {
                    None
                }else{
                    Some(Response::precondition_failed())
                }
            },
            None =>
                None
        }
    }

    fn check_if_none_match(req: &Request, full_path: &Path) -> Option<Response> {
        use etag::*;

        match &req.headers.if_none_match {
            Some(etags) => {
                let comp_etag = file_etag(full_path).ok()?;
                for etag in etags.iter() {
                    if comp_etag == *etag {
                        return None;
                    }
                }
                Some(Response::precondition_failed())
            },
            None =>
                None
        }
    }

    fn log_response() -> Response {
        let mut buff = String::new();
        {
            let log_list = LOG_LIST.read().unwrap();
            for entry in log_list.iter() {
                buff.push_str(&format!("{}\n", entry));
            }
        }

        let buff: Vec<u8> = buff.into();

        let mut headers      = HeaderList::response_headers();
        headers.content_len  = Some(buff.len());
        headers.content_type = Some(mime::TEXT_PLAIN);

        Response {
            code: StatusCode::Ok,
            headers: headers,
            data: Some(buff),
        }
    }

    fn options(&mut self, req: &Request) -> Response {
        let url = SocketHandler::sterilize_path(&req.path);

        if url.starts_with(&*ROOT) {
            Response::options_response(&url)
        }else{
            Response::forbidden()
        }
    }

    fn trace(&mut self, req: &Request) -> Response {
        Response::trace_response(req)
    }
}
