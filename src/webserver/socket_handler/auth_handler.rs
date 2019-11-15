use std::path::{PathBuf, Path};
use std::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult
};

use crate::CONFIG;
use crate::webserver::requests::*;
use crate::webserver::responses::*;
use crate::webserver::shared::*;

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

        if pieces.len() == 2 {
            let name = pieces[0];
            let pass = pieces[1];

            Ok(Self {
                name: name.into(),
                pass: pass.into()
            })
        }else if pieces.len() == 3 {
            let name = pieces[0];
            let pass = pieces[2];

            Ok(Self {
                name: name.into(),
                pass: pass.into()
            })
        }else{
            Err(UserParseError{
                had: s.into()
            })
        }
    }
}

#[derive(Debug, PartialEq)]
enum AuthType {
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

#[derive(Debug, PartialEq)]
struct AuthFile {
    typ:   AuthType,
    realm: String,
    users: Vec<User>
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

#[derive(Debug, PartialEq)]
enum SuppliedAuth {
    Basic{
        auth: String
    },
    Digest{
        username: String,
        realm:    String,
        uri:      String,
        qop:      String,
        nonce:    String,
        cnonce:   String,
        nc:       String,
        response: String,
        opaque:   Option<String>
    }
}

#[derive(Debug)]
pub enum SuppliedAuthError {
    InvalidItemFormat(String),
    RequiredFieldNotPresent(String),
    UnknownAuthType(String),
    InvalidBase64(String)
}

fn get_or_error(map: &mut HashMap<String, &str>, field: &str) -> Result<String, SuppliedAuthError> {
    use SuppliedAuthError::*;
    map.get(field)
        .ok_or(RequiredFieldNotPresent(String::from(field)))
        .map(|s| String::from(s.trim().replace("\"", "")))
}

impl FromStr for SuppliedAuth {
    type Err = SuppliedAuthError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let first = s.split(",")
            .nth(0)
            .unwrap();

        let auth_type: &str = first.split_whitespace()
            .nth(0)
            .unwrap()
            .trim();

        match auth_type.to_lowercase().as_str() {
            "basic" => {
                let cont = s.split_whitespace()
                    .nth(1)
                    .unwrap();
                
                Ok(Self::Basic {
                    auth: cont.into()
                })
            },
            "digest" => {
                let mut holder = HashMap::new();
                let section = s.splitn(2, " ")
                    .skip(1)
                    .nth(0)
                    .unwrap();

                for field in section.split(",") {
                    let mut ab_iter = field.split("=");
                    let a = ab_iter.nth(0)
                        .expect("invalid field a")
                        .trim();
                    let b = ab_iter.nth(0)
                        .expect("invalid field b")
                        .trim();

                    holder.insert(a.to_lowercase(), b);
                }

                Ok(Self::Digest{
                    username: get_or_error(&mut holder, "username")?,
                    realm:    get_or_error(&mut holder, "realm")?,
                    uri:      get_or_error(&mut holder, "uri")?,
                    qop:      get_or_error(&mut holder, "qop")?,
                    nonce:    get_or_error(&mut holder, "nonce")?,
                    nc:       get_or_error(&mut holder, "nc")?,
                    cnonce:   get_or_error(&mut holder, "cnonce")?,
                    response: get_or_error(&mut holder, "response")?,
                    opaque:   get_or_error(&mut holder, "opaque")
                        .ok()
                })
            },
            _ => Err(SuppliedAuthError::UnknownAuthType(auth_type.into()))
        }
    }
}

impl AuthFile {
    fn new(path: &Path) -> Result<Self, AuthFileParseError> {
        use std::io::Read;

        let mut buff = String::new();
        let mut file = std::fs::File::open(path)?;

        file.read_to_string(&mut buff)?;
        Ok(buff.parse()?)
    }

