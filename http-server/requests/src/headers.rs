/// This module simple contains the header structure as well as parsing code.
/// The usage is as follows: 
/// ```rust
/// # use requests::headers::*;
/// let example_string = "Connection: close";
/// let as_enum: Result<Header, _> = example_string.parse();
///
/// assert_eq!(
///     as_enum.unwrap(),
///     Header::Connection(ConnectionOption::Close)
/// );
/// ```

///The enum that contains each possible header the server can receive.
#[derive(Debug, PartialEq)]
pub enum Header {
    ///The connection status after this request.
    Connection(ConnectionOption)
}

///An enum that contains the available options for modifiying a connection.
#[derive(Debug, PartialEq)]
pub enum ConnectionOption {
    ///Close the connection.
    Close
}

use std::str::FromStr;
use std::error::Error;

///An error received when a supplied header is not implemented/unknown.
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

///An error that occurs when an option given for the 'Connection' header is unknown.
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
