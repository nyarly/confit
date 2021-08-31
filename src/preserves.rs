use crate::git;
use std::fmt;

use git::parse::for_each_ref::ObjectType::*;
use git::parse::status::{Head, LineStatus, Oid, StatusLine::*, StatusPair};
use git::parse::{ObjectName, TrackingCounts};
use serde::Serialize;
use datasource::{STATUS, REFS, REMOTE, union};

pub mod datasource {
  use serde::Serialize;

  #[derive(Clone, Copy, Serialize)]
  pub struct Group(u16);

  impl Group {
    pub fn includes(self, item: Group) -> bool {
      (self.0 & item.0) != 0
    }
  }

  impl std::fmt::Debug for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
      write!(f, "Group({:#x})", self.0)
    }
  }

  use std::ops::BitOr;
  impl BitOr for Group {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
      Group(self.0 | rhs.0)
    }
  }

  pub const EMPTY: Group = Group(0);

  pub const STATUS: Group = Group(1);
  pub const REFS: Group = Group(1 << 1);
  pub const REMOTE: Group = Group(1 << 2);

  pub const fn union(l: Group, r: Group) -> Group {
    Group(l.0 | r.0)
  }

}

pub struct Summary<'a> {
  pub status: git::Status,
  pub(crate) ls_remote: Vec<git::RefPair>,
  pub(crate) for_each_ref: Vec<git::RefLine>,
  checks: Vec<&'a Check>,
}


#[derive(Serialize)]
pub struct Check {
  label: &'static str,
  tags: &'static [&'static str],
  glyph: char,
  status_group: u8,
  required_data: datasource::Group,
  threshold: u16,
  #[serde(skip)]
  eval: fn(&Summary) -> CheckResult,
}

#[derive(Clone,Copy,Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckResult {
  // check passed
  Passed,
  // some checks simply fail
  Failed,
  // some checks have a count of failures
  Bad(usize)
}

impl From<usize> for CheckResult {
  fn from(n: usize) -> Self {
    if n == 0 {
      CheckResult::Passed
    } else {
      CheckResult::Bad(n)
    }
  }
}

impl From<u64> for CheckResult {
  fn from(n: u64) -> Self {
    CheckResult::from(n as usize)
  }
}

impl From<bool> for CheckResult {
  fn from(x: bool) -> Self {
    if x {
      CheckResult::Passed
    } else {
      CheckResult::Failed
    }
  }
}

impl Check {
  pub fn all_checks<'a>() -> Vec<&'a Check> {
    ALL_CHECKS.iter().collect()
  }

  pub fn tagged_checks<'a, 'b>(tags: impl Clone + IntoIterator<Item=&'b str>) -> Vec<&'a Check> {
    ALL_CHECKS.iter().filter(move |ch| tags.clone().into_iter()
        .any(|t|  ch.tags.iter().any(|&c| (t == c) ))).collect()
  }

  pub fn all_tags() -> Vec<&'static str> {
    let mut tags = ALL_CHECKS.iter().flat_map(|ch| ch.tags.iter().copied()).collect::<Vec<_>>();
    tags.sort_unstable();
    tags.dedup();
    tags
  }
}

pub trait CheckList {
  fn required_sources(&mut self) -> datasource::Group;
}

impl CheckList for Vec<&Check> {
  fn required_sources(&mut self) -> datasource::Group {
    self.iter().fold(datasource::EMPTY, |acc, check| acc | check.required_data)
  }
}

/// Collects and reports reasons that your current workspace
/// could not be reproduced on another workstation, in another place or time.
impl<'a> Summary<'a> {
  pub fn new(
    ls_remote: Vec<git::RefPair>,
    status: git::Status,
    for_each_ref: Vec<git::RefLine>,
    checks: Vec<&'a Check>,
  ) -> Self {
    Summary {
      status,
      ls_remote,
      for_each_ref,
      checks,
    }
  }

  pub fn items(&self) -> Vec<Item> {
    self.checks.iter().map(|ch| Item::build(ch, self)).collect()
  }

  pub fn exit_status(&self) -> i32 {
    self.items()
      .iter()
      .fold(0, |status, item| {
        if item.passed {
          status
        } else {
          status | (1 << item.check.status_group)
        }
      })
  }

  fn tag_on_commit(&self, c: ObjectName) -> Option<ObjectName> {
    self.for_each_ref
      .iter()
      .find(|rl| {
        rl.object_type == Tag
          && rl
          .referred_object
          .clone()
          .map(|ro| ro == c)
          .unwrap_or(false)
      })
    .map(|rl| rl.object_name.clone())
  }
}

#[derive(Serialize)]
pub struct Item<'a> {
  check: &'a Check,
  result: CheckResult,
  passed: bool
}

impl<'a> Item<'a> {
  fn build(check: &'a Check, summary: &Summary) -> Self {
    let result = (check.eval)(summary);
    Item{
      check,
      result,
      passed: matches!(result, CheckResult::Passed)
    }
  }
}

