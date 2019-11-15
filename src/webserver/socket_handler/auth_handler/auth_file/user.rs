use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct User {
    pub name: String,
    pub pass: String
}

#[derive(Debug, PartialEq)]
pub struct UserParseError {
    had: String
}

impl FromStr for User {
    type Err = UserParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pieces: Vec<_> = s.split(":")
            .map(|s| s.trim())
            .collect();

        if pieces.len() == 2 {
            let name = pieces[0];
            let pass = pieces[1];

            Ok(Self {
                name: name.into(),
                pass: pass.into()
            })
        }else if pieces.len() == 3 {
            let name = pieces[0];
            let pass = pieces[2];

            Ok(Self {
                name: name.into(),
                pass: pass.into()
            })
        }else{
            Err(UserParseError{
                had: s.into()
            })
        }
    }
}
