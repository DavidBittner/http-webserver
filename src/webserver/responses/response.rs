mod templates;
use templates::*;

use crate::webserver::socket_handler::etag::*;
use crate::webserver::socket_handler::ROOT;
use crate::webserver::requests::Request;
use super::status_code::StatusCode;
use crate::webserver::shared::*;
use super::redirect::*;
use crate::CONFIG;

use std::path::{Path, PathBuf};
use std::io::{Write, Cursor};
use std::io::Result as ioResult;

use num_traits::ToPrimitive;
use mime::Mime;
use log::*;
use tera::Tera;


lazy_static::lazy_static!{
    static ref TERA: Tera = {
        use crate::CONFIG;
        use tera::compile_templates;

        lazy_static::initialize(&CONFIG);

        compile_templates!(&CONFIG.get_str("templates").unwrap())
    };

    static ref INDEXES: Vec<PathBuf> = {
        CONFIG.get_array("indexes")
            .unwrap()
            .into_iter()
            .map(|val| val
                .try_into()
                .expect("failed to get indexes")
            )
            .collect()
    };
}

pub static SERVER_NAME: &'static str = "Ruserv";
pub static SERVER_VERS: &'static str = env!("CARGO_PKG_VERSION");

pub static CHUNK_SIZE: usize = 2048;

pub enum ResponseData {
    Buffer(Vec<u8>),
    Stream(Box<dyn std::io::Read>)
}

impl Into<ResponseData> for Vec<u8> {
    fn into(self) -> ResponseData {
        ResponseData::Buffer(self)
    }
}

impl From<Box<dyn std::io::Read>> for ResponseData {
    fn from(oth: Box<dyn std::io::Read>) -> Self {
        ResponseData::Stream(oth)
    }
}

