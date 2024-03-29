pub mod auth_handler;
pub use auth_handler::*;
pub mod etag;

use is_executable::IsExecutable;

use super::requests::*;
use super::responses::*;
use super::shared::headers::*;
use super::shared::*;

use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::time::SystemTime;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use log::*;

use super::clf::*;
use crate::CONFIG;

type Result<T> = std::result::Result<T, SocketError>;

lazy_static::lazy_static! {
    static ref LOG_LIST: RwLock<Vec<LogEntry>> = {
        Default::default()
    };
}

pub struct SocketHandler {
    stream:   TcpStream,
    addr:     SocketAddr,
    req_buff: Vec<u8>,
}

#[derive(Debug)]
pub enum SocketError {
    IoError(std::io::Error),
    RequestError(RequestParsingError),
    NoTerminator,
    ConnectionClosed,
}

impl Display for SocketError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use SocketError::*;

        match self {
            IoError(err) => write!(f, "IoError: {}", err),
            RequestError(err) => write!(f, "{}", err),
            NoTerminator      => write!(f, "no terminator found at the end of the request"),
            ConnectionClosed => write!(f, "connection closed by user"),
        }
    }
}

impl Error for SocketError {}

impl From<RequestParsingError> for SocketError {
    fn from(err: RequestParsingError) -> Self { SocketError::RequestError(err) }
}

impl From<std::io::Error> for SocketError {
    fn from(err: std::io::Error) -> Self { SocketError::IoError(err) }
}

use std::io::Result as ioResult;
impl SocketHandler {
    pub fn new(stream: TcpStream) -> ioResult<Self> {
        stream.set_nonblocking(true)?;

        Ok(SocketHandler {
            addr:     stream.peer_addr()?,
            stream,
            req_buff: Vec::new(),
        })
    }

    pub fn dispatch(mut self) -> Result<()> {
        loop {
            log::trace!("waiting for request...");
            let req = self.read_request();

            let mut passed_auth = None;
            //If the response failed to be parsed, send a bad request
            let mut resp: Response = match &req {
                Ok(req) => {
                    debug!("\n---->\n{:#?}", req);
                    if req.ver != "HTTP/1.1" {
                        Response::unsupported_version()
                    } else {
                        let url = SocketHandler::sterilize_path(&req.path);

                        let auth_handler = AuthHandler::new(&url);
                        if let Ok(auth_handler) = auth_handler {
                            let res = auth_handler.check(req);
                            match res {
                                Ok(res) => {
                                    use AuthCheckResult::*;

                                    if res != Passed {
                                        passed_auth = Some(false);
                                        warn!(
                                            "connection '{}' failed \
                                             authentication",
                                            self.addr
                                        );

                                        if res == Failed {
                                            auth_handler.create_unauthorized(req)
                                        }else{
                                            let allows = auth_handler
                                                .allows();
                                            Response::not_allowed(allows)
                                        }
                                    } else {
                                        passed_auth = Some(true);
                                        match req.method {
                                            Method::Get => self.get(&req),
                                            Method::Head => {
                                                let mut resp = self.get(&req);
                                                resp.data = None;
                                                resp
                                            },
                                            Method::Options => {
                                                self.options(&req)
                                            },
                                            Method::Trace   =>
                                                self.trace(&req),
                                            Method::Put     =>
                                                self.put(&req),
                                            Method::Delete  =>
                                                self.delete(&req),
                                            Method::Post    =>
                                                self.post(&req)
                                        }
                                    }
                                }
                                Err(err) => {
                                    warn!(
                                        "failed parsing auth header: '{:?}'",
                                        err
                                    );
                                    Response::bad_request()
                                }
                            }
                        } else {
                            warn!(
                                "failed to create auth_handler: '{:?}'",
                                auth_handler.unwrap_err()
                            );
                            Response::internal_error()
                        }
                    }
                }
                Err(err) => {
                    use SocketError::*;
                    match err {
                        IoError(err) => {
                            use std::io::ErrorKind::*;
                            match err.kind() {
                                TimedOut => {
                                    error!("request timed out: '{}'", err);
                                    Response::timed_out()
                                }
                                _ => {
                                    error!("io error occurred: '{}'", err);
                                    Response::bad_request()
                                }
                            }
                        }
                        ConnectionClosed => {
                            return Ok(());
                        }
                        _ => {
                            error!("error parsing request:\n\t{}", err);
                            Response::bad_request()
                        }
                    }
                }
            };

            let conn: String;
            match &req {
                Ok(req) => {
                    let entry = LogEntry::new(&self.addr, &req, &resp);
                    let mut list = LOG_LIST.write().unwrap();
                    list.push(entry);

                    if let Some(passed) = passed_auth {
                        if passed {
                            let url = SocketHandler::sterilize_path(&req.path);
                            AuthHandler::create_passed(
                                &url,
                                req,
                                &mut resp.headers,
                            );
                        }
                    }

                    conn = req
                        .headers
                        .get(headers::CONNECTION)
                        .unwrap_or(connection::LONG_LIVED)
                        .into();
                }
                Err(_) => {
                    conn = resp
                        .headers
                        .get(headers::CONNECTION)
                        .unwrap_or(connection::LONG_LIVED)
                        .into()
                }
            };

            resp.headers
                .connection(&conn);

            resp.headers.connection(&conn);
            self.write_response(resp)?;
            trace!("response written to '{}'", self.addr);

            match conn.to_lowercase().as_str() {
                connection::CLOSE => break,
                _ => (),
            };
        }

        Ok(())
    }

