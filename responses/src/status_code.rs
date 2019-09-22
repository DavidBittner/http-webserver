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
}

use std::error::Error;

pub fn parse_code(i: &str) -> Result<StatusCode, Box<dyn Error>> {
    let num = i.parse::<u16>()?;
    num_traits::FromPrimitive::from_u16(
        num
    ).ok_or(format!("Unknown status code: {}", num).into())
}

use std::fmt::{Display, Formatter};
impl Display for StatusCode {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        use StatusCode::*;
        let wr = match self {
            Ok => "Ok",
            _  => "Unimplemented"
        };

        write!(fmt, "{}", wr)
    }
}
