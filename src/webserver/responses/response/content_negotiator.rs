use std::path::{PathBuf, Path};
use super::map_file;
use mime::*;
use crate::webserver::shared::headers::*;

///This struct is used to find the most relevant content
///at a given URL. This is by the Content Negotation RFC.
pub struct ContentNegotiator<'a, 'b> {
    path:    &'a Path,
    headers: &'b HeaderList,
}

pub enum NegotiationError {
    IoError(std::io::Error),
    MultipleResponses(Vec<(u32, PathBuf)>),
    NoMatches,
    NotAcceptable,
}

impl From<std::io::Error> for NegotiationError {
    fn from(err: std::io::Error) -> Self {
        NegotiationError::IoError(err)
    }
}

impl<'a, 'b> ContentNegotiator<'a, 'b> {
    pub fn new(path: &'a Path, headers: &'b HeaderList) -> Self {
        Self {
            headers: headers,
            path:    path
        }
    }

    pub fn best_choice(&self) -> Result<Vec<PathBuf>, NegotiationError> {
        let mut paths = Vec::new();
        let stub: String = self.path.file_name().unwrap()
            .to_string_lossy()
            .into();

        let root = self.path.parent().unwrap();

        let types: RankedEntryList<Mime> = RankedEntryList::new_list(
            self.headers.get(ACCEPT).unwrap_or("")
        ).unwrap();

        let langs: RankedEntryList<String> = RankedEntryList::new_list(
            self.headers.get(ACCEPT_LANGUAGE).unwrap_or("")
        ).unwrap();

        let encodings: RankedEntryList<String> = RankedEntryList::new_list(
            self.headers.get(ACCEPT_ENCODING).unwrap_or("")
        ).unwrap();

        let charset: RankedEntryList<String> = RankedEntryList::new_list(
            self.headers.get(ACCEPT_CHARSET).unwrap_or("")
        ).unwrap();

        if types.has_zeroes()     ||
           encodings.has_zeroes() ||
           langs.has_zeroes()     ||
           charset.has_zeroes()
        {
            return Err(NegotiationError::NotAcceptable);
        }

        for file in std::fs::read_dir(root)? {
            if let Ok(file) = file {
                let file_path = file.path();
                if let Some(file_name) = file_path.file_stem() {
                    let file_name: String = file_name.to_string_lossy()
                        .into();
                    if file_name.as_str().starts_with(&stub) {
                        paths.push((0, file_path));
                    }
                }
            }
        }

        let paths = types.filter(paths, |path, entry| {
            let desc = map_file(path).typ;
            if entry.type_() == desc.type_() ||
               entry.type_() == "*"
            {
                entry.subtype() == "*" ||
                entry.subtype() == desc.subtype()
            }else{
                false
            }
        });

        let paths = langs.filter(paths, |path, entry| {
            *entry == map_file(path).lang
        });

        let paths = charset.filter(paths, |path, entry| {
            let desc = map_file(path).charset;
            if let Some(charset) = desc {
                charset == *entry  
            }else{
                false
            }
        });

        let mut paths = encodings.filter(paths, |path, entry| {
            *entry == map_file(path).enc
                .unwrap_or("".into())
        });

        paths.sort_by(|(score_a, _), (score_b, _)| score_a.cmp(score_b));
        if paths.len() >= 2 {
            let (a, _) = paths[paths.len()-1];
            let (b, _) = paths[paths.len()-2];

            if a != b {
                let temp = paths.pop().unwrap();
                let mut paths = Vec::new();
                paths.push(temp.1);

                Ok(paths)
            }else{
                return Err(NegotiationError::MultipleResponses(paths));
            }
        }else if paths.len() == 1 {
            Ok(paths.into_iter().map(|(_, path)| path).collect())
        }else{
            Err(NegotiationError::NoMatches)
        }
    }
}