use std::fmt::{
    Debug,
    Formatter,
    Result as fmtResult
};
impl Debug for ResponseData {
    fn fmt(&self, fmt: &mut Formatter) -> fmtResult {
        use ResponseData::*;

        match self {
            Buffer(buff) =>
                write!(fmt, "Buffer([{}])", buff.len()),
            Stream(_) =>
                write!(fmt, "Stream(?)")
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub code: StatusCode,
    pub headers: HeaderList,
    pub data: Option<ResponseData>
}

impl Response {
    fn error(code: StatusCode, desc: &str, mut headers: HeaderList) -> Self {
        let holder = ErrorTemplate::new(code, desc);
        let data   = TERA.render("error.html", &holder);

        match data {
            Ok(string) => {
                let data: Vec<_> = string.into();
                headers.content("text/html".into(), data.len());
                headers.chunked_encoding();

                Self {
                    code: code,
                    headers: headers,
                    data: Some(data.into())
                }
            },
            Err(_) => {
                Response::internal_error()
            }
        }
    }

    pub fn not_found() -> Self {
        Response::error(
            StatusCode::NotFound,
            "The file requested could not be found.",
            HeaderList::response_headers()
        )
    }

    pub fn internal_error() -> Self {
        Response::error(
            StatusCode::InternalServerError,
            "An error occurred on our end. Sorry!",
            HeaderList::response_headers()
        )
    }

    pub fn forbidden() -> Self {
        Response::error(
            StatusCode::Forbidden,
            "You do not have permission to request that resource.",
            HeaderList::response_headers()
        )
    }

    pub fn unsupported_version() -> Self {
        Response::error(
            StatusCode::VersionNotSupported,
            "The requested HTTP version is not supported.",
            HeaderList::response_headers()
        )
    }

    pub fn bad_request() -> Self {
        Response::error(
            StatusCode::BadRequest,
            "Your request could not be understood.",
            HeaderList::response_headers()
        )
    }

    pub fn not_implemented() -> Self {
        Response::error(
            StatusCode::NotImplemented,
            "The requested function or method is not implemented.",
            HeaderList::response_headers()
        )
    }

    pub fn timed_out() -> Self {
        let mut headers = HeaderList::response_headers();
        headers.connection(
            connection::CLOSE.into()
        );

        Response::error(
            StatusCode::RequestTimeout,
            "The request timed out.",
            headers
        )
    }

    pub fn not_modified(loc: &Path) -> Self {
        let mut headers = HeaderList::response_headers();

        let new_path = loc.strip_prefix(&*ROOT)
            .unwrap_or(loc);

        let temp = if loc.starts_with(&*ROOT) {
            loc.into()
        }else{
            ROOT.join(
                loc
                    .strip_prefix("/")
                    .unwrap_or(loc)
            )
        };

        if temp.is_dir() {
            headers.location(format!("{}/", new_path.display()));
        }else{
            headers.location(format!("{}", new_path.display()));
        }

        Self {
            code: StatusCode::NotModified,
            headers: headers,
            data: None,
        }
    }

    pub fn precondition_failed() -> Self {
        Response::error(
            StatusCode::PreconditionFailed,
            "The supplied precondition failed.",
            HeaderList::response_headers()
        )
    }

    pub fn options_response(_path: &Path) -> Self {
        let mut headers = HeaderList::response_headers();
        headers.accept(
            &[
                Method::Post,
                Method::Get,
                Method::Trace,
                Method::Head
            ]
        );

        Self {
            code: StatusCode::Ok,
            headers: headers,
            data: None
        }
    }

    pub fn trace_response(req: &Request) -> Self {
        let mut headers = HeaderList::response_headers();

        let req_data = format!("{}", req);
        let req_data: Vec<u8> = req_data.into();

        headers.content("message/http", req_data.len());

        Self {
            code: StatusCode::Ok,
            headers: headers,
            data: Some(req_data.into())
        }
    }

    fn file_response(path: &Path) -> Self {
        use std::fs;
        let mut headers = HeaderList::response_headers();

        match std::fs::File::open(path) {
            Ok(file) => {
                let code = StatusCode::Ok;

                match fs::metadata(path) {
                    Ok(meta) => {
                        let time = meta.modified();
                        match time {
                            Ok(time) => {
                                use chrono::{DateTime, Utc};
                                let time: DateTime<Utc> = time.into();
                                headers.last_modified(&time);
                            },
                            Err(err) => {
                                error!("error occured while retrieving modified time: '{}'", err);
                                return Self::internal_error();
                            }
                        }

                        let ext = path.extension()
                            .unwrap_or(std::ffi::OsStr::new(""))
                            .to_string_lossy();

                        headers.content(
                            &map_extension(&ext)
                                .to_string(),
                            meta.len() as usize
                        );
                    },
                    Err(err) => {
                        error!("error occurred retrieving metadata: '{}'", err);
                        return Self::forbidden();
                    }
                }

                let etag = file_etag(path);
                match etag {
                    Ok(etag) =>
                        headers.etag(&etag),
                    Err(err) =>
                        warn!(
                            "failed to generate etag for file '{}' err was: '{}'",
                            path.display(),
                            err
                        )
                }

                Self {
                    code: code,
                    headers: headers,
                    data: Some(ResponseData::Stream(Box::new(file)))
                }
            },
            Err(err) => {
                error!(
                    "error reading file '{}' to string: '{}'",
                    path.display(),
                    err
                );
                Response::not_found()
            }
        }
    }

    fn partial_content(path: &Path, ranges: RangeList) -> ioResult<Self> {
        use std::fs::File;
        use std::io::{Seek, SeekFrom, Read};

        let mut ret_buff = Vec::new();
        if !path.exists() {
            Ok(Self::not_found())
        }else{
            let mut file = File::open(path)?;
            for range in ranges.ranges.into_iter() {
                if range.end.is_none() {
                    if range.start < 0 {
                        file.seek(SeekFrom::End(range.start))?;
                        file.read_to_end(&mut ret_buff)?;
                    }else{
                        file.seek(SeekFrom::Start(range.start as u64))?;
                        file.read_to_end(&mut ret_buff)?;
                    }
                }else{
                    let mut temp_buff = vec![
                        0;
                        (range.end.unwrap() - range.start) as usize
                    ];

                    file.seek(SeekFrom::Start(range.start as u64))?;
                    file.read(&mut temp_buff)?;

                    ret_buff.append(&mut temp_buff);
                }

            }

            let mut headers = HeaderList::response_headers();
            headers.content(
                &mime::APPLICATION_OCTET_STREAM.to_string(),
                ret_buff.len()
            );

            Ok(Self{
                data: Some(ret_buff.into()),
                headers: headers,
                code: StatusCode::PartialContent
            })
        }
    }

    pub fn path_response(path: &Path, req: &Request) -> Self {
        for redir in REDIRECTS.iter() {
            let temp = path
                .strip_prefix(
                    CONFIG
                        .get_str("root")
                        .unwrap()
                )
                .unwrap();

            let temp = PathBuf::from(format!("/{}", temp.display()));

            if redir.matches(&temp) {
                let new_path = redir.subst(&temp);
                return Response::redirect(
                    &new_path,
                    redir.code
                );
            }
        }

        //I know, this is an abomination. Thanks Rust for
        //doing a weird amount of behind the scenes
        //sterilizing on paths.
        //Link: https://github.com/rust-lang/rust/issues/29008
        //Horrible solution IMO as join removes trailing slash
        //without warning, ends_with also does not consider
        //trailing slashes.
        if path.is_dir() {
            let ends_with = path
                .as_os_str()
                .to_string_lossy()
                .ends_with("/");

            if ends_with {
                for file in INDEXES.iter() {
                    let temp = path.join(file);
                    if temp.exists() {
                        //Remove an excess slashes, make the
                        //path pretty, we can do this because
                        //we know the file exists.
                        let canon = temp.canonicalize();
                        match canon {
                            Ok(path) => {
                                return Response::path_response(
                                    &path,
                                    req
                                );
                            },
                            Err(err) => {
                                error!("could not canonicalize: '{}'", err);
                                return Response::internal_error();
                            }
                        }
                    }
                }

                return Response::directory_listing(path);
            }else{
                return Response::redirect(
                    &path,
                    StatusCode::MovedPermanently
                );
            }
        }else if req.headers.has(RANGE) {
            use std::io::ErrorKind::*;

            let range_str = req.headers.get(RANGE)
                .unwrap();

            let ranges: Result<RangeList, _> = range_str.parse();
            match ranges {
                Ok(ranges) =>
                    match Self::partial_content(path, ranges) {
                        Ok(resp) => resp,
                        Err(err) => {
                            error!(
                                "error occurred while getting partial content: '{}'",
                                err
                            );
                            match err.kind() {
                                NotFound         => Self::not_found(),
                                PermissionDenied => Self::forbidden(),
                                _                => Self::internal_error()
                            }
                        }
                    }
                Err(err)   => {
                    warn!("issue parsing ranges '{}', err was: '{}'",
                        range_str,
                        err
                    );
                    Self::internal_error()
                }
            }
        }else{
            Self::file_response(path)
        }
    }

    pub fn directory_listing(path: &Path) -> Self {
        match DirectoryListing::new(path) {
            Ok(dir) => {
                let data = TERA.render("directory.html", &dir);
                let mut headers = HeaderList::response_headers();

                match data {
                    Ok(string) => {
                        let data: Vec<_> = string.into();

                        headers.content("text/html", data.len());
                        headers.chunked_encoding();

                        let etag = dir_etag(path);
                        match etag {
                            Ok(etag) =>
                                headers.etag(&etag),
                            Err(err) =>
                                warn!("error generating etag for dir: '{}'", err)
                        }

                        Self {
                            code: StatusCode::Ok,
                            headers: headers,
                            data: Some(data.into())
                        }
                    },
                    Err(_) => {
                        Response::internal_error()
                    }
                }
            },
            Err(err) => {
                error!(
                    "failed to generate directory listing: '{}'",
                    err
                );
                Response::internal_error()
            }
        }
    }

    fn redirect(path: &Path, code: StatusCode) -> Self {
        let mut headers = HeaderList::response_headers();
        let new_path = path.strip_prefix(&*ROOT)
            .unwrap_or(path);

        let temp = if path.starts_with(&*ROOT) {
            path.into()
        }else{
            ROOT.join(
                path
                    .strip_prefix("/")
                    .unwrap_or(path)
            )
        };

        if temp.is_dir() {
            if temp.is_absolute() {
                headers.location(format!("/{}/", new_path.display()));
            }else{
                headers.location(format!("{}/", new_path.display()));
            }
        }else{
            if temp.is_absolute() {
                headers.location(format!("/{}", new_path.display()));
            }else{
                headers.location(format!("{}", new_path.display()));
            }
        }

        Self {
            code: code,
            headers: headers,
            data: None
        }
    }

    fn write_w_timeout<'a, T>(writer: &'a mut T, dat: &[u8]) -> ioResult<()>
    where
        T: std::io::Write + Sized
    {
        use super::super::socket_handler::WRITE_TIMEOUT;
        use std::io::ErrorKind;
        use std::time::Instant;

        let mut start = Instant::now();

        let mut at = 0;
        while at < dat.len() {
            if (Instant::now() - start) >= *WRITE_TIMEOUT {
                return Err(std::io::Error::new(
                    ErrorKind::TimedOut,
                    "writing the response timed out"
                ));
            }else {
                match writer.write(&dat[at..]) {
                    Ok(siz) => {
                        at += siz;
                        start = Instant::now();
                    },
                    Err(err) =>
                        match err.kind() {
                            ErrorKind::WouldBlock =>
                                continue,
                            _                     =>
                                return Err(err)
                        }
                }
            }
        }
        writer.flush()?;

        Ok(())
    }

    pub fn write_self<'a, T>(self, writer: &'a mut T) -> ioResult<()>
    where
        T: std::io::Write + Sized
    {
        let num = self.code.to_u16()
            .unwrap_or(0);

        let mut write_buff = Vec::new();
        write!(write_buff, "{} {} {}\r\n", "HTTP/1.1", num, self.code)?;
        write!(write_buff, "{}\r\n", self.headers)?;

        Self::write_w_timeout(writer, &mut write_buff)?;

        match self.data {
            Some(dat) => {
                use ResponseData::*;
                match dat {
                    Buffer(buff) =>
                        Self::write_w_timeout(writer, &buff)?,
                    Stream(mut stream) => {
                        use std::io::ErrorKind;

                        write_buff = vec![0; 2048];
                        loop {
                            match stream.read(&mut write_buff) {
                                Ok(siz) => {
                                    if siz == 0 {
                                        break;
                                    }else{
                                        Self::write_w_timeout(writer, &write_buff[0..siz])?;
                                    }
                                },
                                Err(err) =>
                                    match err.kind() {
                                        ErrorKind::WouldBlock =>
                                            continue,
                                        _                     =>
                                            return Err(err)
                                    }
                            }
                        }
                    }
                };
            },
            None => ()
        }
        Ok(())
    }

