use crate::webserver::requests::Request;
use crate::webserver::responses::{Response, StatusCode};
use crate::webserver::shared::*;
use crate::webserver::socket_handler::SuppliedAuth;
use crate::CONFIG;
use super::super::{SERVER_NAME, SERVER_VERS};

use std::path::{PathBuf, Path};
use std::net::SocketAddr;
use std::io::Write;
use std::process::{
    Command,
    Stdio
};
use std::fmt::{
    Result as FmtResult,
    Formatter,
    Display
};

pub struct CgiHandler<'a> {
    req: &'a Request,
    com: Command,
    buff: Option<String>
}

pub enum CgiHandlerError {
    NoFileName(PathBuf),
    IoError(std::io::Error),
    FromUtf8Error(std::string::FromUtf8Error),
    InvalidStatus(String),
    NoStdinError
}

impl Display for CgiHandlerError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        use CgiHandlerError::*;

        match self {
            NoFileName(path) => write!(
                fmt,
                "a filename could not be retrieved from the path: '{}'",
                path.display()
            ),
            IoError(err)    => write!(
                fmt,
                "{}",
                err
            ),
            NoStdinError => write!(
                fmt,
                "could not open stdin stream"
            ),
            FromUtf8Error(err) => write!(
                fmt,
                "could not parse utf8 from stdout of process: '{}'",
                err
            ),
            InvalidStatus(s) => write!(
                fmt,
                "script returned invalid status line: '{}'",
                s
            )
        }
    }
}

impl From<std::io::Error> for CgiHandlerError {
    fn from(oth: std::io::Error) -> Self {
        CgiHandlerError::IoError(oth)
    }
}

impl From<std::string::FromUtf8Error> for CgiHandlerError {
    fn from(oth: std::string::FromUtf8Error) -> Self {
        CgiHandlerError::FromUtf8Error(oth)
    }
}

type Result<T> = std::result::Result<T, CgiHandlerError>;

