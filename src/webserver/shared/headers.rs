/// This module simply contains a wrapper for a HashMap that allows
/// easier access/modification to header values. It does not allow you
/// to get a direct mutable reference to an internal value.
/// You can only modify headers through the various setter functions
/// to make sure that they are not set to arbitrary strings.
/// The usage is as follows:
/// ```rust
/// let headers = "Connection: close\r\nhost: localhost:80"
/// let headers: HeaderList = headers.parse()
///     .unwrap();
///
/// assert_eq!(headers.connection(), connection::CLOSE);
/// ```

use chrono::{DateTime, Utc};
use super::method::*;
use chrono::prelude::*;
use mime::*;
use std::collections::HashMap;

pub mod range;
pub use range::*;

/// Used to define a constant in the form of a header.
macro_rules! define_const {
    { $($vn:ident = $st:literal),+ } => {
        $(
            #[allow(dead_code)]
            pub const $vn: &'static str = $st;
        )+
    }
}

define_const!{
    CONNECTION          = "connection",
    HOST                = "host",
    SERVER              = "server",
    DATE                = "date",
    CONTENT_TYPE        = "content-type",
    CONTENT_LENGTH      = "content-length",
    CONTENT_LANGUAGE    = "content-language",
    CONTENT_LOCATION    = "content-location",
    CONTENT_ENCODING    = "content-encoding",
    CONTENT_RANGE       = "content-range",
    LAST_MODIFIED       = "last-modified",
    LOCATION            = "location",
    ETAG                = "etag",
    IF_MODIFIED_SINCE   = "if-modified-since",
    IF_UNMODIFIED_SINCE = "if-unmodified-since",
    IF_MATCH            = "if-match",
    IF_NONE_MATCH       = "if-none-match",
    IF_RANGE            = "if-range",
    VARY                = "vary",
    ACCEPT              = "accept",
    ACCEPT_CHARSET      = "accept-charset",
    ACCEPT_ENCODING     = "accept-encoding",
    ACCEPT_LANGUAGE     = "accept-language",
    ACCEPT_RANGE        = "accept-range",
    NEGOTIATE           = "negotiate",
    RANGE               = "range",
    USER_AGENT          = "user-agent",
    REFERER             = "referer",
    TRANSFER_ENCODING   = "transfer-encoding",
    ALTERNATES          = "alternates",
    TCN                 = "TCN"
}

/// The list of constants corresponding to the acceptable values of
/// a connection header.
pub mod connection {
    define_const! {
        LONG_LIVED = "long-lived",
        CLOSE      = "close",
        PIPELINED  = "pipelined",
        KEEP_ALIVE = "keep-alive"
    }
}

/// A wrapper around a hashmap that provides
/// convienience functions for dealing with headers.
#[derive(Debug, PartialEq, Default)]
pub struct HeaderList(HashMap<String, String>);

use std::str::FromStr;
use std::error::Error;

/// An error received when a supplied header is not known or is
/// in a provably incorrect format.
#[derive(Debug, PartialEq)]
pub enum HeaderError {
    InvalidFormatError(String),
    UnrecognizedParameterError{head: String, param: String}
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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

        let mut ret: HashMap<String, String> = HashMap::new();

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

                let res: Option<(&str, String)> = match verb.to_lowercase().as_str() {
                    CONNECTION => {
                        match desc.to_lowercase().as_str() {
                            connection::LONG_LIVED |
                            connection::PIPELINED  |
                            connection::CLOSE      |
                            connection::KEEP_ALIVE =>
                                Some((CONNECTION, desc.to_lowercase())),
                            _ =>
                                return Err(UnrecognizedParameterError{
                                    head:  CONNECTION.into(),
                                    param: desc.into()
                                })
                        }
                    },
                    DATE => {
                        let desc = desc.parse::<DateTime<Utc>>();

                        if let Ok(desc) = desc {
                            Some((DATE, desc.to_string()))
                        }else{
                            None
                        }
                    },
                    CONTENT_TYPE => {
                        desc.parse::<Mime>()
                            .map_err(|_| {
                                InvalidFormatError(
                                    format!(
                                        "unknown mime type: '{}'",
                                        desc
                                    )
                                )
                            })?;

                        Some((CONTENT_TYPE.into(), desc.into()))
                    },
                    CONTENT_LENGTH => {
                        desc.parse::<usize>()
                            .map_err(|_| {
                                InvalidFormatError(
                                    format!(
                                        "invalid content length: '{}'",
                                        desc
                                    )
                                )
                            })?;

                        Some((CONTENT_LENGTH.into(), desc.into()))
                    },
                    LAST_MODIFIED => {
                        let desc = Utc.datetime_from_str(
                                desc.into(),
                                "%a, %d %b %Y %T GMT"
                            );

                        if let Ok(date) = desc {
                            Some((
                                LAST_MODIFIED.into(),
                                Self::format_date(&date)
                            ))
                        }else{
                            None
                        }
                    },
                    IF_MODIFIED_SINCE => {
                        let desc = Utc.datetime_from_str(
                                desc.into(),
                                "%a, %d %b %Y %T GMT"
                            );

                        if let Ok(date) = desc {
                            Some((
                                IF_MODIFIED_SINCE.into(),
                                Self::format_date(&date)
                            ))
                        }else{
                            None
                        }
                    },
                    IF_UNMODIFIED_SINCE => {
                        let desc = Utc.datetime_from_str(
                                desc.into(),
                                "%a, %d %b %Y %T GMT"
                            );

                        if let Ok(date) = desc {
                            Some((
                                IF_UNMODIFIED_SINCE.into(),
                                Self::format_date(&date)
                            ))
                        }else{
                            None
                        }
                    }
                    _ => {
                        Some((verb.into(), desc.into()))
                    }
                };