    fn get_password(&self, name: &str) -> Option<String> {
        for user in self.users.iter() {
            if user.name == name {
                return Some(String::from(user.pass.clone()));
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct AuthHandler {
    auth_file: Option<Arc<AuthFile>>
}

impl AuthHandler {
    pub fn new(loc: &Path) -> Result<Self, AuthFileParseError> {
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
                Self::find_config(temp_path)?
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
            auth_file
        })
    }

    pub fn check(&self, req: &Request) -> Result<bool, SuppliedAuthError> {
        if let Some(ref auth_file) = self.auth_file {
            let auth_text = req.headers.authorization();
            if auth_text.is_none() {
                return Ok(false);
            }

            let auth: SuppliedAuth = req.headers
                .authorization()
                .unwrap()
                .parse()?;

            match auth {
                SuppliedAuth::Basic{auth} => {
                    let decoded: Vec<u8> = base64::decode(&auth)
                        .map_err(|_| 
                            SuppliedAuthError::InvalidBase64(auth.clone()))?;

                    let decoded = String::from_utf8(decoded)
                        .map_err(|_| 
                            SuppliedAuthError::InvalidBase64(auth))?;

                    let username       = &decoded[0..decoded.find(":").unwrap_or(0)];
                    let given_password = &decoded[decoded.find(":").unwrap_or(0)+1..];
                    let password       = auth_file.get_password(username);

                    let given_password = format!(
                        "{:x}",
                        md5::compute(given_password.as_bytes())
                    );

                    if let Some(password) = password {
                        Ok(given_password == password)
                    }else{
                        Ok(false)
                    }
                },
                SuppliedAuth::Digest{
                    username,
                    realm,
                    uri,
                    qop,
                    nonce,
                    nc,
                    cnonce,
                    response,
                    opaque: _opaque
                } => {
                    if realm != auth_file.realm {
                        return Ok(false);
                    }

                    let password = auth_file.get_password(&username);
                    if password.is_none() {
                        return Ok(false)
                    }

                    let password = password.unwrap();

                    let a2 = match qop.as_str() {
                        "auth"     => 
                            md5::compute(format!("{}:{}",
                                    req.method, uri)),
                        _          =>
                            return Err(
                                SuppliedAuthError::UnknownAuthType(qop)
                            )
                    };

                    let to_hash = format!(
                        "{a1}:{nonce}:{ncount}:{cnonce}:auth:{a2}",
                        a1     = password,
                        nonce  = nonce,
                        ncount = nc,
                        cnonce = cnonce,
                        a2     = format!("{:x}", a2)
                    );

                    let digest = md5::compute(to_hash.as_bytes());

                    log::debug!("{:x} == {} = {}",
                        digest,
                        response.to_lowercase(),
                        format!("{:x}", digest) == response
                    );

                    Ok(format!("{:x}", digest) == response)
                }
            }
        }else{
            Ok(true)
        }
    }

    fn generate_nonce() -> String {
        format!("{:x}", 
            md5::compute(
                format!("{} {}",
                    chrono::Utc::now(),
                    format!("{}:{}",
                        chrono::Utc::now(),
                        CONFIG.auth.private_key
                    )
                )
            )
        )
    }

    pub fn create_unauthorized(&self, req: &Request) -> Response {
        let mut headers = HeaderList::response_headers();
        match self.auth_file {
            Some(ref file) => {
                let auth_header = match file.typ {
                    AuthType::Basic =>
                        format!("Basic realm=\"{}\"",
                            file.realm),
                    AuthType::Digest => {
                        let nonce = Self::generate_nonce();

                        format!(
                            "{} realm=\"{}\", nonce=\"{}\", algorithm=md5, qop=\"auth\"",
                            file.typ,
                            file.realm,
                            nonce
                        )
                    }
                };

                headers.resp_authenticate(auth_header);
            },
            None => panic!(
                        "requested to create unauthorized, yet no auth file exists here: '{}'",
                        req.path.display()
                    )
        }

        Response::unauthorized(headers)
    }

    pub fn create_passed(&self, req: &Request, headers: &mut HeaderList) {
        let auth: SuppliedAuth = req.headers
            .authorization()
            .unwrap()
            .parse()
            .unwrap();

        match auth {
            SuppliedAuth::Basic{auth} => {

            },
            SuppliedAuth::Digest{
                qop,
                nc,
                cnonce,
                ..
            } => {
                let nonce_count = usize::from_str_radix(&nc, 16)
                    .unwrap();

                headers.authentication_info(
                    format!("nextnonce={}", Self::generate_nonce())
                );
            }
        }
    }

    fn find_config(loc: &Path) -> Result<Option<AuthFile>, AuthFileParseError> {
        let file = loc.join(&CONFIG.auth.file_name);
        if file.exists() {
            log::trace!("found auth file: '{}'", file.display());
            Ok(Some(AuthFile::new(&file)?))
        }else{
            let new_loc = loc.parent();
            if let Some(new_loc) = new_loc {
                if new_loc == CONFIG.root {
                    Ok(None)
                }else{
                    Ok(Self::find_config(new_loc)?)
                }
            }else{
                Ok(None)
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

        let auth_file: AuthFile =
r#"#
# A4 password file
#
authorization-type=Digest
#
realm="Colonial Place"
#
bda:Colonial Place:b8e13248f7bb96682093c850d5c7da46
jbollen:Colonial Place:c5d7f97a6ac34b393ba2d252c7331d5a
mln:Colonial Place:53bbb5135e0f39c1eb54804a66a95f08
vaona:Colonial Place:fbcc0f347e4ade65a337a4febc421c81"#
            .parse()
            .unwrap();

        assert_eq!(
            AuthFile {
                typ: AuthType::Digest,
                realm: "Colonial Place".into(),
                users: vec![
                    User{name: "bda".into(),     pass: "b8e13248f7bb96682093c850d5c7da46".into()},
                    User{name: "jbollen".into(), pass: "c5d7f97a6ac34b393ba2d252c7331d5a".into()},
                    User{name: "mln".into(),     pass: "53bbb5135e0f39c1eb54804a66a95f08".into()},
                    User{name: "vaona".into(),   pass: "fbcc0f347e4ade65a337a4febc421c81".into()},
                ]
            },
            auth_file
        );
    }

    #[test]
    fn test_parse_supplied_auth() {
        let sup: SuppliedAuth =
r#"Digest username="Mufasa",
realm="http-auth@example.org",
uri="/dir/index.html",
algorithm=MD5,
nonce="7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v",
nc=00000001,
cnonce="f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ",
qop=auth,
response="8ca523f5e9506fed4657c9700eebdbec",
opaque="FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS"#
            .parse()
            .unwrap();

        assert_eq!(
            SuppliedAuth::Digest {
                username: "Mufasa".into(),
                realm:    "http-auth@example.org".into(),
                uri:      "/dir/index.html".into(),
                nonce:    "7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v".into(),
                nc:       "00000001".into(),
                cnonce:   "f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ".into(),
                qop:      "auth".into(),
                response: "8ca523f5e9506fed4657c9700eebdbec".into(),
                opaque:   Some("FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS".into())
            },
            sup
        );

        let sup: SuppliedAuth = "Basic dGVzdDoxMjPCow=="
            .parse()
            .unwrap();

        assert_eq!(
            SuppliedAuth::Basic {
                auth: "dGVzdDoxMjPCow==".into()
            },
            sup
        );
    }
}
