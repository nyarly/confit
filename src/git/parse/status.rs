extern crate nom;
use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until, take_while},
    character::complete::one_of,
    combinator::{map, map_res, opt},
    multi::{count, many0},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};
use std::array::TryFromSliceError;
use std::convert::TryFrom;
use std::ffi::OsString;

use super::{filepath, settle_parse_result, sha};

#[derive(Debug, PartialEq)]
pub enum StatusLine {
    One {
        status: StatusPair,
        sub: SubmoduleStatus,
        head_mode: Mode,
        index_mode: Mode,
        worktree_mode: Mode,
        head_obj: String,
        index_obj: String,
        path: OsString,
    },
    Two {
        status: StatusPair,
        sub: SubmoduleStatus,
        head_mode: Mode,
        index_mode: Mode,
        worktree_mode: Mode,
        head_obj: String,
        index_obj: String,
        change_score: ChangeScore,
        path: OsString,
        orig_path: OsString,
    },
    Unmerged {
        status: StatusPair,
        sub: SubmoduleStatus,
        stage1_mode: Mode,
        stage2_mode: Mode,
        stage3_mode: Mode,
        worktree_mode: Mode,
        stage1_obj: String,
        stage2_obj: String,
        stage3_obj: String,
        path: OsString,
    },
    Untracked {
        path: OsString,
    },
    Ignored {
        path: OsString,
    },
}

#[derive(Debug, PartialEq)]
pub struct Status {
    pub branch: Option<Branch>,
    pub lines: Vec<StatusLine>,
}

#[derive(Debug, PartialEq)]
pub struct Branch {
    pub oid: Oid,   //(intial): None, or Some("0a03ba3cfde6472cb7431958dd78ca2c0d65de74")
    pub head: Head, //: None for (detached) or Some("bulk_update_api")
    pub upstream: Option<String>, //: "origin/bulk_update_api",
    pub commits: Option<Commits>, //: Commits(0,0),
}

#[derive(Debug, PartialEq)]
pub struct Commits(u16, u16);

#[derive(Debug, PartialEq)]
pub enum Oid {
    Initial,
    Commit(String),
}

#[derive(Debug, PartialEq)]
pub enum Head {
    Detached,
    Branch(String),
}

pub fn parse(input: &str) -> super::Result<&str, Status> {
    settle_parse_result(status(input))
}

fn status(input: &str) -> IResult<&str, Status> {
    let (i, branch) = opt(branch)(input)?;
    let (i, lines) = many0(terminated(status_line, tag("\n")))(i)?;
    Ok((i, Status { branch, lines }))
}

fn branch(input: &str) -> IResult<&str, Branch> {
    let (i, oid) = branch_oid(input)?;
    let (i, head) = branch_head(i)?;
    let (i, upstream) = opt(branch_upstream)(i)?;
    let (i, commits) = opt(branch_commits)(i)?;
    Ok((
        i,
        Branch {
            oid,
            head,
            upstream,
            commits,
        },
    ))
}

fn branch_oid(input: &str) -> IResult<&str, Oid> {
    delimited(
        tag("# branch.oid "),
        alt((
            map(tag("(initial)"), |_| Oid::Initial),
            map(take_until("\n"), |s: &str| Oid::Commit(s.into())),
        )),
        tag("\n"),
    )(input)
}

fn branch_head(input: &str) -> IResult<&str, Head> {
    delimited(
        tag("# branch.head "),
        alt((
            map(tag("(detached)"), |_| Head::Detached),
            map(take_until("\n"), |s: &str| Head::Branch(s.into())),
        )),
        tag("\n"),
    )(input)
}

fn branch_upstream(input: &str) -> IResult<&str, String> {
    delimited(
        tag("# branch.upstream "),
        map(take_until("\n"), |s: &str| s.into()),
        tag("\n"),
    )(input)
}

fn branch_commits(input: &str) -> IResult<&str, Commits> {
    map(
        delimited(
            tag("# branch.ab "),
            separated_pair(tagged_commits("+"), tag(" "), tagged_commits("-")),
            tag("\n"),
        ),
        |(a, b)| Commits(a, b),
    )(input)
}

