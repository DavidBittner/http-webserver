use std::str::FromStr;
use std::error::Error;

///An enum that contains the available options for modifiying a connection.
#[derive(Debug, PartialEq, Clone)]
pub enum Connection {
    ///Close the connection.
    Close,
    ///Keep the connection running
    LongLived,
    ///Keep the connection running
    KeepAlive
}

impl Default for Connection {
    fn default() -> Connection {
        Connection::Close
    }
}

impl std::fmt::Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Connection::Close =>
                write!(f, "close"),
            Connection::LongLived =>
                write!(f, "long-lived"),
            Connection::KeepAlive =>
                write!(f, "keep-alive")
        }
    }
}

///An error that occurs when an option given for the 'Connection' header is unknown.
#[derive(Debug)]
pub struct UnknownConnectionOption(String);

impl std::fmt::Display for UnknownConnectionOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnknownConnectionOption: '{}'", self.0)
    }
}

impl Error for UnknownConnectionOption {}

impl FromStr for Connection {
    type Err = UnknownConnectionOption;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "close"      => Ok(Connection::Close),
            "long-lived" => Ok(Connection::LongLived),
            "keep-alive" => Ok(Connection::KeepAlive),
            _       => Err(UnknownConnectionOption(String::from(s)))
        }
    }
}
