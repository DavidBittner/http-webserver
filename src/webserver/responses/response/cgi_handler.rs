use crate::webserver::requests::Request;
use crate::webserver::responses::{Response, StatusCode};
use crate::webserver::shared::*;

use std::path::{PathBuf, Path};
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
    pub fn new(path: &Path, req: &'a Request) -> Result<CgiHandler<'a>> {
        let envs = Self::generate_env(req);

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
                let header_lines: String = buff.lines()
                    .take_while(|l| !l.is_empty())
                    .collect();

                match header_lines.parse::<HeaderList>() {
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
                            .skip(header_lines.len())
                            .collect();

                        headers.content_length(buff.len());
                        if headers.get("location").is_some() {
                            Ok(Response {
                                code:   status_c.unwrap_or(StatusCode::Found),
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

    fn generate_env(req: &'a Request) -> Result<Vec<(String, String)>> {
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
        ])
    }
}
