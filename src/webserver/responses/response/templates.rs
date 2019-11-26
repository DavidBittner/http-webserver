use crate::webserver::responses::StatusCode;

use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
pub struct ErrorTemplate {
    code_phrase: String,
    code_num:    usize,
    description: String,
}

impl ErrorTemplate {
    pub fn new(code: StatusCode, desc: &str) -> Self {
        Self {
            code_phrase: format!("{}", code),
            code_num:    code.to_num(),
            description: desc.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    path: PathBuf,
    name: String,
    #[serde(with = "ts_seconds")]
    date: DateTime<Utc>,
    size: u64,
}

#[derive(Serialize, Deserialize)]
pub struct DirectoryListing {
    dir_path: PathBuf,
    files:    Vec<FileInfo>,
}

impl DirectoryListing {
    pub fn new(path: &Path) -> std::io::Result<Self> {
        use std::time::SystemTime;

        let mut files = Vec::new();
        for file in std::fs::read_dir(path)? {
            let file = file?;
            let meta = file.metadata()?;

            files.push(FileInfo {
                path: file
                    .path()
                    .strip_prefix(path)
                    .unwrap_or(&PathBuf::default())
                    .into(),
                name: file.file_name().to_string_lossy().into(),
                date: meta.modified().unwrap_or(SystemTime::now()).into(),
                size: meta.len(),
            });
        }

        Ok(Self {
            dir_path: path.file_name().unwrap_or(Default::default()).into(),
            files,
        })
    }
}
