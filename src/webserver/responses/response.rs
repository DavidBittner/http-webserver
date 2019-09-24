use super::status_code::StatusCode;
use num_traits::ToPrimitive;

use mime::Mime;
use std::path::Path;
use crate::webserver::shared::*;
use std::fs::File;
use std::io::Read;
use log::*;

static SERVER_NAME: &'static str = "ScratchServer";
static SERVER_VERS: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug, PartialEq)]
pub struct Response {
    pub code: StatusCode,
    pub headers: HeaderList,
    pub data: Option<Vec<u8>>
}

impl Response {
    fn get_name() -> String {
        format!("{}-{}", SERVER_NAME, SERVER_VERS)
    }

    fn not_found(mut headers: HeaderList) -> Self {
        headers.content_len = Some(0);
        Self {
            code: StatusCode::NotFound,
            headers: headers,
            data: None
        }
    }

    pub fn file_response(path: &Path) -> Self {
        use std::fs;

        let mut headers = HeaderList::response_headers(Response::get_name());

        match fs::read(path) {
            Ok(buff) => {
                let code = StatusCode::Ok;

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

    match ext {
        "txt" => TEXT_PLAIN,
        "png" => IMAGE_PNG,
        "js"  => APPLICATION_JAVASCRIPT,

        "htm"  |
        "html" => TEXT_HTML,
        
        "css"  => TEXT_CSS,

        _ =>     APPLICATION_OCTET_STREAM
    }
}
