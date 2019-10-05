use serde::{Serialize, Deserialize};
use std::path::{PathBuf, Path};
use crate::webserver::responses::StatusCode;
use regex::Regex;
use num_traits::FromPrimitive;

use crate::CONFIG;

lazy_static::lazy_static! {
    pub static ref REDIRECTS: Vec<Redirect> = {
        lazy_static::initialize(&CONFIG);

        let content: Vec<config::Value> = 
            CONFIG.get_array("redirects")
            .expect("couldn't find redirects in config structure");

        let content: Vec<TempRedirect> = content
            .into_iter()
            .map(|conf| {
                conf.try_into()
                    .expect("failed to deserialize redirect")
            })
            .collect();

        content
            .into_iter()
            .map(|conf| {
                Redirect {
                    regex: Regex::new(&conf.regex)
                        .expect("invalid regex"),
                    path:  PathBuf::from(conf.url),
                    code:  FromPrimitive::from_u32(conf.code)
                        .unwrap_or(StatusCode::Unknown)
                }
            })
            .collect()
    };
}

#[derive(Serialize, Deserialize)]
struct TempRedirect {
    regex: String,
    url:   String,
    code:  u32
}

pub struct Redirect {
    regex:    Regex,
    pub path: PathBuf,
    pub code: StatusCode
}

impl Redirect {
    pub fn matches(&self, path: &Path) -> bool {
        self.regex.is_match(&path.to_string_lossy())
    }
}