impl<'a> CgiHandler<'a> {
    pub fn new(remote: SocketAddr, path: &Path, req: &'a Request) -> Result<CgiHandler<'a>> {
        let envs = Self::generate_env(remote, req);

        log::trace!("running cgi script: '{}'", path.display());
        let mut com = Command::new(path.clone());
        com
            .envs((envs?).into_iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());

        Ok(Self {
            req,
            com,
            buff: None
        })
    }

    pub fn run(mut self) -> Result<Response> {
        let mut child = self.com.spawn()?;
        {
            let stdin  = child.stdin.as_mut()
                .ok_or(CgiHandlerError::NoStdinError)?;

            match self.req.payload {
                Some(ref payload) => {
                    stdin.write(&payload)?;
                    drop(stdin)
                },
                None => drop(stdin)
            }

            let out = child.wait_with_output()?;
            self.buff = Some(String::from_utf8(out.stdout)?);
            if let Some(ref buff) = self.buff {
                log::trace!(
                    "script completed with output:\
                    \n```\n{}\n```",
                    buff
                );
            }
        }

        Ok(self.create_response()?)
    }

    fn create_response(self) -> Result<Response> {
        match self.buff {
            Some(buff) => {
                let header_lines: Vec<&str> = buff
                    .lines()
                    .take_while(|l| !l.is_empty())
                    .collect();

                let header_str: String = header_lines.iter()
                    .map(|l| format!("{}\n", l))
                    .collect();

                match header_str.parse::<HeaderList>() {
                    Ok(mut headers) => {
                        let mut status_c: Option<StatusCode> = None;
                        if let Some(status) = headers.get("status") {
                            let (s, e) = status.split_at(status.find(" ")
                                .ok_or(
                                    CgiHandlerError::InvalidStatus(
                                        status.into()
                                    )
                                )?
                            );
                            status_c = Some(
                                StatusCode::Custom(
                                    String::from(e.trim()),
                                    s.trim().parse()
                                        .map_err(|_|
                                            CgiHandlerError::InvalidStatus(status.into())
                                        )?,
                                )
                            );
                        }

                        headers.remove("status");
                        headers.chunked_encoding();
                        headers.merge(HeaderList::response_headers());

                        let buff: String = buff.lines()
                            .skip(header_lines.len()+1)
                            .map(|l| format!("{}\n", l))
                            .collect();

                        if headers.get("location").is_some() {
                            let code = status_c
                                .unwrap_or(
                                    match self.req.method {
                                        Method::Post => StatusCode::Created,
                                        _            => StatusCode::Found
                                    }
                                );
                            Ok(Response {
                                code,
                                data:   Some(buff.into_bytes().into()),
                                headers
                            })
                        }else if !headers.has(headers::CONTENT_TYPE) {
                            Ok(Response {
                                code: status_c
                                    .unwrap_or(
                                        StatusCode::InternalServerError
                                    ),
                                data: None,
                                headers
                            })
                        }else{
                            Ok(Response {
                                code:   status_c.unwrap_or(StatusCode::Ok),
                                data:   Some(buff.into_bytes().into()),
                                headers
                            })
                        }
                    },
                    Err(err) => {
                        log::warn!(
                            "values matching headers found, but failed to parse: '{:#?}'",
                            err
                        );
                        Ok(Response::internal_error())
                    }
                }
            },
            None => {
                let mut headers = HeaderList::response_headers();
                headers.content("plain/text", None, 0);

                Ok(Response {
                    code:    StatusCode::Ok,
                    data:    None,
                    headers
                })
            }
        }
    }

    fn generate_env(remote: SocketAddr, req: &Request) -> Result<Vec<(String, String)>> {
        let (auth, user) = match req.headers.get(AUTHORIZATION) {
            Some(auth) => {
                match auth.parse::<SuppliedAuth>() {
                    Ok(auth) => {
                        auth.get_info()
                    },
                    Err(_) => {
                        (String::new(), String::new())
                    }
                }
            },
            None => {
                (String::new(), String::new())
            }
        };

        Ok(vec![
            ("SCRIPT_NAME".into(),
             req.path
                .file_stem()
                .ok_or(CgiHandlerError::NoFileName(req.path.clone()))?
                .to_string_lossy()
                .into()
            ),
            ("SCRIPT_URI".into(),
             req.path
                .display()
                .to_string()
                .into()
            ),
            ("SCRIPT_FILENAME".into(),
             req.path
                .file_name()
                .ok_or(CgiHandlerError::NoFileName(req.path.clone()))?
                .to_string_lossy()
                .into()
            ),
            ("QUERY_STRING".into(),
             req.query.clone()
            ),
            ("CONTENT_LENGTH".into(),
             req.headers.get(CONTENT_LENGTH)
                .unwrap_or("0")
                .into()
            ),
            ("CONTENT_TYPE".into(),
             req.headers.get(CONTENT_TYPE)
                .unwrap_or("")
                .into()
            ),
            ("PATH_INFO".into(),
             req.path.clone().display().to_string()
            ),
            ("PATH_TRANSLATED".into(),
             CONFIG.root.join(&req.path).display().to_string()
            ),
            ("REMOTE_ADDR".into(),
             remote.to_string()
            ),
            ("REMOTE_HOST".into(),
             remote.to_string()
            ),
            ("REQUEST_METHOD".into(),
             req.method.to_string()
            ),
            ("SERVER_PROTOCOL".into(),
             "HTTP/1.1".into()
            ),
            ("HTTP_USER_AGENT".into(),
             req.headers.get(USER_AGENT)
                .unwrap_or("")
                .to_owned()
            ),
            ("AUTH_TYPE".into(),
             auth
            ),
            ("SERVER_PORT".into(),
             CONFIG.port.to_string()
            ),
            ("SERVER_SOFTWARE".into(),
             format!("{}-{}", SERVER_NAME, SERVER_VERS)
            ),
            ("SERVER_NAME".into(),
             SERVER_NAME.into()
            ),
            ("REMOTE_USER".into(),
             user
            )
        ])
    }
}
