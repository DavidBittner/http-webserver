use crate::webserver::shared::method::*;
use crate::webserver::shared::headers::*;

use std::str::FromStr;
use std::fmt::{Display, Formatter};
use url::{ParseError, Url};
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct Request {
    pub method:  Method,
    pub path:    PathBuf,
    pub ver:     String,
    pub headers: HeaderList
}

#[derive(Debug)]
pub enum RequestParsingError {
    MethodError(UnknownMethodError),
    UrlError(ParseError),
    HeaderError(HeaderError),
    FormatError
}

impl Display for RequestParsingError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use RequestParsingError::*;

        match self {
            MethodError(err) => write!(f, "error with method: '{}'", err),
            HeaderError(err) => write!(f, "error with header: '{}'", err),
            UrlError(err)    => write!(f, "error with url: '{}'", err),
            FormatError      => write!(f, "could not understand the given request")
        }
    }
}

impl std::error::Error for RequestParsingError {}

impl From<HeaderError> for RequestParsingError {
    fn from(err: HeaderError) -> Self {
        RequestParsingError::HeaderError(err)
    }
}

impl From<UnknownMethodError> for RequestParsingError {
    fn from(err: UnknownMethodError) -> Self {
        RequestParsingError::MethodError(err)
    }
}

impl From<ParseError> for RequestParsingError {
    fn from(err: ParseError) -> Self {
        RequestParsingError::UrlError(err)
    }
}

impl FromStr for Request {
    type Err = RequestParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use RequestParsingError::*;

        let mut lines: Vec<&str> = s.lines()
            .collect();

        let verbs: Vec<&str> = lines.remove(0)
            .split_whitespace()
            .collect();

        if verbs.len() != 3 {
            return Err(FormatError);
        }

        let mut header_block = String::new();
        for line in lines.iter() {
            if line.is_empty() {
                break;
            }
            header_block.push_str(format!("{}\r\n", line).as_str());
        }

        let method = verbs[0];
        let url    = verbs[1];
        let ver    = verbs[2];

        let headers: HeaderList = header_block.parse()?;

        let url = if url != "*" {
            let url = urlencoding::decode(url)
                .map_err(|_|
                    FormatError
                )?;

            match headers.get(HOST) {
                Some(host) => {
                    let base = format!("http://{}/", host);

                    Url::options()
                        .base_url(Some(&Url::parse(&base)?))
                        .parse(&url)?
                        .path()
                        .to_owned()
                },
                None =>
                    Url::parse(&url)?
                        .path()
                        .to_owned()
            }
        }else{
            url.to_owned()
        };
        
        Ok(Request{
            method: method.parse()?,
            path: url.into(),
            ver:  ver.into(),
            headers: headers
        })
    }
}

impl Display for Request {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        write!(
            fmt,
            "{} {} {}\r\n",
            self.method,
            self.path.display(),
            self.ver
        )?;

        write!(fmt, "{}", self.headers)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_test() {
        let request_str = "GET * HTTP/1.1\r\nConnection: close\r\n\r\n";

        let request: Result<Request, _> = request_str.parse();
        assert!(request.is_ok());
    }
}
