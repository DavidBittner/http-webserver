pub mod user;
pub use user::*;

use std::str::FromStr;
use std::path::Path;
use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult
};

#[derive(Debug, PartialEq)]
pub struct AuthFile {
    pub typ:   AuthType,
    pub realm: String,
    pub users: Vec<User>
}

impl AuthFile {
    pub fn new(path: &Path) -> Result<Self, AuthFileParseError> {
        use std::io::Read;

        let mut buff = String::new();
        let mut file = std::fs::File::open(path)?;

        file.read_to_string(&mut buff)?;
        Ok(buff.parse()?)
    }

    pub fn get_password(&self, name: &str) -> Option<String> {
        for user in self.users.iter() {
            if user.name == name {
                return Some(String::from(user.pass.clone()));
            }
        }
        None
    }
}

#[derive(Debug, PartialEq)]
pub enum AuthType {
    Basic,
    Digest
}

impl Display for AuthType {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        use AuthType::*;

        match self {
            Basic  => write!(fmt, "Basic"),
            Digest => write!(fmt, "Digest")
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct AuthTypeParseError {
    what: String
}

impl FromStr for AuthType {
    type Err = AuthTypeParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "basic"  => Ok(AuthType::Basic),
            "digest" => Ok(AuthType::Digest),
            _        => Err(AuthTypeParseError{what: s.into()})
        }
    }
}

#[derive(Debug)]
pub enum AuthFileParseError {
    InvalidFormat{msg: String},
    UnknownAuthType(AuthTypeParseError),
    UserParseError(UserParseError),
    MalformedEntry{entry: String, had: String},
    IoError(std::io::Error)
}

impl From<std::io::Error> for AuthFileParseError {
    fn from(oth: std::io::Error) -> Self {
        AuthFileParseError::IoError(oth)
    }
}

impl From<AuthTypeParseError> for AuthFileParseError {
    fn from(err: AuthTypeParseError) -> Self {
        AuthFileParseError::UnknownAuthType(err)
    }
}

impl From<UserParseError> for AuthFileParseError {
    fn from(err: UserParseError) -> Self {
        AuthFileParseError::UserParseError(err)
    }
}

impl FromStr for AuthFile {
    type Err = AuthFileParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines: Vec<_> = s.lines()
            .filter(|s| !s.trim().starts_with("#"))
            .filter(|s| !s.is_empty())
            .map(|s| s.trim())
            .collect();

        if lines.len() < 2 {
            Err(
                AuthFileParseError::InvalidFormat{
                    msg: format!("file only has {} lines", lines.len())
                }
            )
        }else{
            let mut auth_type: Option<AuthType> = Option::None;
            let mut realm:     Option<&str>     = Option::None;

            for line in lines.iter().take(2) {
                let pieces: Vec<_> = line.split("=")
                    .map(|s| s.trim())
                    .collect();

                if pieces.len() != 2 {
                    return Err(AuthFileParseError::MalformedEntry{
                        entry: pieces[0].into(),
                        had:   String::from(*line)
                    });
                }else{
                    match pieces[0].to_lowercase().as_str() {
                        "authorization-type" => auth_type = Some(pieces[1].parse()?),
                        "realm"              => realm     = Some(pieces[1]),
                        _                    => ()
                    }
                }
            }

            if realm.is_none() {
                return Err(AuthFileParseError::InvalidFormat{
                    msg: "missing realm".into()
                });
            }

            lines.remove(0);
            lines.remove(0);

            let mut users = Vec::new();
            for line in lines.into_iter() {
                users.push(line.parse()?);
            }

            Ok(Self{
                users,
                typ: auth_type.unwrap_or(AuthType::Basic),
                realm: realm
                    .unwrap()
                    .replace("\"", "")
                    .into()
            })
        }
    }
}
