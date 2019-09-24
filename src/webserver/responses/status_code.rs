use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Debug, FromPrimitive, ToPrimitive, PartialEq)]
pub enum StatusCode {
    Ok                  = 200,
    BadRequest          = 400,
    Forbidden           = 403,
    NotFound            = 404,
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
            NotImplemented | _  => "Not Implemented",
        };

        write!(fmt, "{}", wr)
    }
}
