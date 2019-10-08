use serde::{Serialize, Deserialize};
use std::path::{PathBuf, Path};
use crate::webserver::responses::StatusCode;
use regex::Regex;
use num_traits::FromPrimitive;

use crate::CONFIG;

lazy_static::lazy_static! {
    pub static ref REDIRECTS: Vec<Redirect> = {
        lazy_static::initialize(&CONFIG);

        let content: Vec<config::Value> = CONFIG
            .get_array("redirects")
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
                let regex_url = PathBuf::from(conf.regex);
                let redir_url = PathBuf::from(conf.url);

                let mut regexes = Vec::new();
                let mut paths   = Vec::new();

                for (reg, compo) in regex_url
                    .into_iter()
                    .zip(redir_url.into_iter())
                {
                    regexes.push(Regex::new(&reg.to_string_lossy())
                            .expect("failed to compile regex"));

                    if compo
                        .to_string_lossy()
                        .starts_with("$")
                    {
                        paths.push(None);
                    }else{
                        paths.push(Some(compo
                                .to_string_lossy()
                                .into()));
                    }
                }

                Redirect {
                    code: StatusCode::from_u32(conf.code)
                        .unwrap_or(StatusCode::Unknown),
                    paths: paths,
                    regexs: regexes
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
    regexs:    Vec<Regex>,
    pub paths: Vec<Option<String>>,
    pub code:  StatusCode
}

impl Redirect {
    pub fn matches(&self, path: &Path) -> bool {
        for (regex, comp) in self
            .regexs
            .iter()
            .zip(path.iter())
        {
            if !regex.is_match(&comp.to_string_lossy()) {
                return false;
            }
        }

        true
    }

    pub fn subst(&self, path: &Path) -> PathBuf {
        let mut ret = PathBuf::new();
        for (a, b) in path
            .iter()
            .zip(self.paths.iter())
        {
            ret.push(b
                .clone()
                .unwrap_or(a
                    .to_string_lossy()
                    .into()
                )
            );
        }

        ret
    }
}