    fn contains_slice(sl: &[u8], cn: &[u8]) -> bool {
        sl.windows(cn.len())
            .any(|sub| sub == cn)
    }

    fn find_in_slice(sl: &[u8], fin: &[u8]) -> Option<usize> {
        sl.windows(fin.len())
            .enumerate()
            .find(|(_, val)| *val == fin)
            .map(|(loc, _)| loc)
    }

    fn read_request(&mut self) -> Result<Request> {
        use std::time::Instant;
        let mut start = Instant::now();

        let mut in_buff = vec![0; 2048];
        while !Self::contains_slice(&self.req_buff, &[13u8,10u8,13u8,10u8]) &&
              !Self::contains_slice(&self.req_buff, &[10u8, 10u8])
        {
            //Check for timeouts
            if Instant::now() - start >= CONFIG.read_timeout {
                use std::io::{Error, ErrorKind};
                return Err(Error::from(ErrorKind::TimedOut).into());
            }

            match self.stream.read(&mut in_buff) {
                Ok(siz) => {
                    if siz != 0 {
                        let dat = &in_buff[0..siz];
                        self.req_buff.extend_from_slice(dat);
                        start = Instant::now();
                    } else {
                        use std::net::Shutdown;
                        self.stream.shutdown(Shutdown::Both)?;
                        return Err(SocketError::ConnectionClosed);
                    }
                }
                Err(err) => {
                    use std::io::ErrorKind;
                    match err.kind() {
                        ErrorKind::WouldBlock => continue,
                        _ => return Err(err.into()),
                    }
                }
            }
        }

        //Can unwrap due to the fact it will only get here if
        //we know the buffer contains the marker.
        let pos = 
            Self::find_in_slice(&self.req_buff, &[13u8,10u8,13u8,10u8])
                .or(Self::find_in_slice(&self.req_buff, &[10u8, 10u8]))
                .ok_or(SocketError::NoTerminator)?;

        debug!("separator found at: {}", pos);
        //Add four to the pos because we want to keep the ending chars
        let mut req_str = self.req_buff.split_off(pos + 4);
        std::mem::swap(&mut req_str, &mut self.req_buff);
        let req_str = String::from_utf8(req_str)
            .expect("should be valid UTF-8");

        trace!("sent request string was: \n```\n{}```", req_str);

        trace!(
            "request of size '{}' received from '{}'",
            req_str.len(),
            self.addr
        );

        let mut req: Request = req_str.parse()?;
        if let Some(len) = req.headers.get(headers::CONTENT_LENGTH) {
            let len: i64 = len.trim()
                .parse()
                .unwrap_or(0);

            let diff = len - self.req_buff.len() as i64;
            if diff <= 0 {
                let mut payload = self.req_buff.split_off((len - 1) as usize);
                std::mem::swap(&mut payload, &mut self.req_buff);
                req.set_payload(payload);
            }else{
                let mut buff = vec![0; diff as usize];
                self.stream.read_exact(&mut buff)?;

                let mut temp: Vec<_> = self.req_buff.split_off(0);

                temp.append(&mut buff);
                req.set_payload(temp);
            }
        }

        debug!("remaining buffer length: {}", self.req_buff.len());
        Ok(req)
    }

