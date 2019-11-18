use super::requests::*;
use super::responses::*;
use super::shared::headers::*;
use chrono::{DateTime, Utc};
use num_traits::ToPrimitive;
use std::net::SocketAddr;

pub struct LogEntry {
    client_addr:  SocketAddr,
    client_ident: Option<String>,
    userid:       Option<String>,
    time:         DateTime<Utc>,
    req_line:     String,
    status_code:  StatusCode,
    sent_size:    usize,
}

impl LogEntry {
    pub fn new(addr: &SocketAddr, req: &Request, resp: &Response) -> Self {
        let req_line =
            format!("{} {} {}", req.method, req.path.display(), req.ver);

        let cont_len = resp
            .headers
            .get(CONTENT_LENGTH)
            .clone()
            .unwrap_or(&String::from("0"))
            .parse()
            .unwrap_or(0);

        Self {
            client_addr:  addr.clone(),
            client_ident: None,
            userid:       None,
            time:         Utc::now(),
            req_line:     req_line,
            status_code:  resp.code,
            sent_size:    cont_len,
        }
    }
}

use std::fmt::{Display, Formatter, Result as fmtResult};
impl Display for LogEntry {
    fn fmt(&self, fmt: &mut Formatter) -> fmtResult {
        let ident = self.client_ident.clone().unwrap_or("-".into());
        let usrid = self.userid.clone().unwrap_or("-".into());

        let date_form = self.time.format("%d/%h/%Y:%T %z");

        write!(
            fmt,
            "{} {} {} [{}] \"{}\" {} {}",
            self.client_addr.ip(),
            ident,
            usrid,
            date_form,
            self.req_line,
            self.status_code.to_u16().unwrap(),
            self.sent_size
        )
    }
}
