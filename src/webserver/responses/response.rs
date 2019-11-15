mod templates;
mod content_negotiator;

use content_negotiator::*;
use templates::*;

use crate::webserver::socket_handler::etag::*;
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

        compile_templates!(&CONFIG.templates
            .display()
            .to_string())
    };

    static ref DEFAULT_LANGUAGE: String = {
        "en".into()
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
                headers.content(
                    "text/html".into(),
                    None,
                    data.len()
                );
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

    pub fn multiple_choices(headers: HeaderList) -> Self {
        Response::error(
            StatusCode::MultipleChoice,
            "Multiple resources matched the query.",
            headers
        )
    }

    pub fn not_modified(loc: &Path) -> Self {
        let mut headers = HeaderList::response_headers();

        let new_path = loc.strip_prefix(&CONFIG.root)
            .unwrap_or(loc);

        let temp = if loc.starts_with(&CONFIG.root) {
            loc.into()
        }else{
            CONFIG.root.join(
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

    pub fn unauthorized(headers: HeaderList) -> Self {
        Self {
            code: StatusCode::Unauthorized,
            data: None,
            headers
        }
    }

    pub fn precondition_failed() -> Self {
        Response::error(
            StatusCode::PreconditionFailed,
            "The supplied precondition failed.",
            HeaderList::response_headers()
        )
    }

    pub fn not_acceptable() -> Self {
        Response::error(
            StatusCode::NotAcceptable,
            "The only file found could not match the request.",
            HeaderList::response_headers()
        )
    }

    pub fn options_response(_path: &Path) -> Self {
        let mut headers = HeaderList::response_headers();
        headers.allow(
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

        headers.content("message/http", None, req_data.len());

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

                        let desc = map_file(path);
                        headers.content(
                            &desc.typ.to_string(),
                            desc.charset,
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

                let desc = map_file(path);
                headers.content_language(
                    &desc.lang
                );

                if let Some(enc) = desc.enc {
                    headers.content_encoding(&enc);
                }

                if let Some(charset) = desc.charset {
                    headers.content_charset(charset);
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
            for range in ranges.ranges.iter() {
                if range.end.is_none() {
                    if range.start < 0 {
                        file.seek(SeekFrom::End(range.start))?;
                        file.read_to_end(&mut ret_buff)?;
                    }else{
                        file.seek(SeekFrom::Start(range.start as u64))?;
                        file.read_to_end(&mut ret_buff)?;
                    }
                }else{
                    //Plus 1 because it's right inclusive
                    let mut temp_buff = vec![
                        0;
                        ((range.end.unwrap() - range.start) as usize)+1
                    ];

                    file.seek(SeekFrom::Start(range.start as u64))?;
                    file.read(&mut temp_buff)?;

                    ret_buff.append(&mut temp_buff);
                }

            }

            let desc = map_file(path);

            let mut headers = HeaderList::response_headers();
            headers.content(
                &desc.typ.to_string(),
                desc.charset,
                ret_buff.len()
            );

            let len = path.metadata()?.len();
            headers.content_range(&ranges, Some(len as usize));

            headers.content_language(
                &desc.lang
            );

            Ok(Self{
                data:    Some(ret_buff.into()),
                headers: headers,
                code:    StatusCode::PartialContent
            })
        }
    }


    pub fn path_response(path: &Path, req: &Request) -> Self {
        for redir in REDIRECTS.iter() {
            let temp = path
                .strip_prefix(&CONFIG.root)
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
                for file in CONFIG.indexes.iter() {
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
            if path.exists() {
                Self::file_response(path)
            }else{
                use content_negotiator::NegotiationError::*;

                let list = ContentNegotiator::new(path, &req.headers)
                    .best_choice();

                match list {
                    Ok(mut list) => {
                        Self::file_response(&list.pop().unwrap())
                    },
                    Err(err) =>
                        match err {
                            NotAcceptable        => 
                                Self::not_acceptable(),
                            MultipleResponses(list) => {
                                let mut headers = 
                                    HeaderList::response_headers();
                                let alt = format_alternates(list);
                                headers.alternates(alt);

                                Self::multiple_choices(
                                    headers
                                )
                            },
                            _                    =>
                                Self::not_found()
                        }
                }
            }
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

                        headers.content("text/html", None, data.len());
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
                            headers,
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
        let new_path = path.strip_prefix(&CONFIG.root)
            .unwrap_or(path);

        let temp = if path.starts_with(&CONFIG.root) {
            path.into()
        }else{
            CONFIG.root.join(
                path
                    .strip_prefix("/")
                    .unwrap_or(path)
            )
        };

        if temp.is_dir() {
            if temp.is_absolute() {
                headers.location(format!("/{}/", new_path.display()));
            }else{
                headers.location(format!("/{}/", new_path.display()));
            }
        }else{
            if temp.is_absolute() {
                headers.location(format!("{}", new_path.display()));
            }else{
                headers.location(format!("/{}", new_path.display()));
            }
        }

        Self::error(
            code,
            &format!(
                "This page has moved to: <a href=\"{0}\">{0}</a>",
                path.display()
            ),
            headers
        )
    }

    fn write_w_timeout<'a, T>(writer: &'a mut T, dat: &[u8]) -> ioResult<()>
    where
        T: std::io::Write + Sized
    {
        use std::io::ErrorKind;
        use std::time::Instant;

        let mut start = Instant::now();

        let mut at = 0;
        while at < dat.len() {
            if (Instant::now() - start) >= CONFIG.write_timeout {
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

struct FileDescriptor {
    pub typ:  Mime,
    pub lang: String,
    pub enc:  Option<String>,
    pub charset: Option<String>
}

fn map_file(file: &Path) -> FileDescriptor {
    use mime::*;

    let mut ret_desc = FileDescriptor {
        typ: APPLICATION_OCTET_STREAM,
        lang: DEFAULT_LANGUAGE.clone(),
        enc:  None,
        charset: None
    };

    map_lang(file, &mut ret_desc);
    map_charset(file, &mut ret_desc);
    map_encoding(file, &mut ret_desc);
    map_extension(file, &mut ret_desc);

    ret_desc
}

fn map_lang(path: &Path, desc: &mut FileDescriptor) {
    if let Some(ext) = path.extension() {
        let ext: String = ext.to_string_lossy()
            .into();

        let lang = match ext.as_str() {
            "en" |
            "es" |
            "de" |
            "ja" |
            "ko" |
            "ru" =>
                Some(ext.into()),
            _    => {
                if let Some(stem) = path.file_stem() {
                    let stem = PathBuf::from(stem);

                    map_charset(&stem, desc);
                    map_encoding(&stem, desc);
                    map_extension(&stem, desc);
                }
                None
            }
        };

        if let Some(lang) = lang {
            desc.lang = lang;
        }
    }
}

fn map_encoding(path: &Path, desc: &mut FileDescriptor) {
    if let Some(ext) = path.extension() {
        let ext: String = ext.to_string_lossy()
            .into();

        let enc = match ext.as_str() {
            "gz"        => Some(headers::encoding::GZIP.into()),
            "zip" | "Z" => Some(headers::encoding::COMPRESS.into()),
            _           => {
                if let Some(stem) = path.file_stem() {
                    let stem = PathBuf::from(stem);

                    map_lang(&stem, desc);
                    map_charset(&stem, desc);
                    map_extension(&stem, desc);
                }
                None
            }
        };

        if let Some(enc) = enc {
            desc.enc = Some(enc); 
        }
    }

}

fn map_extension(path: &Path, desc: &mut FileDescriptor) {
    use mime::*;

    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy();

        let typ = match ext.to_lowercase().as_str() {
            "js"  => Some(APPLICATION_JAVASCRIPT),

            "htm"  |
            "html" => Some(TEXT_HTML),
            "css"  => Some(TEXT_CSS),
            "xml"  => Some(TEXT_XML),
            "txt"  => Some(TEXT_PLAIN),

            "jpg"  |
            "jpeg" => Some(IMAGE_JPEG),
            "png"  => Some(IMAGE_PNG),
            "gif"  => Some(IMAGE_GIF),
            "pdf"  => Some(APPLICATION_PDF),

            "ppt"  |
            "pptx" => Some("application/vnd.ms-powerpoint"
                        .parse()
                        .expect("failed to parse mime type")),
            "doc"  |
            "docx" => Some("application/vnd.ms-word"
                        .parse()
                        .expect("failed to parse mime type")),

            _ => {
                if let Some(stem) = path.file_stem() {
                    let stem = PathBuf::from(stem);

                    map_lang(&stem, desc);
                    map_encoding(&stem, desc);
                    map_charset(&stem, desc);
                }
                None
            }
        };

        if let Some(typ) = typ {
            desc.typ = typ;
        }
    }
}

fn map_charset(path: &Path, desc: &mut FileDescriptor) {
    if let Some(ext) = path.extension() {
        let ext: String = ext.to_string_lossy()
            .into();

        let charset = match ext.as_str() {
            "jis"    => Some("iso-2022-jp"),
            "koi8-r" => Some("koi8-r"),
            "euc-kr" => Some("euc-kr"),
            _           => {
                if let Some(stem) = path.file_stem() {
                    let stem = PathBuf::from(stem);

                    map_lang(&stem, desc);
                    map_encoding(&stem, desc);
                    map_extension(&stem, desc);
                }
                None
            }
        };

        if let Some(charset) = charset {
            desc.charset = Some(charset.into()); 
        }
    }

}

fn format_alternates(paths: Vec<(u32, PathBuf)>) -> String {
    let mut ret = String::new();
    for (score, path) in paths.into_iter() {
        let desc = map_file(&path);
        ret.push_str(
            &format!(
                "{{{:?} {} {{type {}}} {{language {}}}",
                path.file_name().unwrap(),
                score as f32 / 1000.,
                desc.typ.to_string(),
                desc.lang
            )
        );

        if let Some(charset) = desc.charset {
            ret.push_str(&format!(" {{charset {}}}", charset));
        }

        ret.push_str("},");
    }
    ret.pop();

    ret
}
