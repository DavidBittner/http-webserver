pub mod connection;
pub use connection::*;

/// This module simple contains the header structure as well as parsing code.
/// The usage is as follows: 
/// ```rust
/// # use requests::headers::*;
/// let example_string = "Connection: close";
/// let as_struct: Result<HeaderList, _> = example_string.parse();
///
/// assert_eq!(
///     as_struct.unwrap(),
///     HeaderList{
///         connection: Some(Connection::Close),
///         host: None
///     }
/// );
/// ```

///A struct that contains all the headers a request can contain.
///By default it is created setting everything to it's standard defaults
///and values are overwritten as they are parsed.
#[derive(Debug, PartialEq, Default)]
pub struct HeaderList {
    ///The connection status after this request.
    pub connection: Option<Connection>,
    pub host: Option<String>
}

use std::str::FromStr;
use std::error::Error;

///An error received when a supplied header is not implemented/unknown.
#[derive(Debug, PartialEq)]
pub enum HeaderError {
    UnknownHeaderError(String),
    InvalidFormatError(String),
    UnrecognizedParameterError{head: String, param: String}
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderError::UnknownHeaderError(head) =>
                write!(f, "UnknownHeaderError: '{}'", head),
            HeaderError::InvalidFormatError(head) =>
                write!(f, "InvalidFormatError: '{}'", head),
            HeaderError::UnrecognizedParameterError{head, param} =>
                write!(f, "UnrecognizedParameterError: '{} >{}<'", head, param)
        }
    }
}

impl Error for HeaderError {}

impl FromStr for HeaderList {
    type Err = HeaderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use HeaderError::*;

        let mut ret: HeaderList = Default::default();

        for line in s.lines() {
            let req: Vec<_> = s
                .split_whitespace()
                .take(2)
                .collect();

            if req.len() != 2 {
                return Err(InvalidFormatError(line.into()));
            }else{
                let verb = req[0];
                let desc = req[1];

                match verb.to_lowercase().as_str() {
                    "connection:" => {
                        match desc.parse::<Connection>() {
                            Ok(opt) => ret.connection = Some(opt),
                            Err(_)  => return Err(UnrecognizedParameterError{
                                head: verb.into(),
                                param: desc.into()
                            })
                        }
                    },
                    "host:" =>
                        ret.host = Some(desc.into()),
                    _       =>
                        return Err(UnknownHeaderError(verb.into()))
                }
            }
        }

        Ok(ret)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let test_str = "ConnecTion: cLoSe";
        let as_struct: Result<HeaderList, _> = test_str.parse();

        assert!(as_struct.is_ok());
    }

    #[test]
    fn parse_header_fail() {
        let test_str = "Invalid: header";
        let as_struct: Result<HeaderList, _> = test_str.parse();

        assert!(as_struct.is_err());
        let er = as_struct.err().unwrap();

        assert_eq!(er, HeaderError::UnknownHeaderError("Invalid:".into()));
    }
}
