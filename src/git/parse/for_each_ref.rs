use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while, take_while_m_n},
    combinator::{map, map_res},
    sequence::delimited,
    multi::separated_nonempty_list,
    error::ErrorKind,
    //multi::many0,
    //sequence::{terminated, tuple},
    IResult,
};

use super::is_digit;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};

use std::str::FromStr;

/*
 * git for-each-ref --shell --format "%(objectname) %(objecttype) %(refname) %(upstream) %(upstream:remotename) %(upstream:track) %(creator)"
 */

#[derive(Debug, PartialEq, Eq)]
struct EachRefName {
    object_name: String,
    object_type: ObjectType,
    local_ref: String,
    upstream: TrackSync,
    creator_name: String,
    creator_email: String,
    creation_date: String,
}

#[derive(Debug, PartialEq, Eq)]
enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag,
}

#[derive(Debug, PartialEq, Eq)]
struct RemoteRef {
    remote: String,
    refname: String,
}

#[derive(Debug, PartialEq, Eq)]
enum TrackSync {
    Untracked,
    Track {
        remote_ref: RemoteRef,
        ahead: u64,
        behind: u64,
    },
    Gone {
        remote_ref: RemoteRef,
    },
}

fn object_type(input: &str) -> IResult<&str, ObjectType> {
    alt((
        map(tag("commit"), |_| ObjectType::Commit),
        map(tag("blob"), |_| ObjectType::Blob),
        map(tag("tree"), |_| ObjectType::Tree),
        map(tag("tag"), |_| ObjectType::Tag),
    ))(input)
}

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

fn build_timestamp(offset: (Sign, i32, i32), secs_epoch: i64) -> Option<DateTime<Utc>> {
    let (sign, hours, minutes) = offset;
    let offset_secs = (hours * 60 + minutes) * 60;

    let tz = match sign {
        Sign::Pos => FixedOffset::east(offset_secs),
        Sign::Neg => FixedOffset::west(offset_secs),
    };

    tz.timestamp_opt(secs_epoch, 0).earliest().map(|t| t.into())
}

fn creator(input: &str) -> IResult<&str, (&str, &str, DateTime<Utc>)> {
    let (rest, name) = take_until(" <")(input)?;
    let (rest, _) = tag(" <")(rest)?;
    let (rest, email) = take_until("> ")(rest)?;
    let (rest, _) = tag("> ")(rest)?;
    let (rest, secs_epoch): (_, i64) = map_res(take_while(is_digit), |s: &str| s.parse())(rest)?;
    let (rest, _) = tag(" ")(rest)?;
    let (rest, sign) = map_res(alt((tag("+"), tag("-"))), |s: &str| s.parse())(rest)?;
    let (rest, hours): (_, i32) =
        map_res(take_while_m_n(2, 2, is_digit), |s: &str| s.parse())(rest)?;
    let (rest, minutes): (_, i32) =
        map_res(take_while_m_n(2, 2, is_digit), |s: &str| s.parse())(rest)?;

    let ts = build_timestamp((sign, hours, minutes), secs_epoch)
        .ok_or(nom::Err::Error((rest, ErrorKind::TakeWhileMN)))?;

    Ok((rest, (name, email, ts)))
}

fn tracking_state(input: &str) -> IResult<&str, Option<(u64, u64)>> {
    alt((
            map(tag("[gone]"), |_| None),
            map(delimited(tag("["), ahead_behind, tag("]")), |p| Some(p)),
            map(tag(""), |_| Some((0,0))),
    ))(input)
}

