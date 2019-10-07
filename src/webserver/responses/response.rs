use super::status_code::StatusCode;
use num_traits::ToPrimitive;

use mime::Mime;
use std::path::{Path};
use crate::webserver::shared::*;
use crate::webserver::requests::Request;
use super::redirect::*;
use log::*;

pub static SERVER_NAME: &'static str = "ScratchServer";
pub static SERVER_VERS: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug, PartialEq)]
pub struct Response {
    pub code: StatusCode,
    pub headers: HeaderList,
    pub data: Option<Vec<u8>>
}

impl Response {
    pub fn not_found(mut headers: HeaderList) -> Self {
        headers.content_len = Some(0);
        Self {
            code: StatusCode::NotFound,
            headers: headers,
            data: None
        }
    }

    pub fn internal_error(mut headers: HeaderList) -> Self {
        headers.content_len = None;
        headers.content_type = None;
        Self {
            code: StatusCode::InternalServerError,
            headers: headers,
            data: None
        }
    }

    pub fn forbidden(mut headers: HeaderList) -> Self {
        headers.content_len = None;
        headers.content_type = None;
        Self {
            code: StatusCode::Forbidden,
            headers: headers,
            data: None
        }
    }

    pub fn unsupported_version(headers: HeaderList) -> Self {
        Self {
            code: StatusCode::VersionNotSupported,
            headers: headers,
            data: None
        }
    }

    pub fn bad_request(headers: HeaderList) -> Self {
        Self {
            code: StatusCode::BadRequest,
            headers: headers,
            data: None
        }
    }

    pub fn not_implemented(headers: HeaderList) -> Self {
        Self {
            code: StatusCode::NotImplemented,
            headers: headers,
            data: None
        }
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
            if redir.matches(path) {
                return Response::redirect(
                    path,
                    redir.code
                );
            }
        }

        if path.is_dir() &&
          !path.ends_with("/") {
            return Response::redirect(
                &path,
                StatusCode::MovedPermanently
            );
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
                                return Self::internal_error(headers);
                            }
                        }
                    },
                    Err(err) => {
                        error!("error occurred retrieving metadata: '{}'", err);
                        return Self::forbidden(headers);
                    }
                }

                let ext = path.extension()
                    .unwrap_or(std::ffi::OsStr::new(""))
                    .to_string_lossy();

                headers.content_len = Some(buff.len());
                headers.content_type = Some(map_extension(&ext));

                Self {
                    code: code,
                    headers: headers,
                    data: Some(buff)
                }
            },
            Err(err) => {
                error!("error reading file '{}' to string: '{}'", path.display(), err);
                Response::not_found(headers)
            }
        }
    }

    fn redirect(path: &Path, code: StatusCode) -> Self {
        use crate::webserver::socket_handler::ROOT;

        let mut headers = HeaderList::response_headers();
        let new_path = path.strip_prefix(&*ROOT)
            .unwrap_or(path);

        headers.location = Some(format!("/{}/", new_path.display()));
        Self {
            code: code,
            headers: headers,
            data: None
        }
    }

    pub fn write_self<'a, T: std::io::Write + Sized>(self, writer: &'a mut T) -> std::io::Result<()> {
        let num = self.code.to_u16()
            .unwrap_or(0);

        write!(writer, "{} {} {}\r\n", "HTTP/1.1", num, self.code)?;
        write!(writer, "{}\r\n", self.headers)?;
        match self.data {
            Some(dat) => {
                std::io::copy(&mut dat.as_slice(), &mut *writer)?;
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
        "pptx" => "application/vnd.ms-powerpoint".parse().expect("failed to parse mime type"),
        "doc"  |
        "docx" => "application/vnd.ms-word".parse().expect("failed to parse mime type"),

        _ => APPLICATION_OCTET_STREAM
    }
}
