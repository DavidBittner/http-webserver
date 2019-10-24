use std::str::FromStr;
use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult
};

#[derive(Debug)]
pub struct RatedEntry<T: FromStr> {
    pub entry: T,
    pub rating: Option<i32>
}

#[derive(Debug, PartialEq)]
pub struct InvalidEntry(String);
impl std::error::Error for InvalidEntry {}

impl Display for InvalidEntry {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "invalid entry: '{}'", self.0)
    }
}

impl<T: FromStr> RatedEntry<T> {
    pub fn new_list(s: &str) -> Result<Vec<T>, <T as FromStr>::Err> {
        if s.is_empty() {
            return Ok(Vec::new());
        }

        let mut ret = Vec::new();
        let pieces: Vec<_> = s.split(",")
            .map(|s| s.trim())
            .collect();

        for piece in pieces.into_iter() {
            ret.push(piece.parse()?);
        }

        Ok(ret)
    }
}

impl<T: FromStr> FromStr for RatedEntry<T> {
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
                rating: Some((val * 1000.) as i32)
            })
        }
    }
}
