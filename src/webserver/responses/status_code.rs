#[derive(Debug, PartialEq, Clone)]
pub enum StatusCode {
    Ok,
    PartialContent,
    MultipleChoice,
    MovedPermanently,
    Found,
    NotModified,
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    NotAcceptable,
    RequestTimeout,
    PreconditionFailed,
    RangeNotSatisfiable,
    InternalServerError,
    NotImplemented,
    VersionNotSupported,
    Custom(String, usize)
}

impl StatusCode {
    pub fn to_num(&self) -> usize {
        use StatusCode::*;
        match self {
            Ok                  => 200,
            PartialContent      => 206,
            MultipleChoice      => 300,
            MovedPermanently    => 301,
            Found               => 302,
            NotModified         => 304,
            BadRequest          => 400,
            Unauthorized        => 401,
            Forbidden           => 403,
            NotFound            => 404,
            NotAcceptable       => 406,
            RequestTimeout      => 408,
            PreconditionFailed  => 412,
            RangeNotSatisfiable => 416,
            InternalServerError => 500,
            NotImplemented      => 501,
            VersionNotSupported => 505,
            Custom(_, n)        => *n
        }
    }

    pub fn from_num(num: usize) -> Self {
        use StatusCode::*;

        match num {
            200 => Ok,
            206 => PartialContent,
            300 => MultipleChoice,
            301 => MovedPermanently,
            302 => Found,
            304 => NotModified,
            400 => BadRequest,
            401 => Unauthorized,
            403 => Forbidden,
            404 => NotFound,
            406 => NotAcceptable,
            408 => RequestTimeout,
            412 => PreconditionFailed,
            416 => RangeNotSatisfiable,
            500 => InternalServerError,
            501 => NotImplemented,
            505 => VersionNotSupported,
            _   => Custom(String::new(), num)
        }
    }
}

use std::fmt::{Display, Formatter};
impl Display for StatusCode {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        use StatusCode::*;
        let wr = match self {
            Ok                  => "Ok",
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
            PreconditionFailed  => "Precondition Failed",
            PartialContent      => "Partial Content",
            MultipleChoice      => "Multiple Choice",
            NotAcceptable       => "Not Acceptable",
            RangeNotSatisfiable => "Range Not Satisfiable",
            Unauthorized        => "Authorization Required",
            Custom(msg, _)      => msg,
        };

        write!(fmt, "{}", wr)
    }
}