    fn write_response(&mut self, resp: Response) -> Result<()> {
        debug!("\n<----\n{:#?}", resp);
        match resp.headers.is_chunked() {
            true => {
                resp.write_chunked(&mut self.stream)?;
            }
            false => resp.write_self(&mut self.stream)?,
        };

        Ok(())
    }

    fn sterilize_path(path: &PathBuf) -> PathBuf {
        let has_slash = path.as_os_str().to_string_lossy().ends_with("/");

        let rel_path = if path.starts_with("/") {
            path.strip_prefix("/").unwrap()
        } else {
            &path
        };

        if has_slash {
            PathBuf::from(format!("{}/", CONFIG.root.join(rel_path).display()))
        } else {
            CONFIG.root.join(rel_path)
        }
    }

    fn get(&mut self, req: &Request) -> Response {
        let url = SocketHandler::sterilize_path(&req.path);

        if url.starts_with(&CONFIG.root) {
            let not_mod = SocketHandler::check_if_match(req, &url)
                .or(SocketHandler::check_modified_since(req, &url))
                .or(SocketHandler::check_unmodified_since(req, &url))
                .or(SocketHandler::check_if_none_match(req, &url));

            if let Some(not_mod) = not_mod {
                not_mod
            }else {
                let comp =
                    CONFIG.root.join(PathBuf::from(".well-known/access.log"));
                if url.clone() == comp {
                    SocketHandler::log_response()
                }else {
                    if     url.is_executable()
                       && !url.is_dir()
                    {
                        Response::cgi_response(self.addr.clone(), &url, req)
                    }else{
                        Response::path_response(&url, req)
                    }
                }
            }
        } else {
            Response::forbidden()
        }
    }

    fn post(&mut self, req: &Request) -> Response {
        let url = SocketHandler::sterilize_path(&req.path);

        if url.starts_with(&CONFIG.root) {
            if     url.is_executable()
               && !url.is_dir()
            {
                Response::cgi_response(self.addr.clone(), &url, req)
            }else{
                Response::bad_request()
            }
        } else {
            Response::forbidden()
        }
    }

    fn file_modified(path: &Path) -> Option<SystemTime> {
        if let Ok(meta) = path.metadata() {
            if let Ok(modi) = meta.modified() {
                Some(modi)
            } else {
                warn!(
                    "modified date for file '{}' couldn't be retrieved",
                    path.display()
                );
                None
            }
        } else {
            warn!(
                "metadata for file '{}' couldn't be retrieved",
                path.display()
            );
            None
        }
    }

    fn check_modified_since(
        req: &Request,
        full_path: &Path,
    ) -> Option<Response> {
        if let Some(date) = req.headers.get_date(&headers::IF_MODIFIED_SINCE) {
            let check_time: SystemTime = date.into();
            let modi: SystemTime = Self::file_modified(full_path)?;

            use chrono::{DateTime, Utc};
            //If the file has been modified after the check_time
            //then that means we want to just retrieve it.
            //If it hasn't, not changed.
            let temp: DateTime<Utc> = modi.into();
            debug!("check: {} modified: {}", date, temp);
            if check_time < modi {
                debug!("has been modified");
                None
            } else {
                debug!("hasn't been modified");
                Some(Response::not_modified(full_path))
            }
        } else {
            None
        }
    }

