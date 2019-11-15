use std::net::IpAddr;
use std::time::Duration;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer};

fn deserialize_duration<'de, D> (des: D) -> Result<Duration, D::Error>
    where D: Deserializer<'de>
{
    let val = u64::deserialize(des)?;
    Ok(Duration::from_millis(val))
}

#[derive(Deserialize, Debug)]
pub struct Auth {
    #[serde(default)]
    pub file_name:   String,
    #[serde(default)]
    pub private_key: String
}

#[derive(Deserialize, Debug)]
pub struct Redirect {
    pub regex: String,
    pub url:   String,
    pub code:  u32
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub port: u32,
    pub root: PathBuf,
    pub redirects: Vec<Redirect>,
    pub indexes: Vec<PathBuf>,
    pub addr: IpAddr,
    pub templates: PathBuf,
    #[serde(deserialize_with = "deserialize_duration")]
    pub read_timeout: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub write_timeout: Duration,
    pub max_request_size: usize,
    pub auth: Auth,
}