fn ahead_behind(input: &str) -> IResult<&str, (u64, u64)> {
    map(separated_nonempty_list(tag(", "), alt((ahead, behind))), |list|
       list.into_iter().fold((0,0), |sum, pair| {
                             let ((a,b),(c,d)) = (sum, pair); (a+c,b+d)
                         }))(input)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn object_type_parse() {
        use super::ObjectType::*;

        assert_eq!(object_type("commit"), Ok(("", Commit)));
        assert_eq!(object_type("blob"), Ok(("", Blob)));
        assert_eq!(object_type("tree"), Ok(("", Tree)));
        assert_eq!(object_type("tag"), Ok(("", Tag)));
    }

    #[test]
    fn creator_parse() {
        assert_eq!(
            creator("Judson <nyarly@gmail.com> 1570644797 -0700"),
            Ok((
                "",
                (
                    "Judson",
                    "nyarly@gmail.com",
                    DateTime::<Utc>::from(
                        DateTime::parse_from_rfc2822("Wed, 9 Oct 2019 18:13:17 +0000").unwrap()
                    )
                )
            ))
        )
    }

    #[test]
    fn track_sync_parse() {
        assert_eq!(
            tracking_state(""),
            Ok(("", Some((0,0))))
        );
        assert_eq!(
            tracking_state("[gone]"),
            Ok(("", None))
        );
        assert_eq!(
            tracking_state("[ahead 7]"),
            Ok(("", Some((7,0))))
        );
        assert_eq!(
            tracking_state("[behind 9]"),
            Ok(("", Some((0,9))))
        );
        assert_eq!(
            tracking_state("[ahead 13, behind 15]"),
            Ok(("", Some((13,15))))
        );
    }
}

