use crate::git;
use std::fmt;

use git::parse::for_each_ref::ObjectType::*;
use git::parse::status::{Head, LineStatus, Oid, StatusLine::*, StatusPair};
use git::parse::{ObjectName, TrackingCounts};

pub struct Summary {
    status: git::Status,
    ls_remote: Vec<git::RefPair>,
    for_each_ref: Vec<git::RefLine>,
}

struct Item<'a> {
    check: &'a Check,
    passed: bool,
}

pub struct Check {
    label: &'static str,
    tag: &'static str,
    description: &'static str,
    status_group: u8,
    eval: fn(&Summary) -> bool,
}

impl Check {
    pub fn all_tags() -> Vec<&'static str> {
        ALL_CHECKS.iter().map(|ch| ch.tag).collect()
    }
}

static ALL_CHECKS: [Check; 9] = [
    Check{
        label: "all files tracked",
        tag: "track_files",
        description: "no files in the workspace are untracked",
        status_group: 1,
        eval: untracked_files,
    },
    Check {
        label: "no unstaged changes",
        tag: "stage",
        description: "",
        status_group: 1,
        eval: modified_files,
    },
    Check {
        label: "no uncommited changes",
        tag: "commit",
        description: "",
        status_group: 1,
        eval: uncommited_changes,
    },
    Check {
        label: "commit tracked by local ref",
        tag: "detached",
        description: "",
        status_group: 1,
        eval: detached_head,
    },
    Check {
        label: "branch tracks remote",
        tag: "track_remote",
        description: "",
        status_group: 2,
        eval: untracked_branch,
    },
    Check {
        label: "all commits merged from remote",
        tag: "merge",
        description: "",
        status_group: 3,
        eval: remote_changes,
    },
    Check {
        label: "all commits pushed to remote",
        tag: "push",
        description: "",
        status_group: 2,
        eval: unpushed_commit,
    },
    Check {
        label: "current commit is tagged",
        tag: "tag",
        description: "",
        status_group: 4,
        eval: untagged_commit,
    },
    Check {
        label: "tag is pushed",
        tag: "push_tag",
        description: "",
        status_group: 4,
        eval: unpushed_tag,
    },
];

fn untracked_files(s: &Summary) -> bool {
    s.status.lines.iter().all(|line| match line {
        Untracked { .. } => false,
        _ => true,
    })
}

fn modified_files(s: &Summary) -> bool {
     s.status.lines.iter().all(|line| match line {
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
        } if *m != LineStatus::Unmodified => false,
        _ => true,
    })
}

fn uncommited_changes(s: &Summary) -> bool {
     s.status.lines.iter().all(|line| match line {
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
        } if *m != LineStatus::Unmodified => false,
        _ => true,
    })

}

fn detached_head(s: &Summary) -> bool {
     s.status
        .branch
        .clone()
        .map_or(false, |b| b.head != Head::Detached)
}

fn untracked_branch(s: &Summary) -> bool {
     s
        .status
        .branch
        .clone()
        .map_or(false, |b| b.upstream.is_some())
}

fn remote_changes(s: &Summary) -> bool {
     s.status.branch.clone().map_or(false, |b| {
        b.commits
            .map_or(false, |TrackingCounts(_, behind)| behind == 0)
    })
}

fn unpushed_commit(s: &Summary) -> bool {
     s.status.branch.clone().map_or(false, |b| {
        b.commits
            .map_or(false, |TrackingCounts(ahead, _)| ahead == 0)
    })
}

fn untagged_commit(s: &Summary) -> bool {
     if let Some(oid) = s.status.branch.clone().map(|b| b.oid) {
        if let Oid::Commit(c) = oid {
            s.tag_on_commit(c).is_some()
        } else {
            false
        }
    } else {
        false
    }

}

fn unpushed_tag(s: &Summary) -> bool {
     if let Some(oid) = s.status.branch.clone().map(|b| b.oid) {
        if let Oid::Commit(c) = oid {
            if let Some(t) = s.tag_on_commit(c) {
                s.ls_remote.iter().any(|rp| rp.refname == t)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }

}
/// Collects and reports reasons that your current workspace
/// could not be reproduced on another workstation, in another place or time.
impl Summary {
    pub fn new(
        ls_remote: Vec<git::RefPair>,
        status: git::Status,
        for_each_ref: Vec<git::RefLine>,
    ) -> Self {
        Summary {
            ls_remote,
            status,
            for_each_ref,
        }
    }

    fn items(&self) -> Vec<Item> {
        ALL_CHECKS.iter().map(|ch| Item::build(ch, self)).collect()
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

impl<'a> Item<'a> {
    fn build(check: &'a Check, summary: &Summary) -> Self {
        Item{
            check,
            passed: (check.eval)(summary),
        }
    }

}

impl fmt::Display for Summary {
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

impl<'a> fmt::Display for Item<'a> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.check.label, self.passed)
    }
}
