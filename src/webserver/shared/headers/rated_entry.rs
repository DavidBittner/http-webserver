use std::str::FromStr;
use std::path::{Path, PathBuf};
use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult
};

#[derive(Debug)]
pub struct RankedEntry<T: FromStr> {
    pub entry: T,
    pub rating: Option<u32>
}

#[derive(Debug)]
pub struct RankedEntryList<T: FromStr>(Vec<RankedEntry<T>>);

#[derive(Debug, PartialEq)]
pub struct InvalidEntry(String);
impl std::error::Error for InvalidEntry {}

impl Display for InvalidEntry {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "invalid entry: '{}'", self.0)
    }
}

impl<T: FromStr> RankedEntryList<T> {
    pub fn new_list(s: &str) -> Result<Self, <T as FromStr>::Err> {
        if s.is_empty() {
            return Ok(Self(Vec::new()));
        }

        let mut ret = Vec::new();
        let pieces: Vec<_> = s.split(",")
            .map(|s| s.trim())
            .collect();

        for piece in pieces.into_iter() {
            ret.push(piece.parse()?);
        }

        Ok(Self(ret))
    }

    pub fn filter<'a>(&self, paths: Vec<(u32, PathBuf)>, check: fn(&Path, &T) -> bool) -> Vec<(u32, PathBuf)> {
        if self.0.len() == 0 {
            return paths;
        }

        let mut ret = Vec::new();

        for item in self.0.iter() {
            for (score, path) in paths.iter() {
                if let Some(rating) = item.rating {
                    if rating == 0 {
                        continue;
                    }else{
                        if check(&path, &item.entry) {
                            ret.push((score + rating, path.clone()));
                        }
                    }
                }else{
                    if check(&path, &item.entry) {
                        ret.push((*score, path.clone()));
                    }
                }
            }
        }

        ret
    }

    pub fn has_zeroes(&self) -> bool {
        for item in self.0.iter() {
            if let Some(score) = item.rating {
                if score == 0 {
                    return true;
                }
            }
        }
        false
    }
}

impl<T: FromStr> FromStr for RankedEntry<T> {
    type Err = <T as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces: Vec<_> = s.split(";")
            .map(|s| s.trim())
            .collect();

        if pieces.len() == 1 {
            Ok(Self {
                entry: pieces[0].parse()?,
                rating: None
            })
        }else{
            let val: f32 = pieces[1].split("=")
                .nth(1)
                .unwrap()
                .parse()
                .unwrap_or(0.);

            Ok(Self {
                entry: pieces[0].parse()?,
                rating: Some((val * 1000.) as u32)
            })
        }
    }
}