                if let Some((key, val)) = res {
                    ret.insert(key.to_lowercase().into(), val.into());
                }
            }
        }

        Ok(Self(ret))
    }
}

impl HeaderList {
    /// Generates the basic headers to get ready for a response.
    /// Sets the server and date headers.
    pub fn response_headers() -> Self {
        use crate::webserver::responses::{
            SERVER_NAME,
            SERVER_VERS
        };

        let mut ret: HashMap<String, String> = Default::default();
        ret.insert(DATE.into(), Self::format_date(&Utc::now()));
        ret.insert(SERVER.into(), format!("{}-{}", SERVER_NAME, SERVER_VERS));

        Self(ret)
    }

    /// Used to retrieve a date stored under the given header name.
    pub fn get_date(&self, name: &str) -> Option<DateTime<Utc>> {
        let date = self.0.get(name)?;
        let date = Utc
            .datetime_from_str(
                date,
                "%a, %d %b %Y %T GMT"
            );

        Some(date
            .expect("date existed in hashmap, but wasnt a valid format"))
    }

    /// A helper method for when specifying the content type
    /// and length of a response.
    pub fn content(&mut self, typ: &str, len: usize) {
        debug_assert!(typ.parse::<Mime>().is_ok());

        self.0.insert(
            CONTENT_LENGTH.into(),
            len.to_string()
        );

        self.0.insert(
            CONTENT_TYPE.into(),
            typ.into()
        );
    }

    pub fn content_language(&mut self, lang: &str) {
        self.0.insert(
            CONTENT_LANGUAGE.into(),
            lang.into()
        );
    }

    /// Sets the etag header
    pub fn etag(&mut self, etag: &str) {
        self.0.insert(
            ETAG.into(),
            etag.into()
        );
    }

    /// Sets the connection header
    pub fn connection(&mut self, conn: &str) {
        self.0.insert(
            CONNECTION.into(),
            conn.into()
        );
    }

    /// Sets the accept header from a list of methods
    pub fn accept(&mut self, methods: &[Method]) {
        let mut buff = String::new();
        for method in methods.iter() {
            buff.push_str(&method.to_string());
            buff.push_str(",");
        }
        //remove the extra comma
        buff.remove(buff.len()-1);

        self.0.insert(ACCEPT.into(), buff);
    }

    pub fn chunked_encoding(&mut self) {
        self.0.insert(
            TRANSFER_ENCODING.into(),
            "chunked".into()
        );

        self.0.remove(CONTENT_LENGTH);
    }

    pub fn is_chunked(&self) -> bool {
        if let Some(enc) = self.0.get(TRANSFER_ENCODING.into()) {
            enc == "chunked"
        }else{
            false
        }
    }

    /// Sets the location header
    pub fn location(&mut self, path: String) {
        self.0.insert(
            LOCATION.into(),
            path
        );
    }

    pub fn content_range(&mut self, ranges: &RangeList, total: Option<usize>) {
        let st = if let Some((min, max)) = ranges.get_bounds() {
            format!(
                "{} {}-{}/{}",
                ranges.unit,
                min,
                max,
                if let Some(total) = total {
                    total.to_string()
                }else{
                    "*".into()
                }
            )
        }else{
            format!(
                "{} */{}",
                ranges.unit,
                if let Some(total) = total {
                    total.to_string()
                }else{
                    "*".into()
                }
            )
        };

        self.0.insert(
            CONTENT_RANGE.into(),
            st
        );
    }

    /// Sets the last modified header
    pub fn last_modified(&mut self, time: &DateTime<Utc>) {
        self.0.insert(
            LAST_MODIFIED.into(),
            Self::format_date(time)
        );
    }

    pub fn get(&self, what: &str) -> Option<&str> {
        match self.0.get(what) {
            Some(val) => Some(val),
            None      => None
        }
    }

    pub fn has(&self, what: &str) -> bool {
        self.0.get(what).is_some()
    }

    fn format_date(date: &DateTime<Utc>) -> String {
        date.format("%a, %d %b %Y %T GMT")
            .to_string()
    }
}

//I know, this function is hideous.
fn title_case(s: &str) -> String {
    let mut ret = String::new();
    ret.push_str(&s
        .chars()
        .nth(0)
        .unwrap()
        .to_ascii_uppercase()
        .to_string());

    for (ind, _) in s.match_indices("-") {
        ret.push_str(&s[ret.len()..=ind]);
        ret.push_str(&s.chars()
            .nth(ind+1)
            .unwrap()
            .to_ascii_uppercase()
            .to_string()
        );
    }
    ret.push_str(&s[ret.len()..s.len()]);

    ret
}

use std::fmt::{Display, Formatter};
impl Display for HeaderList {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        for (key, val) in self.0.iter() {
            write!(fmt, "{}: {}\r\n", title_case(key), val)?;
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
        let as_struct = as_struct.unwrap();
        assert_eq!(connection::CLOSE, as_struct.get(CONNECTION).unwrap());
    }
}
