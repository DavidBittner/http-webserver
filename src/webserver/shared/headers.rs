pub mod connection;
pub use connection::*;

use crate::webserver::shared::method::*;

use chrono::{DateTime, Utc};
use std::path::PathBuf;
use mime::*;

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
    pub connection:    Option<Connection>,
    pub host:          Option<String>,
    pub server:        Option<String>,
    pub date:          Option<DateTime<Utc>>,
    pub content_type:  Option<Mime>,
    pub content_len:   Option<usize>,
    pub last_modified: Option<DateTime<Utc>>,
    pub allow:         Option<Vec<Method>>,
    pub user_agent:    Option<String>,
    pub accept:        Option<String>,
    pub location:      Option<String>
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
            let mut req: Vec<_> = line
                .splitn(2, ": ")
                .collect();

            if line.trim().is_empty() {
                break;
            }else if req.len() < 2 {
                return Err(InvalidFormatError(line.into()));
            }else{
                let verb = req.remove(0);
                let desc = req.remove(0);
                if req.len() != 0 {
                    return Err(
                        InvalidFormatError(
                            format!(
                                "remaining data in container: '{:?}'",
                                req
                            )
                        )
                    );
                }

                match verb.to_lowercase().as_str() {
                    "connection" => {
                        match desc.parse::<Connection>() {
                            Ok(opt) => ret.connection = Some(opt),
                            Err(_)  => return Err(UnrecognizedParameterError{
                                head: verb.into(),
                                param: desc.into()
                            })
                        }
                    },
                    "host" =>
                        ret.host = Some(desc.into()),
                    "server" =>
                        ret.server = Some(desc.into()),
                    "date" => {
                        let date: DateTime<Utc> = desc.parse()
                            .map_err(|_| {
                                InvalidFormatError(
                                    format!(
                                        "'{}' is not a valid date format.",
                                        desc
                                    )
                                )
                            })?;

                        ret.date = Some(date);
                    },
                    "content-type" => {
                        let typ: Mime = desc.parse()
                            .map_err(|_| {
                                InvalidFormatError(
                                    format!(
                                        "unknown mime type: '{}'",
                                        desc
                                    )
                                )
                            })?;

                        ret.content_type = Some(typ);
                    },
                    "content-length" => {
                        let len: usize = desc.parse()
                            .map_err(|_| {
                                InvalidFormatError(
                                    format!(
                                        "invalid content length: '{}'",
                                        desc
                                    )
                                )
                            })?;
                        
                        ret.content_len = Some(len);
                    },
                    "last-modified" => {
                        let time = desc.parse()
                            .map_err(|_| {
                                InvalidFormatError(
                                    format!(
                                        "invalid date format: '{}'",
                                        desc
                                    )
                                )
                            })?;

                        ret.last_modified = Some(time);
                    },
                    "user-agent" =>
                        ret.user_agent = Some(desc.into()),
                    "accept" =>
                        ret.accept = Some(desc.into()),
                    "location:" =>
                        ret.location = Some(desc.into()),
                    _ =>
                        return Err(UnknownHeaderError(verb.into()))
                }
            }
        }

        Ok(ret)
    }
}

impl HeaderList {
    pub fn response_headers() -> Self {
        use crate::webserver::responses::{
            SERVER_NAME,
            SERVER_VERS
        };

        HeaderList {
            date: Utc::now().into(),
            server: format!("{}-{}", SERVER_NAME, SERVER_VERS).into(),
            .. Default::default()
        }
    }

    fn format_date(date: &DateTime<Utc>) -> String {
        date.format("%a, %d %b %Y %T GMT")
            .to_string()
    }
}

use std::fmt::{Display, Formatter};
impl Display for HeaderList {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        match self.date {
            Some(date) =>
                write!(fmt, "Date: {}\r\n", HeaderList::format_date(&date))?,
            None =>
                ()
        };

        match &self.server {
            Some(name) =>
                write!(fmt, "Server: {}\r\n", name)?,
            None => 
                ()
        };

        match &self.content_len {
            Some(len) =>
                write!(fmt, "Content-Length: {}\r\n", len)?,
            None =>
                ()
        };

        match &self.content_type {
            Some(typ) =>
                write!(fmt, "Content-Type: {}\r\n", typ)?,
            None =>
                ()
        };

        match &self.connection {
            Some(typ) =>
                write!(fmt, "Connection: {}\r\n", typ)?,
            None =>
                ()
        };

        match &self.last_modified {
            Some(modi) =>
                write!(fmt, "Last-Modified: {}\r\n", HeaderList::format_date(modi))?,
            None =>
                ()
        };

        match &self.allow {
            Some(allows) => {
                let mut iter = allows.iter();
                let first = iter.next();

                match first {
                    Some(first) => {
                        write!(fmt, "Allow: ")?;
                        write!(fmt, "{}", first)?;

                        for opt in iter {
                            write!(fmt, ", {}", opt)?;
                        }
                        write!(fmt, "\r\n")?;
                    },
                    None => ()
                };

            }
            None =>
                ()
        };

        match &self.host {
            Some(host) =>
                write!(fmt, "Host: {}\r\n", host)?,
            None =>
                ()
        };

        match &self.accept {
            Some(acc) =>
                write!(fmt, "Accept: {}\r\n", acc)?,
            None =>
                ()
        }

        match &self.user_agent {
            Some(agent) =>
                write!(fmt, "User-Agent: {}\r\n", agent)?,
            None =>
                ()
        }

        match &self.location {
            Some(loc) =>
                write!(fmt, "Location: {}\r\n", loc)?,
            None =>
                ()
        }

        Ok(())
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

        assert_eq!(er, HeaderError::UnknownHeaderError("Invalid".into()));
    }
}
