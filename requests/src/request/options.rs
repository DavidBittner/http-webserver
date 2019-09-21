use crate::method::*;
use crate::headers::*;
use std::str::FromStr;
use std::fmt::{Display, Formatter};

type Result<T> = std::result::Result<T, HeaderError>;

#[derive(PartialEq, Debug)]
pub struct OptionsRequest {
    ver: String,
    url: String,
    headers: HeaderList
}

impl OptionsRequest {
    pub fn new<S: Into<String>>(ver: S, url: S, header_block: S) -> Result<Self> {
        Ok(Self {
            ver: ver.into(),
            url: url.into(),
            headers: header_block.into().parse()?
        })
    }
}
