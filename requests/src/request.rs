use crate::method::*;
use crate::headers::*;
use std::str::FromStr;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Default)]
pub struct Request {
    pub method:  Method,
    pub url:     String,
    pub version: String,
    pub headers: HeaderList,
    pub content: Vec<u8>
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

        let header_block: String = lines.iter()
            .take_while(|line| !line.is_empty())
            .fold(String::new(), |cur, new| format!("{}\r\n{}", cur, new));

        Ok(Request{
            method:  verbs[0].parse()?,
            url:     String::from(verbs[1]),
            version: String::from(verbs[2]),
            headers: header_block.parse()?,
            content: Vec::new()
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
