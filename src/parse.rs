extern crate nom;
use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till1, take_while, take_while_m_n},
    character::complete::one_of,
    combinator::{map, map_res},
    multi::{count,many0},
    sequence::{preceded, terminated, tuple},
    IResult,
};
use std::array::TryFromSliceError;
use std::convert::TryFrom;
use std::ffi::OsString;

#[derive(Debug, PartialEq)]
struct Mode([u8; 6]);

impl TryFrom<Vec<u8>> for Mode {
    type Error = TryFromSliceError;
    fn try_from(v: Vec<u8>) -> Result<Mode, TryFromSliceError> {
        Ok(Mode(<[u8; 6]>::try_from(&v[..])?))
    }
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

fn is_hex_digit(c: char) -> bool {
    c.is_digit(16)
}

fn sha(input: &str) -> IResult<&str, &str> {
    take_while_m_n(40, 40, is_hex_digit)(input)
}

#[derive(Debug, PartialEq)]
enum Status {
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

fn status(input: &str) -> IResult<&str, Status> {
    take(1u8)(input).and_then(|parsed| match parsed {
        (i, ".") => Ok((i, Status::Unmodified)),
        (i, "M") => Ok((i, Status::Modified)),
        (i, "A") => Ok((i, Status::Added)),
        (i, "D") => Ok((i, Status::Deleted)),
        (i, "R") => Ok((i, Status::Renamed)),
        (i, "C") => Ok((i, Status::Copied)),
        (i, "U") => Ok((i, Status::Unmerged)),
        (i, "?") => Ok((i, Status::Untracked)),
        (i, "!") => Ok((i, Status::Ignored)),

        (i, _) => Err(nom::Err::Error((i, nom::error::ErrorKind::OneOf))),
    })
}

#[derive(Debug, PartialEq)]
struct StatusPair {
    staged: Status,
    unstaged: Status,
}

impl From<(Status, Status)> for StatusPair {
    fn from(t: (Status, Status)) -> StatusPair {
        let (staged, unstaged) = t;
        StatusPair { staged, unstaged }
    }
}

fn status_pair(input: &str) -> IResult<&str, StatusPair> {
    map(tuple((status, status)), StatusPair::from)(input)
}

#[derive(Debug, PartialEq)]
enum SubmoduleStatus {
    Not,
    Is(bool, bool, bool),
}

fn submodule_status_flag<I>(pattern: &'static str) -> impl Fn(I) -> IResult<I, bool>
where
    I: nom::InputIter<Item = char> + nom::Slice<std::ops::RangeFrom<usize>>,
{
    map(one_of(pattern), |c| !(c == '.'))
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

fn end_of_path(input: char) -> bool {
    match input {
        '\t' | '\n' => true,
        _ => false,
    }
}

fn filepath(input: &str) -> IResult<&str, OsString> {
    map(take_till1(end_of_path), OsString::from)(input)
}

#[derive(Debug, PartialEq)]
enum ChangeScore {
    Rename(u8),
    Copy(u8),
}

fn tagged_score<'a>(pattern: &'static str) -> impl Fn(&'a str) -> IResult<&'a str, &str> {
    preceded(tag(pattern), take_while(|c: char| c.is_digit(10)))
}

fn change_score(input: &str) -> IResult<&str, ChangeScore> {
    alt((
        map_res(tagged_score("R"), |n| {
            n.parse().map(|d| ChangeScore::Rename(d))
        }),
        map_res(tagged_score("C"), |n| {
            n.parse().map(|d| ChangeScore::Copy(d))
        }),
    ))(input)
}

#[derive(Debug, PartialEq)]
enum StatusLine<'a> {
    One {
        status: StatusPair,
        sub: SubmoduleStatus,
        head_mode: Mode,
        index_mode: Mode,
        worktree_mode: Mode,
        head_obj: &'a str,
        index_obj: &'a str,
        path: OsString,
    },
    Two {
        status: StatusPair,
        sub: SubmoduleStatus,
        head_mode: Mode,
        index_mode: Mode,
        worktree_mode: Mode,
        head_obj: &'a str,
        index_obj: &'a str,
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
        stage1_obj: &'a str,
        stage2_obj: &'a str,
        stage3_obj: &'a str,
        path: OsString,
    },
    Untracked {
        path: OsString,
    },
    Ignored {
        path: OsString,
    },
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
            head_obj,
            index_obj,
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
            head_obj,
            index_obj,
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
            stage1_obj,
            stage2_obj,
            stage3_obj,
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

fn status_lines(input: &str) -> Result<Vec<StatusLine>, String> {
    match many0(terminated(status_line, tag("\n")))(input) {
        Ok(("", v)) => Ok(v),
        Ok((extra, _)) => Err(format!("Trailing: {}", extra)),
        Err(nom::Err::Error((rest, kind))) =>  Err(format!("{}: {}", kind.description(), rest)),
        Err(nom::Err::Failure((rest, kind))) =>  Err(format!("{}: {}", kind.description(), rest)),
        Err(nom::Err::Incomplete(nom::Needed::Size(x))) =>  Err(format!("Incomplete: needs {}", x)),
        Err(nom::Err::Incomplete(nom::Needed::Unknown)) =>  Err(format!("Incomplete, but don't know what's needed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom;

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
                    staged: Status::Unmodified,
                    unstaged: Status::Unmodified
                }
            ))
        );
        assert_eq!(
            status_pair("R."),
            Ok((
                "",
                StatusPair {
                    staged: Status::Renamed,
                    unstaged: Status::Unmodified
                }
            ))
        );
        assert_eq!(
            status_pair(".M"),
            Ok((
                "",
                StatusPair {
                    staged: Status::Unmodified,
                    unstaged: Status::Modified
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
        let lines = status_lines(include_str!("../testdata/mezzo-status-2")).unwrap();
        assert_eq!(lines[0],
                   StatusLine::Two{
                       status: StatusPair{staged: Status::Renamed, unstaged: Status::Unmodified},
                       sub: SubmoduleStatus::Not,
                       head_mode: Mode([1,0,0,6,4,4]),
                       index_mode: Mode([1,0,0,6,4,4]),
                       worktree_mode: Mode([1,0,0,6,4,4]),
                       head_obj: "11e1a9446255b2e9bb3eea5105e52967dbf9b1ea",
                       index_obj: "11e1a9446255b2e9bb3eea5105e52967dbf9b1ea",
                       change_score: ChangeScore::Rename(100),
                       path: OsString::from("README-2.md"),
                       orig_path: OsString::from("README.md")
                   }
                  );
        assert_eq!(lines[1],
                   StatusLine::One{
                       status: StatusPair{staged: Status::Unmodified, unstaged: Status::Modified},
                       sub: SubmoduleStatus::Not,
                       head_mode: Mode([1,0,0,6,4,4]),
                       index_mode: Mode([1,0,0,6,4,4]),
                       worktree_mode: Mode([1,0,0,6,4,4]),
                       head_obj: "c68d13474cd3f99964c052e5acc771f4df1e668e",
                       index_obj: "c68d13474cd3f99964c052e5acc771f4df1e668e",
                       path: OsString::from("spec/transitions/service_request_transitions/fulfill_spec.rb"),
                   }
                  );
    }
}
