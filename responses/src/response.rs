use crate::status_code::StatusCode;
use std::time::Instant;
use num_traits::ToPrimitive;

use mime::Mime;
use std::path::Path;
use shared::*;
use std::fs::File;
use std::io::Read;

use chrono::{DateTime, Utc};

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

    pub fn new() -> Self {
        let headers = HeaderList::response_headers(Response::get_name());

        Self {
            code: StatusCode::Ok,
            headers: headers,
            data: None,
        }
    }

    fn not_found(headers: HeaderList) -> Self {
        Self {
            code: StatusCode::NotFound,
            headers: headers,
            data: None
        }
    }

    pub fn file_response(path: &Path) -> Self {
        let mut headers = HeaderList::response_headers(Response::get_name());

        let file = File::open(path);

        let mut file = match file {
            Err(_) =>
                return Response::not_found(headers),
            Ok(file) => file,
        };

        let mut buff = String::new();

        match file.read_to_string(&mut buff) {
            Ok(_) => {
                let code = StatusCode::Ok;
                let data: Vec<_> = buff
                    .as_bytes()
                    .into();

                headers.content_len = Some(buff.len());
                headers.content_type = Some("text/plain".parse()
                    .unwrap());

                Self {
                    code: code,
                    headers: headers,
                    data: Some(data)
                }
            },
            Err(_) => 
                Response::not_found(headers),
            
        }
    }
}

use std::fmt::{Display, Formatter};
impl Display for Response {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        let num = self.code.to_u16()
            .unwrap_or(0);

        write!(fmt, "{} {} {}\r\n", "HTTP/1.1", num, self.code)?;
        write!(fmt, "{}\r\n", self.headers)?;

        match &self.headers.content_type {
            Some(typ) => {
                if *typ == mime::TEXT_PLAIN {
                    let data = self.data.clone()
                        .unwrap_or(Vec::new());

                    unsafe{
                        let string = String::from_utf8_unchecked(data);
                        write!(fmt, "{}", string)?;
                    }
                }
            },
            None => ()
        };

        Ok(())
    }
}
