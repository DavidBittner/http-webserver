use shared::method::*;
use std::str::FromStr;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use shared::headers::*;

#[derive(Debug, PartialEq)]
pub struct Request {
    pub method: Method,
    pub url:    PathBuf,
    pub ver:    String,
    pub headers: HeaderList
}

#[derive(Debug)]
pub enum RequestParsingError {
    MethodError(UnknownMethodError),
    HeaderError(HeaderError),
}

impl Display for RequestParsingError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use RequestParsingError::*;

        match self {
            MethodError(err) => write!(f, "{}", err),
            HeaderError(err) => write!(f, "{}", err)
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

impl FromStr for Request {
    type Err = RequestParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines: Vec<&str> = s.lines()
            .collect();

        let verbs: Vec<&str> = lines.remove(0)
            .split_whitespace()
            .collect();
        assert_eq!(3, verbs.len());

        let header_block: String = lines.iter()
            .take_while(|line| !line.is_empty())
            .fold(String::new(), |cur, new| format!("{}\r\n{}", cur, new));

        let method = verbs[0];
        let url    = verbs[1];
        let ver    = verbs[2];
        
        Ok(Request{
            method: method.parse()?,
            url: url.into(),
            ver: ver.into(),
            headers: header_block.parse()?
        })
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
