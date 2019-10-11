mod templates;
use templates::*;

use num_traits::ToPrimitive;
use crate::webserver::socket_handler::etag::*;

use mime::Mime;
use std::path::{Path, PathBuf};
use log::*;
use tera::Tera;

use crate::CONFIG;
use crate::webserver::socket_handler::ROOT;
use crate::webserver::shared::*;
use crate::webserver::requests::Request;
use super::status_code::StatusCode;
use super::redirect::*;

use async_std::io::Result as ioResult;

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

#[derive(Debug, PartialEq)]
pub struct Response {
    pub code: StatusCode,
    pub headers: HeaderList,
    pub data: Option<Vec<u8>>
}

impl Response {
    fn error(code: StatusCode, desc: &str, mut headers: HeaderList) -> Self {
        let holder = ErrorTemplate::new(code, desc);
        let data   = TERA.render("error.html", &holder);

        match data {
            Ok(string) => {
                let data: Vec<_> = string.into();
                headers.content_len  = Some(data.len());
                headers.content_type = Some("text/html"
                    .parse()
                    .unwrap());

                Self {
                    code: code,
                    headers: headers,
                    data: Some(data)
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
        headers.connection = Some(Connection::Close);

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
            headers.location = Some(
                PathBuf::from(format!("{}/", new_path.display()))
            );
        }else{
            headers.location = Some(new_path.into());
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
        let mut methods = Vec::new();
        methods.push(Method::Trace);
        methods.push(Method::Options);
        methods.push(Method::Get);
        methods.push(Method::Head);

        let mut headers = HeaderList::response_headers();
        headers.allow = Some(methods);

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

        headers.content_type = Some("message/http".parse().unwrap());
        headers.content_len  = Some(req_data.len());

        Self {
            code: StatusCode::Ok,
            headers: headers,
            data: Some(req_data)
        }
    }

    pub fn file_response(path: &Path) -> Self {
        use std::fs;

        let mut headers = HeaderList::response_headers();
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
                                return Response::file_response(
                                    &path
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
        }

        match fs::read(path) {
            Ok(buff) => {
                let code = StatusCode::Ok;

                match fs::metadata(path) {
                    Ok(meta) => {
                        let time = meta.modified();
                        match time {
                            Ok(time) => {
                                let time = time.into();
                                headers.last_modified = Some(time);
                            },
                            Err(err) => {
                                error!("error occured while retrieving modified time: '{}'", err);
                                return Self::internal_error();
                            }
                        }
                    },
                    Err(err) => {
                        error!("error occurred retrieving metadata: '{}'", err);
                        return Self::forbidden();
                    }
                }

                let ext = path.extension()
                    .unwrap_or(std::ffi::OsStr::new(""))
                    .to_string_lossy();

                headers.content_len  = Some(buff.len());
                headers.content_type = Some(map_extension(&ext));

                let etag = file_etag(path);
                match etag {
                    Ok(etag) =>
                        headers.etag = Some(etag),
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
                    data: Some(buff)
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

    pub fn directory_listing(path: &Path) -> Self {
        match DirectoryListing::new(path) {
            Ok(dir) => {
                let data = TERA.render("directory.html", &dir);
                let mut headers = HeaderList::response_headers();

                match data {
                    Ok(string) => {
                        let data: Vec<_> = string.into();
                        headers.content_len  = Some(data.len());
                        headers.content_type = Some("text/html"
                            .parse()
                            .unwrap());

                        let etag = dir_etag(path);
                        match etag {
                            Ok(etag) =>
                                headers.etag = Some(etag),
                            Err(err) =>
                                warn!("error generating etag for dir: '{}'", err)
                        }

                        Self {
                            code: StatusCode::Ok,
                            headers: headers,
                            data: Some(data)
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
            headers.location = Some(
                PathBuf::from(format!("{}/", new_path.display()))
            );
        }else{
            headers.location = Some(new_path.into());
        }

        Self {
            code: code,
            headers: headers,
            data: None
        }
    }

    pub async fn write_self<'a, T>(self, writer: &'a mut T) -> ioResult<()>
    where
        T: async_std::io::Write +
           Sized                + 
           std::marker::Unpin
    {
        use std::fmt::Write;

        let num = self.code.to_u16()
            .unwrap_or(0);

        let mut buff = String::new();
        write!(&mut buff, "{} {} {}\r\n", "HTTP/1.1", num, self.code)?;
        write!(&mut buff, "{}\r\n", self.headers)?;

        writer.write(buff.as_bytes());

        match self.data {
            Some(dat) => {
                async_std::io::copy(&mut dat.as_slice(), &mut *writer)
                    .await?;
                ()
            },
            None => ()
        }
        Ok(())
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
