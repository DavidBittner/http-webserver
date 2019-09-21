use crate::method::*;
use crate::headers::*;
use std::str::FromStr;
use std::fmt::{Display, Formatter};

#[derive(PartialEq, Debug)]
pub struct GetRequest {
    ver: String,
    url: String,
    headers: HeaderList
}

type Result<T> = std::result::Result<T, HeaderError>;

impl GetRequest {
    pub fn new<S: Into<String>>(ver: S, url: S, header_block: S) -> Result<Self> {
        Ok(Self {
            ver: ver.into(),
            url: url.into(),
            headers: header_block.into().parse()?
        })
    }
}
