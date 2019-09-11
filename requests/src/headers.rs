#[derive(Debug, PartialEq)]
pub enum Header {
    Connection(ConnectionOption)
}

#[derive(Debug, PartialEq)]
pub enum ConnectionOption {
    Close
}

use std::str::FromStr;
use std::error::Error;

#[derive(Debug)]
pub struct UnknownHeaderError {}

impl std::fmt::Display for UnknownHeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnknownHeaderError")
    }
}

impl Error for UnknownHeaderError {}

impl FromStr for Header {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let req: Vec<_> = s
            .split_whitespace()
            .take(2)
            .collect();

        if req.len() != 2 {
            Err(format!("invalid header format: {}", s).into())
        }else{
            let verb = req[0];
            let desc = req[1];

            match verb.to_lowercase().as_str() {
                "connection:" => 
                    Ok(Header::Connection(desc.parse()?)),
                _             =>
                    Err(format!("unknown header: {}", verb).into())
            }
        }
    }
}

#[derive(Debug)]
pub struct UnknownConnectionOption;

impl std::fmt::Display for UnknownConnectionOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnknownConnectionOption")
    }
}

impl Error for UnknownConnectionOption {}

impl FromStr for ConnectionOption {
    type Err = UnknownConnectionOption;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "close" => Ok(ConnectionOption::Close),
            _       => Err(UnknownConnectionOption)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let test_str = "ConnecTion: cLoSe";
        let as_enum: Result<Header, _> = test_str.parse();

        assert!(as_enum.is_ok());
        assert_eq!(
            as_enum.unwrap(),
            Header::Connection(ConnectionOption::Close)
        );
    }

    #[test]
    fn parse_header_fail() {
        let test_str = "Invalid: header";
        let as_enum: Result<Header, _> = test_str.parse();

        assert!(as_enum.is_err());
    }
}