impl fmt::Display for Summary<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let width = self.items().iter().map(|i| i.check.label.len()).max();
    for i in self.items() {
      writeln!(
        f,
        "  {:>width$}: {}",
        i.check.label,
        i.passed,
        width = width.unwrap_or(0)
      )?;
    }
    Ok(())
  }
}

impl fmt::Display for Item<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}: {}", self.check.label, self.passed)
  }
}

static ALL_CHECKS: [Check; 9] = [
  Check {
    label: "all commits pushed to remote",
    tags: &["push", "local", "git_prompt"],
    glyph: '↑',
    status_group: 2,
    required_data: STATUS,
    eval: unpushed_commit,
    threshold: 0,
  },
  Check {
    label: "all commits merged from remote",
    tags: &["merge"],
    glyph: '↓',
    status_group: 3,
    required_data: union(STATUS, REMOTE),
    eval: remote_changes,
    threshold: 0,
  },
  Check {
    label: "no uncommited changes",
    tags: &["commit", "local", "git_prompt"],
    glyph: '.',
    status_group: 1,
    required_data: STATUS,
    eval: uncommited_changes,
    threshold: 0,
  },
  Check {
    label: "no unstaged changes",
    tags: &["stage", "local", "git_prompt"],
    glyph: '+',
    status_group: 1,
    required_data: STATUS,
    eval: modified_files,
    threshold: 0,
  },
  Check{
    label: "all files tracked",
    tags: &["track_files", "local", "git_prompt"],
    glyph: '?',
    status_group: 1,
    required_data: STATUS,
    eval: untracked_files,
    threshold: 0,
  },
  Check {
    label: "commit tracked by local ref",
    tags: &["detached", "local", "git_prompt"],
    glyph: '⌱',
    status_group: 1,
    required_data: STATUS,
    eval: detached_head,
    threshold: 0,
  },
  Check {
    label: "branch tracks remote",
    tags: &["track_remote", "local", "git_prompt"],
    glyph: '⍏',
    status_group: 2,
    required_data: STATUS,
    eval: untracked_branch,
    threshold: 0,
  },
  Check {
    label: "current commit is tagged",
    tags: &["tag", "local"],
    glyph: '🏷',
    status_group: 4,
    required_data: union(STATUS, REFS),
    eval: untagged_commit,
    threshold: 0,
  },
  Check {
    label: "tag is pushed",
    tags: &["push_tag"],
    glyph: '🏳',
    status_group: 4,
    required_data: union(STATUS, REMOTE),
    eval: unpushed_tag,
    threshold: 0,
  },
  ];

fn untracked_files(s: &Summary) -> CheckResult {
  s.status
    .lines
    .iter()
    .filter(|line| matches!(line, Untracked{..}))
    .count()
    .into()
}

fn modified_files(s: &Summary) -> CheckResult {
  s.status
    .lines
    .iter()
    .filter(|line| matches!(line,
        One {
          status: StatusPair { unstaged: m, .. },
          ..
        }
        | Two {
          status: StatusPair { unstaged: m, .. },
          ..
        }
        | Unmerged {
          status: StatusPair { unstaged: m, .. },
          ..
        } if *m != LineStatus::Unmodified
    ))
    .count()
    .into()
}

fn uncommited_changes(s: &Summary) -> CheckResult {
  s.status
    .lines
    .iter()
    .filter(|line| matches!( line,
        One {
          status: StatusPair { staged: m, .. },
          ..
        }
        | Two {
          status: StatusPair { staged: m, .. },
          ..
        }
        | Unmerged {
          status: StatusPair { staged: m, .. },
          ..
        } if *m != LineStatus::Unmodified
    ))
    .count()
    .into()

}

fn detached_head(s: &Summary) -> CheckResult {
  s.status
    .branch
    .clone()
    .map_or(false, |b| b.head != Head::Detached)
    .into()
}

fn untracked_branch(s: &Summary) -> CheckResult {
  s.status
    .branch
    .clone()
    .map_or(false, |b| b.upstream.is_some())
    .into()
}

fn remote_changes(s: &Summary) -> CheckResult {
  s.status
    .branch
    .clone()
    .map_or(1, |b| {
      b.commits
        .map_or(1, |TrackingCounts(_, behind)| behind)
    })
    .into()

}

fn unpushed_commit(s: &Summary) -> CheckResult {
  s.status
    .branch
    .clone()
    .map_or(1, |b| {
      b.commits
        .map_or(1, |TrackingCounts(ahead, _)| ahead)
    })
  .into()
}

fn untagged_commit(s: &Summary) -> CheckResult {
  (if let Some(Oid::Commit(c)) = s.status.branch.clone().map(|b| b.oid) {
    s.tag_on_commit(c).is_some()
  } else {
    false
  })
  .into()
}

fn unpushed_tag(s: &Summary) -> CheckResult {
  (if let Some(Oid::Commit(c)) = s.status.branch.clone().map(|b| b.oid) {
    if let Some(t) = s.tag_on_commit(c) {
      s.ls_remote.iter().any(|rp| rp.refname == t)
    } else {
      false
    }
  } else {
    false
  })
  .into()
}
