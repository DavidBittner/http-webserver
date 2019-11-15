mod auth_file;
mod supplied_auth;

use supplied_auth::*;
use auth_file::*;

use std::path::Path;
use std::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

use crate::CONFIG;
use crate::webserver::requests::*;
use crate::webserver::responses::*;
use crate::webserver::shared::*;

lazy_static::lazy_static! {
    static ref AUTH_FILE_CACHE: RwLock<HashMap<Box<Path>, Arc<AuthFile>>> =
        Default::default();
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

                    let username       =
                        &decoded[0..decoded.find(":").unwrap_or(0)];
                    let given_password =
                        &decoded[decoded.find(":").unwrap_or(0)+1..];
                    let password       =
                        auth_file.get_password(username);

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

    pub fn create_passed(loc: &Path, req: &Request, headers: &mut HeaderList) {
        let auth_file = if let Ok(auth) = Self::find_config(loc) {
            if let Some(auth) = auth {
                auth
            }else{
                return;
            }
        }else{
            return;
        };

        if let Some(auth_str) = req.headers.authorization() {
            let auth: SuppliedAuth = auth_str
                .parse()
                .unwrap();

            match auth {
                SuppliedAuth::Basic{..} => (),
                SuppliedAuth::Digest{
                    username,
                    uri,
                    nonce,
                    nc,
                    cnonce,
                    ..
                } => {
                    let password = auth_file.get_password(&username)
                        .unwrap();

                    let a2 = md5::compute(
                        format!(
                            ":{}",
                            uri
                        )
                    );

                    let to_hash = format!(
                        "{a1}:{nonce}:{ncount}:{cnonce}:auth:{a2}",
                        a1     = password,
                        nonce  = nonce,
                        ncount = nc,
                        cnonce = cnonce,
                        a2     = format!("{:x}", a2)
                    );

                    headers.authentication_info(
                        format!("{:x}", md5::compute(to_hash))
                    );
                }
            }
        }else{
            return;
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
