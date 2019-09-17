use crate::method::Method;
use crate::headers::HeaderList;
use std::str::FromStr;

#[derive(Debug, PartialEq, Default)]
pub struct Request {
    pub method:  Method,
    pub url:     String,
    pub version: String,
    pub headers: HeaderList
}

impl FromStr for Request {
    type Err = Box<dyn std::error::Error>;

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
