use std::str::FromStr;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum SuppliedAuth {
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

                Ok(SuppliedAuth::Basic {
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
                        .ok_or(SuppliedAuthError::InvalidItemFormat(field.into()))?
                        .trim();
                    let b = ab_iter.nth(0)
                        .ok_or(SuppliedAuthError::InvalidItemFormat(field.into()))?
                        .trim();

                    holder.insert(a.to_lowercase(), b);
                }

                Ok(SuppliedAuth::Digest{
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

