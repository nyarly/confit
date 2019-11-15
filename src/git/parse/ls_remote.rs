use nom::{
    IResult,
    combinator::map,
    sequence::{tuple},
};

use std::ffi::OsString;
use super::{sha,filepath};

#[derive(Debug, PartialEq)]
struct RefPair {
    refname: String,
    path: OsString,
}

impl From<(String, OsString)> for RefPair {
    fn from(pair: (String, OsString)) -> Self {
        let (refname, path) = pair;
        RefPair{ refname, path }
    }
}

/*
put fn ref_pairs(input: &str) -> Result<Vec<RefPair>, String> {
    match many0(terminated(status_line, tag("\n")))(input) {
}
*/

fn ref_pair(input: &str) -> IResult<&str, RefPair> {
    map(tuple((map(sha, String::from), filepath)), RefPair::from)(input)
}
