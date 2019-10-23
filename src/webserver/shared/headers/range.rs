use std::str::FromStr;
use std::num::ParseIntError;

use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult
};

#[derive(Debug, PartialEq)]
pub struct Range {
    pub start: i64,
    pub end:   Option<i64>
}

#[derive(Debug, PartialEq)]
pub enum RangeError {
    InvalidRange(String),
    UnknownUnit(String),
    InvalidNumber(ParseIntError)
}

impl Display for RangeError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        use RangeError::*;
        match self {
            InvalidRange(r) =>
                write!(fmt, "invalid range: '{}'", r),
            UnknownUnit(u) =>
                write!(fmt, "unknown unit: '{}'", u),
            InvalidNumber(n) =>
                write!(fmt, "failed when parsing number: '{}'", n)
        }
    }
}

impl FromStr for Range {
    type Err = RangeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces: Vec<_> = s.split("-")
            .map(|s| s.trim())
            .collect();

        debug_assert_eq!(pieces.len(), 2);
        let a = pieces[0];
        let b = pieces[1];

        if a.is_empty() {
            Ok(Self{
                start: -b.parse()
                    .map_err(RangeError::InvalidNumber)?,
                end: None
            })
        }else if pieces[1].is_empty() {
            Ok(Self{
                start: a.parse()
                    .map_err(RangeError::InvalidNumber)?,
                end: None
            })
        }else{
            Ok(Self{
                start: a.parse()
                    .map_err(RangeError::InvalidNumber)?,
                end: Some(b.parse()
                    .map_err(RangeError::InvalidNumber)?),
            })
        }
    }
}

pub struct RangeList{
    pub ranges: Vec<Range>,
    pub unit:   String
}

impl FromStr for RangeList {
    type Err = RangeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces: Vec<_> = s.split("=")
            .map(|s| s.trim())
            .collect();
        
        debug_assert_eq!(pieces.len(), 2);
        let unit = pieces[0];

        if unit != "bytes" {
            return Err(RangeError::UnknownUnit(unit.into()));
        }

        let ranges: Vec<Result<Range, _>> = pieces[1].split(",")
            .map(|s| s.trim())
            .map(|s| s.parse())
            .collect();
        
        let mut ret = Vec::new();
        for range in ranges.into_iter() {
            match range {
                Ok(range) =>
                    ret.push(range),
                Err(err) => {
                    log::warn!(
                        "error occurred when parsing range, {}",
                        err
                    );
                    return Err(RangeError::InvalidRange(
                        s.into()
                    ))
                }
            }
        }

        Ok(Self{
            unit: unit.into(),
            ranges: ret
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let parsed: RangeList = "bytes=5-"
            .parse()
            .unwrap();

        assert_eq!(parsed.unit, "bytes");
        assert_eq!(parsed.ranges, &[
            Range{start: 5, end: None}
        ]);

        let parsed: RangeList = "bytes=5-20,10-100, -50, 2-"
            .parse()
            .unwrap();

        assert_eq!(parsed.unit, "bytes");
        assert_eq!(parsed.ranges, &[
            Range{start:   5, end: Some(20)},
            Range{start:  10, end: Some(100)},
            Range{start: -50, end: None},
            Range{start: 2,   end: None}
        ]);

        let parsed: RangeList = "bytes=5-20,10-100, -50, 2-, 10-90"
            .parse()
            .unwrap();

        assert_eq!(parsed.unit, "bytes");
        assert_eq!(parsed.ranges, &[
            Range{start:   5, end: Some(20)},
            Range{start:  10, end: Some(100)},
            Range{start: -50, end: None},
            Range{start:   2, end: None},
            Range{start:  10, end: Some(90)}
        ]);
    }
}
