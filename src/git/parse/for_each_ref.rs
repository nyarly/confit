use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    combinator::{map, map_res},
    multi::{many0, separated_nonempty_list},
    sequence::{delimited, terminated, tuple},
    //multi::many0,
    IResult,
};

use super::{is_digit, settle_parse_result, sha, ObjectName, TrackingCounts};
use chrono::{DateTime, Utc};

/*
 *
 * git for-each-ref --shell --format "%(objectname) %(objecttype) %(refname) %(upstream) %(upstream:remotename) %(upstream:track) %(creator)"
 * '8558b6934276f1b9966c01f7b3e5aeea2902742d' 'commit' 'refs/heads/multiple_provisioning' 'refs/remotes/origin/multiple_provisioning' 'origin' '[ahead 1]' 'Judson <nyarly@gmail.com> 1572973200 -0800'
 */

#[derive(Debug, PartialEq, Eq)]
pub struct RefLine {
    object_name: ObjectName,
    object_type: ObjectType,
    local_ref: String,
    upstream: TrackSync,
    creator_name: String,
    creator_email: String,
    creation_date: DateTime<Utc>,
}

// XXX review pulling this up to RefLine
#[derive(Debug, PartialEq, Eq)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RemoteRef {
    remote: String,
    refname: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TrackSync {
    Untracked,
    Track {
        remote_ref: RemoteRef,
        counts: TrackingCounts,
    },
    Gone {
        remote_ref: RemoteRef,
    },
}

impl From<(String, String, Option<(u64, u64)>)> for TrackSync {
    fn from(t: (String, String, Option<(u64, u64)>)) -> TrackSync {
        use TrackSync::*;

        let (remote, refname, ts) = t;
        match refname.as_ref() {
            "" => Untracked,
            _ => match ts {
                None => Gone {
                    remote_ref: RemoteRef { remote, refname },
                },
                Some((ahead, behind)) => Track {
                    counts: TrackingCounts(ahead, behind),
                    remote_ref: RemoteRef { remote, refname },
                },
            },
        }
    }
}

pub fn parse(input: &str) -> super::Result<&str, Vec<RefLine>> {
    settle_parse_result(many0(terminated(line, tag("\n")))(input))
}

// '8558b6934276f1b9966c01f7b3e5aeea2902742d' 'commit' 'refs/heads/multiple_provisioning' 'refs/remotes/origin/multiple_provisioning' 'origin' '[ahead 1]' 'Judson <nyarly@gmail.com> 1572973200 -0800'
fn line(input: &str) -> IResult<&str, RefLine> {
    let (
        rest,
        (
            _,
            object_name,
            _,
            ot,
            _,
            local_ref,
            _,
            refname,
            _,
            remote,
            _,
            ts,
            _,
            (creator_name, creator_email, creation_date),
            _,
        ),
    ) = tuple((
        tag("'"),       // '
        sha,            // 8558b6934276f1b9966c01f7b3e5aeea2902742d
        tag("' '"),     // ' '
        object_type,    // commit
        tag("' '"),     // ' '
        qstring,        // refs/heads/multiple_provisioning
        tag("' '"),     // ' '
        qstring,        // refs/remotes/origin/multiple_provisioning
        tag("' '"),     // ' '
        qstring,        // origin
        tag("' '"),     // ' '
        tracking_state, // [ahead 1]
        tag("' '"),     // ' '
        creator::parse, // Judson <nyarly@gmail.com> 1572973200 -0800
        tag("'"),       // '
    ))(input)?;

    Ok((
        rest,
        RefLine {
            object_name: object_name,
            object_type: ot,
            local_ref,
            upstream: (remote, refname, ts).into(),
            creator_name,
            creator_email,
            creation_date,
        },
    ))
}

fn qstring(input: &str) -> IResult<&str, String> {
    map(take_until("'"), String::from)(input)
}

fn object_type(input: &str) -> IResult<&str, ObjectType> {
    alt((
        map(tag("commit"), |_| ObjectType::Commit),
        map(tag("blob"), |_| ObjectType::Blob),
        map(tag("tree"), |_| ObjectType::Tree),
        map(tag("tag"), |_| ObjectType::Tag),
    ))(input)
}

fn tracking_state(input: &str) -> IResult<&str, Option<(u64, u64)>> {
    alt((
        map(tag("[gone]"), |_| None),
        map(delimited(tag("["), ahead_behind, tag("]")), |p| Some(p)),
        map(tag(""), |_| Some((0, 0))),
    ))(input)
}

fn ahead_behind(input: &str) -> IResult<&str, (u64, u64)> {
    map(
        separated_nonempty_list(tag(", "), alt((ahead, behind))),
        |list| {
            list.into_iter().fold((0, 0), |sum, pair| {
                let ((a, b), (c, d)) = (sum, pair);
                (a + c, b + d)
            })
        },
    )(input)
}

fn ahead(input: &str) -> IResult<&str, (u64, u64)> {
    let (rest, _) = tag("ahead ")(input)?;
    let (rest, n) = map_res(take_while(is_digit), |s: &str| s.parse())(rest)?;
    Ok((rest, (n, 0)))
}

