pub mod user;
pub use user::*;

use crate::webserver::shared::method::*;

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::Path;
use std::str::FromStr;

use regex::Regex;

lazy_static::lazy_static! {
    pub static ref USER: Regex =
        Regex::new(
            "([A-z]|[0-9])+:([A-z]|[0-9])+(:([A-z]|[0-9])+)?"
        )
        .expect("user");

    pub static ref VALUE: Regex =
        Regex::new(
            "(?P<name>.+)=(?P<value>.+)"
        )
        .expect("value regex");
}

#[derive(Debug, PartialEq)]
pub struct AuthFile {
    pub typ:    AuthType,
    pub realm:  String,
    pub users:  Vec<User>,
    pub allows: Vec<Method>,
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
    Digest,
}

impl Display for AuthType {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        use AuthType::*;

        match self {
            Basic => write!(fmt, "Basic"),
            Digest => write!(fmt, "Digest"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct AuthTypeParseError {
    what: String,
}

impl FromStr for AuthType {
    type Err = AuthTypeParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "basic" => Ok(AuthType::Basic),
            "digest" => Ok(AuthType::Digest),
            _ => Err(AuthTypeParseError { what: s.into() }),
        }
    }
}

#[derive(Debug)]
pub enum AuthFileParseError {
    InvalidFile(String),
    MissingRealm,
    MissingAuthType,
    UnrecognizedSymbol(String),
    UnrecognizedAuthType(AuthTypeParseError),
    UserParseError(UserParseError),
    IoError(std::io::Error),
}

impl From<std::io::Error> for AuthFileParseError {
    fn from(oth: std::io::Error) -> Self { AuthFileParseError::IoError(oth) }
}

impl From<AuthTypeParseError> for AuthFileParseError {
    fn from(err: AuthTypeParseError) -> Self {
        AuthFileParseError::UnrecognizedAuthType(err)
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
        let lines: Vec<_> = s
            .lines()
            .filter(|s| !s.trim().starts_with("#"))
            .filter(|s| !s.is_empty())
            .map(|s| s.trim())
            .collect();

        if lines.len() < 2 {
            Err(AuthFileParseError::InvalidFile(format!(
                "file only has {} lines",
                lines.len()
            )))
        } else {
            let mut allows = vec![
                Method::Get,
                Method::Options,
                Method::Trace,
                Method::Head,
                Method::Post,
            ];

            let mut users = Vec::new();
            let mut realm = None;
            let mut typ = None;

            for line in lines.into_iter() {
                if USER.is_match(line) {
                    users.push(line.parse()?);
                } else if VALUE.is_match(line) {
                    let caps = VALUE.captures(line).unwrap();

                    let name = caps.name("name").unwrap().as_str();

                    let val = caps.name("value").unwrap().as_str();

                    match name.to_lowercase().as_str() {
                        "realm" => realm = Some(val.replace("\"", "").into()),
                        "authorization-type" => typ = Some(val.parse()?),
                        _ => {
                            return Err(AuthFileParseError::UnrecognizedSymbol(
                                name.into(),
                            ))
                        }
                    }
                } else {
                    match line.to_lowercase().as_str() {
                        "allow-put" => allows.push(Method::Put),
                        "allow-delete" => allows.push(Method::Delete),
                        _ => {
                            return Err(AuthFileParseError::UnrecognizedSymbol(
                                line.into(),
                            ))
                        }
                    }
                }
            }

            let realm = realm.ok_or(AuthFileParseError::MissingRealm)?;
            let typ = typ.ok_or(AuthFileParseError::MissingAuthType)?;

            Ok(Self {
                users,
                allows,
                realm,
                typ,
            })
        }
    }
}
