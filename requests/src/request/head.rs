use crate::headers::*;

#[derive(Debug, PartialEq)]
pub struct HeadRequest {
    ver: String,
    url: String,
    headers: HeaderList
}

type Result<T> = std::result::Result<T, HeaderError>;

impl HeadRequest {
    pub fn new<S: Into<String>>(ver: S, url: S, header_block: S) -> Result<Self> {
        Ok(Self {
            ver: ver.into(),
            url: url.into(),
            headers: header_block.into().parse()?
        })
    }
}
