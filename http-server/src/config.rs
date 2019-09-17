use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub port: u16,
    pub addr: IpAddr
}
