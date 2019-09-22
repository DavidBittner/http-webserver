#[derive(Debug, PartialEq)]
pub enum Method {
    Get,
    Head,
    Options,
    Trace,
}

impl Default for Method {
    fn default() -> Self {
        Method::Get
    }
}

use std::error::Error;

#[derive(Debug)]
pub struct UnknownMethodError(pub String);

impl std::fmt::Display for UnknownMethodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnknownMethodError: '{}'", self.0)
    }
}

impl Error for UnknownMethodError {}

use std::str::FromStr;

impl FromStr for Method {
    type Err = UnknownMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET"     => Ok(Method::Get),
            "HEAD"    => Ok(Method::Head),
            "OPTIONS" => Ok(Method::Options),
            "TRACE"   => Ok(Method::Trace),
            _         => Err(UnknownMethodError(String::from(s)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_method() {
        let test_str = "GET";
        let as_enum: Result<Method, _> = test_str.parse();

        assert!(as_enum.is_ok());
    }
}
