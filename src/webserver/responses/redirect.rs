use crate::webserver::responses::StatusCode;
use regex::Regex;
use std::convert::TryInto;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::CONFIG;

lazy_static::lazy_static! {
    pub static ref REDIRECTS: Vec<Redirect> = {
        lazy_static::initialize(&CONFIG);

        CONFIG.redirects
            .iter()
            .map(|conf| {
                Redirect {
                    code: StatusCode::from_num(conf.code
                              .try_into()
                              .unwrap()),
                    subst_str: conf.url.clone(),
                    regex: Regex::new(&conf.regex)
                        .expect("failed to compile regex")
                }
            })
            .collect()
    };
}

#[derive(Serialize, Deserialize)]
struct TempRedirect {
    regex: String,
    url:   String,
    code:  u32,
}

#[derive(Debug)]
pub struct Redirect {
    regex:     Regex,
    subst_str: String,
    pub code:  StatusCode,
}

impl Redirect {
    pub fn matches(&self, path: &Path) -> bool {
        self.regex.is_match(&path.to_string_lossy())
    }

    pub fn subst(&self, path: &Path) -> PathBuf {
        let path: String = path.to_string_lossy().into();

        PathBuf::from(
            self.regex
                .replace(&path, self.subst_str.as_str())
                .into_owned(),
        )
    }
}