fn behind(input: &str) -> IResult<&str, (u64, u64)> {
    let (rest, _) = tag("behind ")(input)?;
    let (rest, n) = map_res(take_while(is_digit), |s: &str| s.parse())(rest)?;
    Ok((rest, (0, n)))
}

mod creator {
    use chrono::{DateTime, FixedOffset, TimeZone, Utc};
    use nom::{
        branch::alt,
        bytes::complete::{tag, take_until, take_while, take_while_m_n},
        combinator::map_res,
        error::ErrorKind,
        IResult,
    };

    use std::str::FromStr;

    use super::is_digit;

    enum Sign {
        Pos,
        Neg,
    }

    impl FromStr for Sign {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            use Sign::*;

            match s {
                "+" => Ok(Pos),
                "-" => Ok(Neg),
                otherwise => Err(format!(
                    "sign can only be parsed from +,-, got {}",
                    otherwise
                )),
            }
        }
    }

    pub fn parse(input: &str) -> IResult<&str, (String, String, DateTime<Utc>)> {
        let (rest, name) = take_until(" <")(input)?;
        let (rest, _) = tag(" <")(rest)?;
        let (rest, email) = take_until("> ")(rest)?;
        let (rest, _) = tag("> ")(rest)?;
        let (rest, secs_epoch): (_, i64) =
            map_res(take_while(is_digit), |s: &str| s.parse())(rest)?;
        let (rest, _) = tag(" ")(rest)?;
        let (rest, sign) = map_res(alt((tag("+"), tag("-"))), |s: &str| s.parse())(rest)?;
        let (rest, hours): (_, i32) =
            map_res(take_while_m_n(2, 2, is_digit), |s: &str| s.parse())(rest)?;
        let (rest, minutes): (_, i32) =
            map_res(take_while_m_n(2, 2, is_digit), |s: &str| s.parse())(rest)?;

        let ts = build_timestamp((sign, hours, minutes), secs_epoch)
            .ok_or(nom::Err::Error((rest, ErrorKind::TakeWhileMN)))?;

        Ok((rest, (name.into(), email.into(), ts)))
    }

    fn build_timestamp(offset: (Sign, i32, i32), secs_epoch: i64) -> Option<DateTime<Utc>> {
        let (sign, hours, minutes) = offset;
        let offset_secs = (hours * 60 + minutes) * 60;

        let tz = match sign {
            Sign::Pos => FixedOffset::east(offset_secs),
            Sign::Neg => FixedOffset::west(offset_secs),
        };

        tz.timestamp_opt(secs_epoch, 0).earliest().map(|t| t.into())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn utc_time(from: &str) -> DateTime<Utc> {
            DateTime::<Utc>::from(
                DateTime::parse_from_rfc2822(format!("{} +0000", from).as_ref()).unwrap(),
            )
        }

        #[test]
        fn creator_parse() {
            assert_eq!(
                parse("Judson <nyarly@gmail.com> 1570644797 -0700"),
                Ok((
                    "",
                    (
                        "Judson".into(),
                        "nyarly@gmail.com".into(),
                        utc_time("Wed, 9 Oct 2019 18:13:17")
                    )
                ))
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc_time(from: &str) -> DateTime<Utc> {
        DateTime::<Utc>::from(
            DateTime::parse_from_rfc2822(format!("{} +0000", from).as_ref()).unwrap(),
        )
    }

    #[test]
    fn smoke() {
        parse(include_str!("testdata/mezzo-for-each-ref-formatted")).unwrap();
    }

    #[test]
    fn line_parse() {
        assert_eq!(
            line("'f8f49343edaa2a1e6903cbad13ddbc50ad9e12d2' 'commit' 'refs/heads/along' 'refs/remotes/along/mezzo' 'along' '' 'Judson <nyarly@gmail.com> 1570644797 -0700'"),
            Ok(("", RefLine{
                local_ref: "refs/heads/along".into(),
                object_name: "f8f49343edaa2a1e6903cbad13ddbc50ad9e12d2".into(),
                object_type: ObjectType::Commit,
                upstream: TrackSync::Track{
                    counts: TrackingCounts(0,0),
                    remote_ref: RemoteRef{
                        remote: "along".into(),
                        refname: "refs/remotes/along/mezzo".into(),
                    },
                },
                creator_name: "Judson".into(),
                creator_email: "nyarly@gmail.com".into(),
                creation_date: utc_time("Wed, 9 Oct 2019 18:13:17"),

            }))
        )
    }

    #[test]
    fn object_type_parse() {
        use super::ObjectType::*;

        assert_eq!(object_type("commit"), Ok(("", Commit)));
        assert_eq!(object_type("blob"), Ok(("", Blob)));
        assert_eq!(object_type("tree"), Ok(("", Tree)));
        assert_eq!(object_type("tag"), Ok(("", Tag)));
    }

    #[test]
    fn track_sync_parse() {
        assert_eq!(tracking_state(""), Ok(("", Some((0, 0)))));
        assert_eq!(tracking_state("[gone]"), Ok(("", None)));
        assert_eq!(tracking_state("[ahead 7]"), Ok(("", Some((7, 0)))));
        assert_eq!(tracking_state("[behind 9]"), Ok(("", Some((0, 9)))));
        assert_eq!(
            tracking_state("[ahead 13, behind 15]"),
            Ok(("", Some((13, 15))))
        );
    }
}
