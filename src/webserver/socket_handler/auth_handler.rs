use std::path::{PathBuf, Path};
use std::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use std::io;

use crate::CONFIG;

lazy_static::lazy_static! {
    static ref AUTH_FILE_CACHE: RwLock<HashMap<Box<Path>, Arc<AuthFile>>> =
        Default::default();
}

#[derive(Debug, PartialEq)]
struct User {
    name: String,
    pass: String
}

#[derive(Debug, PartialEq)]
pub struct UserParseError {
    had: String
}

impl FromStr for User {
    type Err = UserParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces: Vec<_> = s.split(":")
            .map(|s| s.trim())
            .collect();

        if pieces.len() != 2 {
            Err(UserParseError{
                had: s.into()
            })
        }else{
            let name = pieces[0];
            let pass = pieces[1];

            Ok(Self {
                name: name.into(),
                pass: pass.into()
            })
        }
    }
}

#[derive(Debug, PartialEq)]
enum AuthType {
    Basic,
    Digest
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

#[derive(Debug, PartialEq)]
struct AuthFile {
    typ:   AuthType,
    realm: String,
    users: Vec<User>
}

#[derive(Debug, PartialEq)]
pub enum AuthFileParseError {
    InvalidFormat{msg: String},
    UnknownAuthType(AuthTypeParseError),
    UserParseError(UserParseError),
    MalformedEntry{entry: String, had: String}
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

use std::str::FromStr;

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

#[derive(Debug)]
pub struct SuppliedAuth {
}

impl AuthFile {
    fn new() -> Self {
        Self {
            typ: AuthType::Basic,
            realm: "".into(),
            users: Vec::new()
        }
    }
}

pub struct AuthHandler {
    auth_file: Option<Arc<AuthFile>>
}

impl AuthHandler {
    pub fn new(loc: &Path) -> io::Result<Self> {
        let temp_path = if loc.is_dir() {
            loc
        }else{
            loc.parent()
                .unwrap()
        };

        let auth_file = {
            let cache = AUTH_FILE_CACHE
                .read()
                .unwrap();

            if let Some(cached) = cache.get(temp_path) {
                Some(Arc::clone(cached))
            }else{
                Self::find_config(temp_path)
                    .map(|inner| Arc::new(inner))
            }
        };

        if let Some(auth_file) = &auth_file {
            let mut cache = AUTH_FILE_CACHE
                .write()
                .unwrap();

            cache.insert(temp_path.into(), auth_file.clone());
        }

        Ok(Self {
            auth_file: auth_file
        })
    }

    pub fn check(&self, auth: &SuppliedAuth) -> bool {
        if self.auth_file.is_none() {
            true
        }else{
            false
        }
    }

    fn find_config(loc: &Path) -> Option<AuthFile> {
        let file = loc.join(&CONFIG.auth.file_name);
        if file.exists() {
            Some(AuthFile::new())
        }else{
            let new_loc = loc.parent();
            if let Some(new_loc) = new_loc {
                if new_loc == CONFIG.root {
                    None
                }else{
                    Self::find_config(new_loc)
                }
            }else{
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user() {
        let user: User = "name:password"
            .parse()
            .unwrap();

        assert_eq!(User{name: "name".into(), pass: "password".into()}, user);
    }

    #[test]
    fn test_parse_auth_type() {
        let typ: AuthType = "Basic"
            .parse()
            .unwrap();

        assert_eq!(AuthType::Basic, typ);
        let typ: AuthType = " digest"
            .parse()
            .unwrap();

        assert_eq!(AuthType::Digest, typ);
    }

    #[test]
    fn test_parse_auth_file() {
        let auth_file: AuthFile = 
r#"# Hashed lines are comments and order is not important
#
# Following are two special lines:
authorization-type=Basic
realm="Lane Stadium"
# Always quote realm since it might have spaces
#
# User format => name:md5(password)
mln:d3b07384d113edec49eaa6238ad5ff00
bda:c157a79031e1c40f85931829bc5fc552
jbollen:66e0459d0abbc8cd8bd9a88cd226a9b2"#
            .parse()
            .unwrap();

        assert_eq!(AuthFile {
                typ: AuthType::Basic,
                realm: "Lane Stadium".into(),
                users: vec![
                    User{name: "mln".into(), pass: "d3b07384d113edec49eaa6238ad5ff00".into()},
                    User{name: "bda".into(), pass: "c157a79031e1c40f85931829bc5fc552".into()},
                    User{name: "jbollen".into(), pass: "66e0459d0abbc8cd8bd9a88cd226a9b2".into()}
                ]
            },
            auth_file
        );
    }
}
