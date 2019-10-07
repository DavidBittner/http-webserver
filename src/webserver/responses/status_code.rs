use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, FromPrimitive, ToPrimitive, PartialEq, Clone, Copy)]
pub enum StatusCode {
    Ok                  = 200,
    MovedPermanently    = 301,
    Found               = 302,
    NotModified         = 304,
    BadRequest          = 400,
    Forbidden           = 403,
    NotFound            = 404,
    RequestTimeout      = 408,
    InternalServerError = 500,
    NotImplemented      = 501,
    VersionNotSupported = 505,
    Unknown             = 0,
}

use std::fmt::{Display, Formatter};
impl Display for StatusCode {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        use StatusCode::*;
        let wr = match self {
            Ok => "Ok",
            NotFound            => "Not Found",
            Forbidden           => "Forbidden",
            InternalServerError => "Internal Server Error",
            VersionNotSupported => "HTTP Version Not Supported",
            BadRequest          => "Bad Request",
            NotImplemented      => "Not Implemented",
            MovedPermanently    => "Moved Permanently",
            Found               => "Found",
            NotModified         => "Not Modified",
            RequestTimeout      => "Request Timeout",
            Unknown             => panic!("shouldn't be here")
        };

        write!(fmt, "{}", wr)
    }
}