fn tagged_commits<'a>(pattern: &'static str) -> impl Fn(&'a str) -> IResult<&'a str, u16> {
    map_res(
        preceded(tag(pattern), take_while(|c: char| c.is_digit(10))),
        |n: &str| n.parse(),
    )
}

pub fn status_lines(input: &str) -> IResult<&str, Vec<StatusLine>> {
    many0(terminated(status_line, tag("\n")))(input)
}

#[derive(Debug, PartialEq)]
pub struct Mode([u8; 6]);

impl TryFrom<Vec<u8>> for Mode {
    type Error = TryFromSliceError;
    fn try_from(v: Vec<u8>) -> Result<Mode, TryFromSliceError> {
        Ok(Mode(<[u8; 6]>::try_from(&v[..])?))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LineStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Unmerged,
    Untracked,
    Ignored,
}

#[derive(Debug, PartialEq)]
pub struct StatusPair {
    pub staged: LineStatus,
    pub unstaged: LineStatus,
}

impl From<(LineStatus, LineStatus)> for StatusPair {
    fn from(t: (LineStatus, LineStatus)) -> StatusPair {
        let (staged, unstaged) = t;
        StatusPair { staged, unstaged }
    }
}

#[derive(Debug, PartialEq)]
pub enum SubmoduleStatus {
    Not,
    Is(bool, bool, bool),
}

#[derive(Debug, PartialEq)]
pub enum ChangeScore {
    Rename(u8),
    Copy(u8),
}

fn from_octal(input: &str) -> Result<u8, std::num::ParseIntError> {
    u8::from_str_radix(input, 8)
}

fn octal(input: &str) -> IResult<&str, u8> {
    map_res(take(1u8), from_octal)(input)
}

fn mode(input: &str) -> IResult<&str, Mode> {
    map_res(count(octal, 6), Mode::try_from)(input)
}

fn line_status(input: &str) -> IResult<&str, LineStatus> {
    use LineStatus::*;
    take(1u8)(input).and_then(|parsed| match parsed {
        (i, ".") => Ok((i, Unmodified)),
        (i, "M") => Ok((i, Modified)),
        (i, "A") => Ok((i, Added)),
        (i, "D") => Ok((i, Deleted)),
        (i, "R") => Ok((i, Renamed)),
        (i, "C") => Ok((i, Copied)),
        (i, "U") => Ok((i, Unmerged)),
        (i, "?") => Ok((i, Untracked)),
        (i, "!") => Ok((i, Ignored)),

        (i, _) => Err(nom::Err::Error((i, nom::error::ErrorKind::OneOf))),
    })
}

fn status_pair(input: &str) -> IResult<&str, StatusPair> {
    map(tuple((line_status, line_status)), StatusPair::from)(input)
}

fn submodule_status_flag<I>(pattern: &'static str) -> impl Fn(I) -> IResult<I, bool>
where
    I: nom::InputIter<Item = char> + nom::Slice<std::ops::RangeFrom<usize>>,
{
    map(one_of(pattern), |c| c != '.')
}

fn submodule_status(input: &str) -> IResult<&str, SubmoduleStatus> {
    let (i, s) = one_of("NS")(input)?;
    match s {
        'N' => map(count(one_of("."), 3), |_| SubmoduleStatus::Not)(i),
        'S' => map(
            tuple((
                submodule_status_flag("C."),
                submodule_status_flag("M."),
                submodule_status_flag("U."),
            )),
            |(c, m, u)| SubmoduleStatus::Is(c, m, u),
        )(i),
        _ => panic!("one_of violated contract"),
    }
}

fn tagged_score<'a>(pattern: &'static str) -> impl Fn(&'a str) -> IResult<&'a str, u8> {
    map_res(
        preceded(tag(pattern), take_while(|c: char| c.is_digit(10))),
        |n: &str| n.parse(),
    )
}

fn change_score(input: &str) -> IResult<&str, ChangeScore> {
    alt((
        map(tagged_score("R"), ChangeScore::Rename),
        map(tagged_score("C"), ChangeScore::Copy),
    ))(input)
}

fn one_file_line(input: &str) -> IResult<&str, StatusLine> {
    let (i, status) = terminated(status_pair, tag(" "))(input)?;
    let (i, sub) = terminated(submodule_status, tag(" "))(i)?;
    let (i, head_mode) = terminated(mode, tag(" "))(i)?;
    let (i, index_mode) = terminated(mode, tag(" "))(i)?;
    let (i, worktree_mode) = terminated(mode, tag(" "))(i)?;
    let (i, head_obj) = terminated(sha, tag(" "))(i)?;
    let (i, index_obj) = terminated(sha, tag(" "))(i)?;
    let (i, path) = filepath(i)?;
    Ok((
        i,
        StatusLine::One {
            status,
            sub,
            head_mode,
            index_mode,
            worktree_mode,
            head_obj: head_obj.into(),
            index_obj: index_obj.into(),
            path,
        },
    ))
}

fn two_file_line(input: &str) -> IResult<&str, StatusLine> {
    let (i, status) = terminated(status_pair, tag(" "))(input)?;
    let (i, sub) = terminated(submodule_status, tag(" "))(i)?;
    let (i, head_mode) = terminated(mode, tag(" "))(i)?;
    let (i, index_mode) = terminated(mode, tag(" "))(i)?;
    let (i, worktree_mode) = terminated(mode, tag(" "))(i)?;
    let (i, head_obj) = terminated(sha, tag(" "))(i)?;
    let (i, index_obj) = terminated(sha, tag(" "))(i)?;
    let (i, change_score) = terminated(change_score, tag(" "))(i)?;
    let (i, path) = terminated(filepath, tag("\t"))(i)?;
    let (i, orig_path) = filepath(i)?;
    Ok((
        i,
        StatusLine::Two {
            status,
            sub,
            head_mode,
            index_mode,
            worktree_mode,
            head_obj: head_obj.into(),
            index_obj: index_obj.into(),
            change_score,
            path,
            orig_path,
        },
    ))
}

fn unmerged_file_line(input: &str) -> IResult<&str, StatusLine> {
    let (i, status) = terminated(status_pair, tag(" "))(input)?;
    let (i, sub) = terminated(submodule_status, tag(" "))(i)?;
    let (i, stage1_mode) = terminated(mode, tag(" "))(i)?;
    let (i, stage2_mode) = terminated(mode, tag(" "))(i)?;
    let (i, stage3_mode) = terminated(mode, tag(" "))(i)?;
    let (i, worktree_mode) = terminated(mode, tag(" "))(i)?;
    let (i, stage1_obj) = terminated(sha, tag(" "))(i)?;
    let (i, stage2_obj) = terminated(sha, tag(" "))(i)?;
    let (i, stage3_obj) = terminated(sha, tag(" "))(i)?;
    let (i, path) = terminated(filepath, tag("\t"))(i)?;
    Ok((
        i,
        StatusLine::Unmerged {
            status,
            sub,
            stage1_mode,
            stage2_mode,
            stage3_mode,
            worktree_mode,
            stage1_obj: stage1_obj.into(),
            stage2_obj: stage2_obj.into(),
            stage3_obj: stage3_obj.into(),
            path,
        },
    ))
}

fn untracked_line(input: &str) -> IResult<&str, StatusLine> {
    let (i, path) = terminated(filepath, tag("\t"))(input)?;
    Ok((i, StatusLine::Untracked { path }))
}

fn ignored_line(input: &str) -> IResult<&str, StatusLine> {
    let (i, path) = terminated(filepath, tag("\t"))(input)?;
    Ok((i, StatusLine::Ignored { path }))
}

fn status_line(input: &str) -> IResult<&str, StatusLine> {
    alt((
        preceded(tag("1 "), one_file_line),
        preceded(tag("2 "), two_file_line),
        preceded(tag("u "), unmerged_file_line),
        preceded(tag("? "), untracked_line),
        preceded(tag("! "), ignored_line),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom;

    #[test]
    fn full_parse() {
        assert_eq!(
            parse(include_str!("testdata/mezzo-status-2")).unwrap(),
            Status {
                branch: Some(Branch {
                    oid: Oid::Commit("0a03ba3cfde6472cb7431958dd78ca2c0d65de74".into()),
                    head: Head::Branch("bulk_update_api".into()),
                    upstream: Some("origin/bulk_update_api".into()),
                    commits: Some(Commits(0, 0))
                }),
                lines: vec![StatusLine::One {
                    status: StatusPair {
                        staged: LineStatus::Unmodified,
                        unstaged: LineStatus::Modified
                    },
                    sub: SubmoduleStatus::Not,
                    head_mode: Mode([1, 0, 0, 6, 4, 4]),
                    index_mode: Mode([1, 0, 0, 6, 4, 4]),
                    worktree_mode: Mode([1, 0, 0, 6, 4, 4]),
                    head_obj: "befd8a0574f48b0f17a655c8ed4e5a6353b460ac".into(),
                    index_obj: "befd8a0574f48b0f17a655c8ed4e5a6353b460ac".into(),
                    path: "spec/controllers/service_requests_controller_spec.rb".into()
                }]
            }
        )
    }

    #[test]
    fn branch_parse() {
        assert_eq!(
            branch(
                "# branch.oid 0a03ba3cfde6472cb7431958dd78ca2c0d65de74\n\
           # branch.head bulk_update_api\n\
           # branch.upstream origin/bulk_update_api\n\
           # branch.ab +0 -0\n"
            ),
            Ok((
                "",
                Branch {
                    oid: Oid::Commit("0a03ba3cfde6472cb7431958dd78ca2c0d65de74".into()),
                    head: Head::Branch("bulk_update_api".into()),
                    upstream: Some("origin/bulk_update_api".into()),
                    commits: Some(Commits(0, 0)),
                }
            ))
        )
    }

    #[test]
    fn mode_parse() {
        assert_eq!(mode("100644"), Ok(("", Mode([1, 0, 0, 6, 4, 4]))));
        assert_eq!(mode("777777"), Ok(("", Mode([7, 7, 7, 7, 7, 7]))));
        assert_eq!(mode("000000"), Ok(("", Mode([0, 0, 0, 0, 0, 0]))));

        assert_eq!(
            mode("00000"),
            Err(nom::Err::Error(("", nom::error::ErrorKind::Eof)))
        );
        assert_eq!(
            mode("80000"),
            Err(nom::Err::Error(("80000", nom::error::ErrorKind::MapRes)))
        );
    }

    #[test]
    fn sha_parse() {
        assert_eq!(
            sha("11e1a9446255b2e9bb3eea5105e52967dbf9b1ea"),
            Ok(("", "11e1a9446255b2e9bb3eea5105e52967dbf9b1ea"))
        );
    }

    #[test]
    fn status_pair_parse() {
        assert_eq!(
            status_pair(".."),
            Ok((
                "",
                StatusPair {
                    staged: LineStatus::Unmodified,
                    unstaged: LineStatus::Unmodified
                }
            ))
        );
        assert_eq!(
            status_pair("R."),
            Ok((
                "",
                StatusPair {
                    staged: LineStatus::Renamed,
                    unstaged: LineStatus::Unmodified
                }
            ))
        );
        assert_eq!(
            status_pair(".M"),
            Ok((
                "",
                StatusPair {
                    staged: LineStatus::Unmodified,
                    unstaged: LineStatus::Modified
                }
            ))
        )
    }

    #[test]
    fn submodule_status_parse() {
        assert_eq!(submodule_status("N..."), Ok(("", SubmoduleStatus::Not)));
        assert_eq!(
            submodule_status("SCMU"),
            Ok(("", SubmoduleStatus::Is(true, true, true)))
        );
    }

    #[test]
    fn test_path() {
        assert_eq!(
            filepath("README-2.md\tREADME.md"),
            Ok(("\tREADME.md", OsString::from("README-2.md")))
        );
        assert_eq!(
            filepath("README-2.md"),
            Ok(("", OsString::from("README-2.md")))
        );
    }

    #[test]
    fn change_score_parse() {
        assert_eq!(change_score("R75"), Ok(("", ChangeScore::Rename(75))));
        assert_eq!(change_score("C90"), Ok(("", ChangeScore::Copy(90))))
    }

    #[test]
    fn status_lines_parse() {
        let (_rest, lines) = status_lines(include_str!("testdata/mezzo-status-1")).unwrap();

        assert_eq!(lines.len(), 3);

        assert_eq!(
            lines[0],
            StatusLine::One {
                status: StatusPair {
                    staged: LineStatus::Unmodified,
                    unstaged: LineStatus::Deleted
                },
                sub: SubmoduleStatus::Not,
                head_mode: Mode([1, 0, 0, 6, 4, 4]),
                index_mode: Mode([1, 0, 0, 6, 4, 4]),
                worktree_mode: Mode([0, 0, 0, 0, 0, 0]),
                head_obj: "11e1a9446255b2e9bb3eea5105e52967dbf9b1ea".into(),
                index_obj: "11e1a9446255b2e9bb3eea5105e52967dbf9b1ea".into(),
                path: "README.md".into()
            }
        );
        assert_eq!(
            lines[1],
            StatusLine::One {
                status: StatusPair {
                    staged: LineStatus::Added,
                    unstaged: LineStatus::Unmodified
                },
                sub: SubmoduleStatus::Not,
                head_mode: Mode([0, 0, 0, 0, 0, 0]),
                index_mode: Mode([1, 0, 0, 6, 4, 4]),
                worktree_mode: Mode([1, 0, 0, 6, 4, 4]),
                head_obj: "0000000000000000000000000000000000000000".into(),
                index_obj: "11e1a9446255b2e9bb3eea5105e52967dbf9b1ea".into(),
                path: "old-README.md".into()
            }
        );

        assert_eq!(
            lines[2],
            StatusLine::One {
                status: StatusPair {
                    staged: LineStatus::Unmodified,
                    unstaged: LineStatus::Modified
                },
                sub: SubmoduleStatus::Not,
                head_mode: Mode([1, 0, 0, 6, 4, 4]),
                index_mode: Mode([1, 0, 0, 6, 4, 4]),
                worktree_mode: Mode([1, 0, 0, 6, 4, 4]),
                head_obj: "c68d13474cd3f99964c052e5acc771f4df1e668e".into(),
                index_obj: "c68d13474cd3f99964c052e5acc771f4df1e668e".into(),
                path: OsString::from(
                    "spec/transitions/service_request_transitions/fulfill_spec.rb"
                ),
            }
        );
    }
}