    pub fn write_chunked<'a, T>(self, writer: &'a mut T) -> ioResult<()>
    where
        T: std::io::Write + Sized
    {
        let num = self.code.to_u16()
            .unwrap_or(0);

        let mut write_buff = Vec::new();
        write!(&mut write_buff, "{} {} {}\r\n", "HTTP/1.1", num, self.code)?;
        write!(&mut write_buff, "{}\r\n", self.headers)?;

        Self::write_w_timeout(writer, &write_buff)?;

        match self.data {
            Some(data) => {
                let mut reader: Box<dyn std::io::Read> = match data {
                    ResponseData::Buffer(buff) =>
                        Box::new(Cursor::new(buff)),
                    ResponseData::Stream(stream) =>
                        Box::new(stream)
                };

                write_buff.clear();
                reader.read_to_end(&mut write_buff)?;

                for chunk in write_buff.chunks(CHUNK_SIZE) {
                    Self::write_w_timeout(
                        writer,
                        &format!("{:x}\r\n", chunk.len())
                            .into_bytes()
                    )?;
                    Self::write_w_timeout(
                        writer,
                        &chunk
                    )?;
                    Self::write_w_timeout(
                        writer,
                        &format!("\r\n")
                            .into_bytes()
                    )?;
                }

                Self::write_w_timeout(
                    writer,
                    &format!("0\r\n\r\n")
                        .into_bytes()
                )?;
                Ok(())
            },
            None =>
                Ok(())
        }
    }
}

fn map_extension<'a>(ext: &'a str) -> Mime {
    use mime::*;

    match ext.to_lowercase().as_str() {
        "js"  => APPLICATION_JAVASCRIPT,

        "htm"  |
        "html" => TEXT_HTML,
        "css"  => TEXT_CSS,
        "xml"  => TEXT_XML,
        "txt"  => TEXT_PLAIN,

        "jpg"  |
        "jpeg" => IMAGE_JPEG,
        "png"  => IMAGE_PNG,
        "gif"  => IMAGE_GIF,
        "pdf"  => APPLICATION_PDF,

        "ppt"  |
        "pptx" => "application/vnd.ms-powerpoint"
                    .parse()
                    .expect("failed to parse mime type"),
        "doc"  |
        "docx" => "application/vnd.ms-word"
                    .parse()
                    .expect("failed to parse mime type"),

        _ => APPLICATION_OCTET_STREAM
    }
}
