use nom::{
    bytes::complete::tag,
    combinator::map,
    multi::many0,
    sequence::{terminated, tuple},
    IResult,
};

use super::{filepath, settle_parse_result, sha, ObjectName, WorkPath};

#[derive(Debug, PartialEq, Eq)]
pub struct RefPair {
    refname: ObjectName,
    path: WorkPath,
}

impl From<(ObjectName, WorkPath)> for RefPair {
    fn from(pair: (ObjectName, WorkPath)) -> Self {
        let (refname, path) = pair;
        RefPair { refname, path }
    }
}

pub fn parse(input: &str) -> super::Result<&str, Vec<RefPair>> {
    settle_parse_result(many0(terminated(ref_pair, tag("\n")))(input))
}

fn ref_pair(input: &str) -> IResult<&str, RefPair> {
    map(
        tuple((terminated(sha, tag("\t")), filepath)),
        RefPair::from,
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_pairs_parse() {
        let lines = parse(include_str!("testdata/mezzo-ls-remote")).unwrap();
        assert_eq!(lines.len(), 730)
    }

    #[test]
    fn ref_pair_parse() {
        assert_eq!(
            ref_pair("d4ae7077d4ed711a10e89908ab91999ce326dfc0\trefs/heads/approvals_template"),
            Ok((
                "",
                RefPair {
                    refname: "d4ae7077d4ed711a10e89908ab91999ce326dfc0".into(),
                    path: "refs/heads/approvals_template".into(),
                }
            ))
        )
    }
}