    fn check_unmodified_since(
        req: &Request,
        full_path: &Path,
    ) -> Option<Response> {
        if let Some(date) = req.headers.get_date(&headers::IF_UNMODIFIED_SINCE)
        {
            let check_time: SystemTime = date.into();
            let modi: SystemTime = Self::file_modified(full_path)?;

            if check_time >= modi {
                None
            } else {
                Some(Response::not_modified(full_path))
            }
        } else {
            None
        }
    }

    fn check_if_match(req: &Request, full_path: &Path) -> Option<Response> {
        use etag::*;

        match &req.headers.get(IF_MATCH) {
            Some(etag) => {
                let comp_etag = file_etag(full_path).ok()?;
                if &comp_etag == *etag {
                    None
                } else {
                    Some(Response::precondition_failed())
                }
            }
            None => None,
        }
    }

    fn check_if_none_match(
        req: &Request,
        full_path: &Path,
    ) -> Option<Response> {
        use etag::*;

        match &req.headers.get(IF_NONE_MATCH) {
            Some(etags) => {
                let etags: Vec<_> = etags.split(",").collect();

                let comp_etag = file_etag(full_path).ok()?;
                for etag in etags.iter() {
                    if comp_etag == *etag {
                        return None;
                    }
                }
                Some(Response::precondition_failed())
            }
            None => None,
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

        let mut headers = HeaderList::response_headers();
        headers.content(&mime::TEXT_PLAIN.to_string(), None, buff.len());

        Response {
            code: StatusCode::Ok,
            headers,
            data: Some(buff.into()),
        }
    }

    fn options(&mut self, req: &Request) -> Response {
        let url = SocketHandler::sterilize_path(&req.path);

        if url.starts_with(&CONFIG.root) {
            Response::options_response(&url)
        } else {
            Response::forbidden()
        }
    }

    fn put(&mut self, req: &Request) -> Response {
        use std::fs::File;
        use std::io::Write;

        let url = SocketHandler::sterilize_path(&req.path);
        if url.starts_with(&CONFIG.root) {
            let code = if url.exists() {
                StatusCode::Ok
            }else{
                StatusCode::Created
            };

            match File::create(&url) {
                Ok(mut file) => {
                    match req.payload {
                        Some(ref load) => {
                            match file.write_all(load) {
                                Ok(_) =>
                                    Response {
                                        code,
                                        headers: HeaderList::response_headers(),
                                        data: None
                                    },
                                Err(err) => {
                                    use std::io::ErrorKind::*;
                                    warn!(
                                        "could not write file in PUT: '{}'", 
                                        err
                                    );

                                    match err.kind() {
                                        PermissionDenied =>
                                            Response::forbidden(),
                                        _ => 
                                            Response::internal_error()
                                    }
                                }
                            }

                        },
                        None       => {
                            warn!("empty payload on PUT request");
                            Response::bad_request()
                        }
                    }
                },
                Err(err) => {
                    error!(
                        "failed to create file at '{}': '{}'",
                        url.display(),
                        err
                    );
                    Response::internal_error()
                }
            }
        }else{
            Response::forbidden()
        }
    }

    fn delete(&mut self, req: &Request) -> Response {
        let url = SocketHandler::sterilize_path(&req.path);
        if url.starts_with(&CONFIG.root) {
            match std::fs::remove_file(&url) {
                Ok(_) => {
                    Response::error(
                        StatusCode::Ok,
                        &format!("File '{}' deleted successfully.", url.display()),
                        HeaderList::response_headers()
                    )
                },
                Err(err) => {
                    use std::io::ErrorKind::*;
                    match err.kind() {
                        PermissionDenied =>
                            Response::forbidden(),
                        NotFound => {
                            Response::error(
                                StatusCode::Ok,
                                &format!("File '{}' deleted successfully.", url.display()),
                                HeaderList::response_headers()
                            )
                        },
                        _ => {
                            warn!(
                                "error occurred during DELETE req at '{}': '{}'",
                                url.display(),
                                err
                            );
                            Response::internal_error()
                        }
                    }
                }
            }
        }else{
            Response::forbidden()
        }
    }

    fn trace(&mut self, req: &Request) -> Response {
        Response::trace_response(req)
    }
}
