use std::path::{PathBuf, Path};
use std::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use std::io;

use crate::CONFIG;
use crate::webserver::socket_handler::ROOT;

lazy_static::lazy_static! {
    static ref SECRET_KEY: String = {
        lazy_static::initialize(&CONFIG);

        CONFIG.get_table("auth")
            .expect("auth table")
            .get("private-key")
            .expect("private key not found")
            .clone()
            .into_str()
            .expect("failed to turn into string, auth secret key")
            .into()
    };

    static ref AUTH_FILE_NAME: String = {
        lazy_static::initialize(&CONFIG);

        CONFIG.get_table("auth")
            .expect("auth table")
            .get("file-name")
            .expect("file name not found")
            .clone()
            .into_str()
            .expect("failed to turn into string, auth file name")
            .into()
    };

    static ref AUTH_FILE_CACHE: RwLock<HashMap<Box<Path>, Arc<AuthFile>>> =
        Default::default();
}

#[derive(Debug, PartialEq)]
struct User {
    name: String,
    pass: String
}

#[derive(Debug, PartialEq)]
enum AuthType {
    Basic,
    Digest
}

#[derive(Debug)]
struct AuthFile {
    typ:   AuthType,
    realm: String,
    users: Vec<User>
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

    pub fn check(auth: &SuppliedAuth) -> bool {
        true 
    }

    fn find_config(loc: &Path) -> Option<AuthFile> {
        let file = loc.join(&*AUTH_FILE_NAME);
        if file.exists() {
            Some(AuthFile::new())
        }else{
            let new_loc = loc.parent();
            if let Some(new_loc) = new_loc {
                if new_loc == *ROOT {
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