/*
'c3327de22fc2bee6f25988f727700a9932b520dc' 'commit' 'refs/heads/ad_objects' 'refs/remotes/origin/ad_objects' 'origin' '' 'Paula Burke <pburke@opentable.com> 1534292967 -0700'
'f8f49343edaa2a1e6903cbad13ddbc50ad9e12d2' 'commit' 'refs/heads/along' 'refs/remotes/along/mezzo' 'along' '' 'Judson <nyarly@gmail.com> 1570644797 -0700'
'd9d322887ef9af2a3d6ba687e3606449439a8fae' 'commit' 'refs/heads/approval_change_requests' 'refs/remotes/origin/approval_change_requests' 'origin' '' 'Paula Burke <pburke@opentable.com> 1533938779 -0700'
'580cab6ba249b89cba915b38871e64d80577d088' 'commit' 'refs/heads/approvals' 'refs/remotes/origin/approvals' 'origin' '[behind 17]' 'Tom Hsieh <thsieh@opentable.com> 1571859687 -0700'
'6b0fe0089db920eaebe6b8d46a3deb94091e00dd' 'commit' 'refs/heads/backport_from_prod' 'refs/remotes/origin/backport_from_prod' 'origin' '' 'Judson <nyarly@gmail.com> 1572372469 -0700'
'ff887d7fb22ae687fa0e594b6fe6ca72e4e23839' 'commit' 'refs/heads/consist_styling' '' '' '' 'GitHub <noreply@github.com> 1548269418 -0800'
'61621e0ea141428e11e880d6d71020bead67fdbe' 'commit' 'refs/heads/debug_fsm' 'refs/remotes/origin/debug_fsm' 'origin' '' 'Judson <nyarly@gmail.com> 1537836632 -0700'
'64cd3bb41507b0f0657c0ac0857ba1e682ec4f8d' 'commit' 'refs/heads/delegation_autocomplete' '' '' '' 'Judson <nyarly@gmail.com> 1544850541 -0800'
'9e8e637f04c555c55f2dca4025814d0205c60143' 'commit' 'refs/heads/drive_fsm_with_events' 'refs/remotes/origin/drive_fsm_with_events' 'origin' '' 'Judson <nyarly@gmail.com> 1537576144 -0700'
'1924272055a5ce9e2fadfe3d8b9057cbebf70c5c' 'commit' 'refs/heads/employee_query' 'refs/remotes/origin/employee_query' 'origin' '' 'Judson <nyarly@gmail.com> 1533339340 -0700'
'e927ee55913d97f85daa8787f226aa90f856b723' 'commit' 'refs/heads/extract_fsms' 'refs/remotes/origin/extract_fsms' 'origin' '' 'Judson <nyarly@gmail.com> 1533169249 -0700'
'10737cc0e64617d2b03ee30004e730045550e5d5' 'commit' 'refs/heads/fix_ad_errors' 'refs/remotes/origin/fix_ad_errors' 'origin' '' 'Paula Burke <pburke@opentable.com> 1538611873 -0700'
'f00d342419cfe874b7767fd5d703a6590760d427' 'commit' 'refs/heads/fixes_approval_views' 'refs/remotes/origin/fixes_approval_views' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1558475178 -0700'
'd187325e25fa480514518c89aaefb7609fdb3278' 'commit' 'refs/heads/future_prod' 'refs/remotes/origin/future_prod' 'origin' '' 'Judson <nyarly@gmail.com> 1565915317 -0700'
'6b220da736631f4b24fa6f039fd36e63c5f21414' 'commit' 'refs/heads/future_staging' 'refs/remotes/origin/future_staging' 'origin' '[gone]' 'Judson <nyarly@gmail.com> 1561673704 -0700'
'9fde69a22417326e316b9e8df73ecf3e436b6a72' 'commit' 'refs/heads/kayak-services' 'refs/remotes/origin/kayak-services' 'origin' '[behind 9]' 'Tom Hsieh <thsieh@opentable.com> 1572309704 -0700'
'46fe9a8609ea9d28221eb83021a1aaa1ab51716b' 'commit' 'refs/heads/ldap_tools' 'refs/remotes/origin/ldap_tools' 'origin' '' 'Judson <nyarly@gmail.com> 1548700777 -0800'
'5ea12e81a91c19a6c8a59ea470b9bac5649eb159' 'commit' 'refs/heads/loglov3-proper-statuses' 'refs/remotes/origin/loglov3-proper-statuses' 'origin' '' 'Judson <nyarly@gmail.com> 1541030553 -0700'
'3f0a499bf4be2472176481b9724ac2437568c473' 'commit' 'refs/heads/master' 'refs/remotes/origin/master' 'origin' '[behind 20]' 'GitHub <noreply@github.com> 1572654995 -0700'
'2c101a76d9724a3718441664a5554fbb930605cc' 'commit' 'refs/heads/merge_records' 'refs/remotes/origin/merge_records' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1551237497 -0800'
'7f980f38660177e959435e155a6fb67243a60cef' 'commit' 'refs/heads/mockable_concur' 'refs/remotes/origin/mockable_concur' 'origin' '' 'Judson <nyarly@gmail.com> 1546307484 -0800'
'6a62579282502563f6b1579d060731433065a7c5' 'commit' 'refs/heads/move_service_requests' 'refs/remotes/origin/move_service_requests' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1559082921 -0700'
'8558b6934276f1b9966c01f7b3e5aeea2902742d' 'commit' 'refs/heads/multiple_provisioning' 'refs/remotes/origin/multiple_provisioning' 'origin' '[ahead 1]' 'Judson <nyarly@gmail.com> 1572973200 -0800'
'b7522a655cfac262e8557920613ccda3759710f6' 'commit' 'refs/heads/netops_2027' 'refs/remotes/origin/netops_2027' 'origin' '' 'Paula Burke <pburke@opentable.com> 1533582471 -0700'
'd73bcac95025bf396934e6d68ca8c80edbb2fad0' 'commit' 'refs/heads/netops_922' 'refs/remotes/origin/netops_922' 'origin' '' 'Paula Burke <pburke@opentable.com> 1513892065 -0800'
'e279f5b6579d1d0cab2a73447d3fc24c04fcb4e1' 'commit' 'refs/heads/nixsupport' 'refs/remotes/origin/nixsupport' 'origin' '' 'Judson <nyarly@gmail.com> 1531517763 -0700'
'fd516d1c93c6154e0e7b4e11d541d308b121233d' 'commit' 'refs/heads/offboard_guest_account' 'refs/remotes/origin/offboard_guest_account' 'origin' '' 'Judson <nyarly@gmail.com> 1570838435 -0700'
'0acb86a50973e3ba9f276eb50cdc0d9b1e4e65e5' 'commit' 'refs/heads/onboard_with_services' 'refs/remotes/origin/onboard_with_services' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1564519055 -0700'
'0e716e765f1285853b88742dea29e9561536434b' 'commit' 'refs/heads/pagination' 'refs/remotes/origin/pagination' 'origin' '[behind 9]' 'GitHub <noreply@github.com> 1572036839 -0700'
'8ead1272f6c422014769625039d31158f85ec827' 'commit' 'refs/heads/pb_ref_4' 'refs/remotes/origin/pb_ref_4' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1556141616 -0700'
'06e3cae8ecf583464b4d7288a99a58421986b553' 'commit' 'refs/heads/production' 'refs/remotes/origin/production' 'origin' '' 'GitHub <noreply@github.com> 1572642497 -0700'
'4089c1c2b47da97f18fb33ab55768062f26a20aa' 'commit' 'refs/heads/ref_4' 'refs/remotes/origin/ref_4' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1555977757 -0700'
'48718c3b31b25f62a75e721960fd513b9e499890' 'commit' 'refs/heads/refactor_20181117' 'refs/remotes/origin/refactor_20181117' 'origin' '' 'Paula Burke <pburke@opentable.com> 1543541822 -0800'
'1289262bcccdf1f235f453073f6bba62c5f27a15' 'commit' 'refs/heads/refactor_ad_module' 'refs/remotes/origin/refactor_ad_module' 'origin' '' 'Paula Burke <pburke@opentable.com> 1525467667 -0700'
'f35ff092509e53f1395030ac21b163a87974e59f' 'commit' 'refs/heads/refactor_ad_pr_1' 'refs/remotes/origin/refactor_ad_pr_1' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1554504842 -0700'
'df5aac1252d62afae8707678bb419bd52c36d4bc' 'commit' 'refs/heads/refactor_ad_svc' 'refs/remotes/origin/refactor_ad_svc' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1553806228 -0700'
'c3eb3b29dfb5c9a47edc8b451aa123f9195f93a9' 'commit' 'refs/heads/remove_dn_mapping' 'refs/remotes/origin/remove_dn_mapping' 'origin' '' 'Judson <nyarly@gmail.com> 1561679011 -0700'
'70755e81cfd8361bbde487279770508b3c9f37d8' 'commit' 'refs/heads/reverse_lookup_ldaps' 'refs/remotes/origin/reverse_lookup_ldaps' 'origin' '' 'Judson <nyarly@gmail.com> 1560207167 -0700'
'466824389e7511ac6a974642a9fca05619a6a1b1' 'commit' 'refs/heads/service_catalog_spike' 'refs/remotes/origin/service_catalog_spike' 'origin' '' 'Paula Burke <pburke@opentable.com> 1522177029 -0700'
'ad26a66a274ee367513c600d09df6d1ca140a1cb' 'commit' 'refs/heads/single_out_ad_service' 'refs/remotes/origin/single_out_ad_service' 'origin' '' 'Judson <nyarly@gmail.com> 1546039109 -0800'
'ac841cd7f979a0d9fe46baedfb60084493ab9e99' 'commit' 'refs/heads/staging' 'refs/remotes/origin/staging' 'origin' '' 'GitHub <noreply@github.com> 1571862274 -0700'
'4569d8bbe6e0ecf9218a6c57854d67e46db009a0' 'commit' 'refs/heads/staging-fixes' 'refs/remotes/origin/staging-fixes' 'origin' '' 'Judson <nyarly@gmail.com> 1567801556 -0700'
'70ade64d5684e6a0ebd85ed80ce56eccda34842d' 'commit' 'refs/heads/staging-future-is-now' 'refs/remotes/origin/staging-future-is-now' 'origin' '[gone]' 'Judson <nyarly@gmail.com> 1561679569 -0700'
'0a05d5fdcaec86fb775135dace3e9cadbe1c66d3' 'commit' 'refs/heads/staging_idempotency' 'refs/remotes/origin/staging_idempotency' 'origin' '' 'Judson <nyarly@gmail.com> 1567618544 -0700'
'abe73f07d4179cbb07e3721c25b8eab46662c430' 'commit' 'refs/heads/test_parser' 'refs/remotes/origin/test_parser' 'origin' '' 'Paula Burke <pburke@opentable.com> 1544842842 -0800'
'b32658d14457967cdb8c6328ec82e8e1eafa5c15' 'commit' 'refs/heads/transition_onboard_form' 'refs/remotes/origin/transition_onboard_form' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1559092235 -0700'
'd57d1264d754d5caed124d76bd5d9c9028155436' 'commit' 'refs/heads/tz_mapping' 'refs/remotes/origin/tz_mapping' 'origin' '' 'Judson <nyarly@gmail.com> 1546479461 -0800'
'e5634b941c18a9222f010fdaf8e582893c182b2f' 'commit' 'refs/heads/update_service_backend' 'refs/remotes/origin/update_service_backend' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1558040139 -0700'
'31bff71e9644597365836129ede51375d5738f2e' 'commit' 'refs/heads/update_workday_code_lists' 'refs/remotes/origin/update_workday_code_lists' 'origin' '' 'Paula Burke <pburke@opentable.com> 1545942821 -0800'
'edb7f443baac81e4c86f1a3a8a6ff08534528135' 'commit' 'refs/heads/view_specs' 'refs/remotes/origin/view_specs' 'origin' '' 'Paula Burke <pburke@opentable.com> 1541029660 -0700'
'1cc9931c784e106fcbf6773960dcae466fcdf800' 'commit' 'refs/heads/workday_business_process_spike' 'refs/remotes/origin/workday_business_process_spike' 'origin' '' 'Paula Burke <pburke@opentable.com> 1546029157 -0800'
'28bdceef9e4f264461e86af2675d364c2996570b' 'commit' 'refs/heads/workday_termination_report_sync' 'refs/remotes/origin/workday_termination_report_sync' 'origin' '' 'Paula Burke <pburke@opentable.com> 1548371261 -0800'
'306ec64d279c2b7e439a6f29b6a6a4a1e14f2a89' 'commit' 'refs/heads/workday_update_termination' 'refs/remotes/origin/workday_update_termination' 'origin' '' 'Paula Burke <paula.burke@gmail.com> 1550112329 -0800'
'27da5b4dcbe9011846d1aa21d1700c2e99fd08bd' 'commit' 'refs/remotes/along/critical_errors' '' '' '' 'Judson <nyarly@gmail.com> 1556896914 -0700'
'bdf49029a6c9d023a1b074530b11a12da0f15b92' 'commit' 'refs/remotes/along/jdl-nix-fpcli' '' '' '' 'Judson <nyarly@gmail.com> 1540933311 -0700'
'2c81f09752078a09ad4c24682ff9287a8b093294' 'commit' 'refs/remotes/along/jdl-nix-mezzo' '' '' '' 'Judson <nyarly@gmail.com> 1541022603 -0700'
'ab800fde5474ec8bde772fb262545957dce13ea7' 'commit' 'refs/remotes/along/jdl-nix-puppet-modules' '' '' '' 'Judson <nyarly@gmail.com> 1540943561 -0700'
'b0ef6791a4a53236c656a44098232cceb583b7ab' 'commit' 'refs/remotes/along/master' '' '' '' 'Judson <nyarly@gmail.com> 1540923151 -0700'
'f8f49343edaa2a1e6903cbad13ddbc50ad9e12d2' 'commit' 'refs/remotes/along/mezzo' '' '' '' 'Judson <nyarly@gmail.com> 1570644797 -0700'
'7c0e8de6d607886e3bc59ea9ca3d3349387fabf1' 'commit' 'refs/remotes/origin/HEAD' '' '' '' 'GitHub <noreply@github.com> 1572915376 -0800'
'4bc80bdbecb317c8177f507c7e065c4a95925f5e' 'commit' 'refs/remotes/origin/access-mgmt-routes' '' '' '' 'GitHub <noreply@github.com> 1572915481 -0800'
'c3327de22fc2bee6f25988f727700a9932b520dc' 'commit' 'refs/remotes/origin/ad_objects' '' '' '' 'Paula Burke <pburke@opentable.com> 1534292967 -0700'
'd9d322887ef9af2a3d6ba687e3606449439a8fae' 'commit' 'refs/remotes/origin/approval_change_requests' '' '' '' 'Paula Burke <pburke@opentable.com> 1533938779 -0700'
'b20f8eff3652fbbd4317c71ca16dc917a5f25b8b' 'commit' 'refs/remotes/origin/approvals' '' '' '' 'GitHub <noreply@github.com> 1572389142 -0700'
'd4ae7077d4ed711a10e89908ab91999ce326dfc0' 'commit' 'refs/remotes/origin/approvals_template' '' '' '' 'GitHub <noreply@github.com> 1572903504 -0800'
'6b0fe0089db920eaebe6b8d46a3deb94091e00dd' 'commit' 'refs/remotes/origin/backport_from_prod' '' '' '' 'Judson <nyarly@gmail.com> 1572372469 -0700'
'61621e0ea141428e11e880d6d71020bead67fdbe' 'commit' 'refs/remotes/origin/debug_fsm' '' '' '' 'Judson <nyarly@gmail.com> 1537836632 -0700'
'a5c38ce6448522229bae0cba904a233457ad214f' 'commit' 'refs/remotes/origin/delegate' '' '' '' 'GitHub <noreply@github.com> 1572655021 -0700'
'2d2214ee51623b8e73c37568b76fd91bdf235ba5' 'commit' 'refs/remotes/origin/deprecate' '' '' '' 'GitHub <noreply@github.com> 1572904540 -0800'
'9e8e637f04c555c55f2dca4025814d0205c60143' 'commit' 'refs/remotes/origin/drive_fsm_with_events' '' '' '' 'Judson <nyarly@gmail.com> 1537576144 -0700'
'1924272055a5ce9e2fadfe3d8b9057cbebf70c5c' 'commit' 'refs/remotes/origin/employee_query' '' '' '' 'Judson <nyarly@gmail.com> 1533339340 -0700'
'e927ee55913d97f85daa8787f226aa90f856b723' 'commit' 'refs/remotes/origin/extract_fsms' '' '' '' 'Judson <nyarly@gmail.com> 1533169249 -0700'
'10737cc0e64617d2b03ee30004e730045550e5d5' 'commit' 'refs/remotes/origin/fix_ad_errors' '' '' '' 'Paula Burke <pburke@opentable.com> 1538611873 -0700'
'f00d342419cfe874b7767fd5d703a6590760d427' 'commit' 'refs/remotes/origin/fixes_approval_views' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1558475178 -0700'
'd187325e25fa480514518c89aaefb7609fdb3278' 'commit' 'refs/remotes/origin/future_prod' '' '' '' 'Judson <nyarly@gmail.com> 1565915317 -0700'
'bcb3481567a1842169dfcf7d6f2c59aa76f4680a' 'commit' 'refs/remotes/origin/kayak-services' '' '' '' 'GitHub <noreply@github.com> 1572389064 -0700'
'46fe9a8609ea9d28221eb83021a1aaa1ab51716b' 'commit' 'refs/remotes/origin/ldap_tools' '' '' '' 'Judson <nyarly@gmail.com> 1548700777 -0800'
'5ea12e81a91c19a6c8a59ea470b9bac5649eb159' 'commit' 'refs/remotes/origin/loglov3-proper-statuses' '' '' '' 'Judson <nyarly@gmail.com> 1541030553 -0700'
'7c0e8de6d607886e3bc59ea9ca3d3349387fabf1' 'commit' 'refs/remotes/origin/master' '' '' '' 'GitHub <noreply@github.com> 1572915376 -0800'
'2c101a76d9724a3718441664a5554fbb930605cc' 'commit' 'refs/remotes/origin/merge_records' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1551237497 -0800'
'7f980f38660177e959435e155a6fb67243a60cef' 'commit' 'refs/remotes/origin/mockable_concur' '' '' '' 'Judson <nyarly@gmail.com> 1546307484 -0800'
'6a62579282502563f6b1579d060731433065a7c5' 'commit' 'refs/remotes/origin/move_service_requests' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1559082921 -0700'
'65abca85738f2e62e949cb3a9270022805f83339' 'commit' 'refs/remotes/origin/multiple_provisioning' '' '' '' 'Judson <nyarly@gmail.com> 1572566478 -0700'
'b7522a655cfac262e8557920613ccda3759710f6' 'commit' 'refs/remotes/origin/netops_2027' '' '' '' 'Paula Burke <pburke@opentable.com> 1533582471 -0700'
'd73bcac95025bf396934e6d68ca8c80edbb2fad0' 'commit' 'refs/remotes/origin/netops_922' '' '' '' 'Paula Burke <pburke@opentable.com> 1513892065 -0800'
'e279f5b6579d1d0cab2a73447d3fc24c04fcb4e1' 'commit' 'refs/remotes/origin/nixsupport' '' '' '' 'Judson <nyarly@gmail.com> 1531517763 -0700'
'fd516d1c93c6154e0e7b4e11d541d308b121233d' 'commit' 'refs/remotes/origin/offboard_guest_account' '' '' '' 'Judson <nyarly@gmail.com> 1570838435 -0700'
'0acb86a50973e3ba9f276eb50cdc0d9b1e4e65e5' 'commit' 'refs/remotes/origin/onboard_with_services' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1564519055 -0700'
'8c3ba735bf74ed09c347bfd460fc4f481d1a8af4' 'commit' 'refs/remotes/origin/onboarding_events' '' '' '' 'GitHub <noreply@github.com> 1572915524 -0800'
'1fb47b577f447eb91d6b88e4f0626d8a84ed8487' 'commit' 'refs/remotes/origin/pagination' '' '' '' 'GitHub <noreply@github.com> 1572389104 -0700'
'8ead1272f6c422014769625039d31158f85ec827' 'commit' 'refs/remotes/origin/pb_ref_4' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1556141616 -0700'
'06e3cae8ecf583464b4d7288a99a58421986b553' 'commit' 'refs/remotes/origin/production' '' '' '' 'GitHub <noreply@github.com> 1572642497 -0700'
'4089c1c2b47da97f18fb33ab55768062f26a20aa' 'commit' 'refs/remotes/origin/ref_4' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1555977757 -0700'
'48718c3b31b25f62a75e721960fd513b9e499890' 'commit' 'refs/remotes/origin/refactor_20181117' '' '' '' 'Paula Burke <pburke@opentable.com> 1543541822 -0800'
'1289262bcccdf1f235f453073f6bba62c5f27a15' 'commit' 'refs/remotes/origin/refactor_ad_module' '' '' '' 'Paula Burke <pburke@opentable.com> 1525467667 -0700'
'f35ff092509e53f1395030ac21b163a87974e59f' 'commit' 'refs/remotes/origin/refactor_ad_pr_1' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1554504842 -0700'
'df5aac1252d62afae8707678bb419bd52c36d4bc' 'commit' 'refs/remotes/origin/refactor_ad_svc' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1553806228 -0700'
'c3eb3b29dfb5c9a47edc8b451aa123f9195f93a9' 'commit' 'refs/remotes/origin/remove_dn_mapping' '' '' '' 'Judson <nyarly@gmail.com> 1561679011 -0700'
'70755e81cfd8361bbde487279770508b3c9f37d8' 'commit' 'refs/remotes/origin/reverse_lookup_ldaps' '' '' '' 'Judson <nyarly@gmail.com> 1560207167 -0700'
'7ebfa56f0a6f69aff42ec2f688a58b5cf322aa48' 'commit' 'refs/remotes/origin/service-request-bug' '' '' '' 'Tom Hsieh <thsieh@opentable.com> 1572633499 -0700'
'466824389e7511ac6a974642a9fca05619a6a1b1' 'commit' 'refs/remotes/origin/service_catalog_spike' '' '' '' 'Paula Burke <pburke@opentable.com> 1522177029 -0700'
'ad26a66a274ee367513c600d09df6d1ca140a1cb' 'commit' 'refs/remotes/origin/single_out_ad_service' '' '' '' 'Judson <nyarly@gmail.com> 1546039109 -0800'
'ac841cd7f979a0d9fe46baedfb60084493ab9e99' 'commit' 'refs/remotes/origin/staging' '' '' '' 'GitHub <noreply@github.com> 1571862274 -0700'
'4569d8bbe6e0ecf9218a6c57854d67e46db009a0' 'commit' 'refs/remotes/origin/staging-fixes' '' '' '' 'Judson <nyarly@gmail.com> 1567801556 -0700'
'0a05d5fdcaec86fb775135dace3e9cadbe1c66d3' 'commit' 'refs/remotes/origin/staging_idempotency' '' '' '' 'Judson <nyarly@gmail.com> 1567618544 -0700'
'abe73f07d4179cbb07e3721c25b8eab46662c430' 'commit' 'refs/remotes/origin/test_parser' '' '' '' 'Paula Burke <pburke@opentable.com> 1544842842 -0800'
'b32658d14457967cdb8c6328ec82e8e1eafa5c15' 'commit' 'refs/remotes/origin/transition_onboard_form' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1559092235 -0700'
'd57d1264d754d5caed124d76bd5d9c9028155436' 'commit' 'refs/remotes/origin/tz_mapping' '' '' '' 'Judson <nyarly@gmail.com> 1546479461 -0800'
'e5634b941c18a9222f010fdaf8e582893c182b2f' 'commit' 'refs/remotes/origin/update_service_backend' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1558040139 -0700'
'31bff71e9644597365836129ede51375d5738f2e' 'commit' 'refs/remotes/origin/update_workday_code_lists' '' '' '' 'Paula Burke <pburke@opentable.com> 1545942821 -0800'
'edb7f443baac81e4c86f1a3a8a6ff08534528135' 'commit' 'refs/remotes/origin/view_specs' '' '' '' 'Paula Burke <pburke@opentable.com> 1541029660 -0700'
'1cc9931c784e106fcbf6773960dcae466fcdf800' 'commit' 'refs/remotes/origin/workday_business_process_spike' '' '' '' 'Paula Burke <pburke@opentable.com> 1546029157 -0800'
'28bdceef9e4f264461e86af2675d364c2996570b' 'commit' 'refs/remotes/origin/workday_termination_report_sync' '' '' '' 'Paula Burke <pburke@opentable.com> 1548371261 -0800'
'306ec64d279c2b7e439a6f29b6a6a4a1e14f2a89' 'commit' 'refs/remotes/origin/workday_update_termination' '' '' '' 'Paula Burke <paula.burke@gmail.com> 1550112329 -0800'
'de4280e681a2a5989e17f223054d13858aef2861' 'tag' 'refs/tags/v2.0' '' '' '' 'Judson <nyarly@gmail.com> 1572973897 -0800'

 */
