use crate::git;
use std::fmt;

pub struct Summary {
    status: git::Status,
    ls_remote: Vec<git::RefPair>,
    for_each_ref: Vec<git::RefLine>,
}

struct Item {
    name: &'static str,
    passed: bool,
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
        vec![
            Item::untracked_files(self),
            Item::modified_files(self),
            Item::uncommited_changes(self),
            Item::detached_head(self),
            Item::untracked_branch(self),
            Item::unpushed_commit(self),
            Item::remote_changes(self),
            Item::untagged_commit(self),
        ]
    }

    pub fn exit_status(&self) -> i32 {
        self.items()
            .iter()
            .enumerate()
            .fold(0, |status, (n, item)| {
                if item.passed {
                    status
                } else {
                    status + (1 << n)
                }
            })
    }
}

use git::parse::status::{Head, LineStatus, StatusLine::*, StatusPair, Oid};
use git::parse::for_each_ref::ObjectType::*;
use git::parse::TrackingCounts;
impl Item {
    fn untracked_files(s: &Summary) -> Self {
        let passed = s.status.lines.iter().all(|line| match line {
            Untracked { .. } => false,
            _ => true,
        });

        Item {
            name: "no unstaged changes",
            passed,
        }
    }

    fn modified_files(s: &Summary) -> Self {
        let passed = s.status.lines.iter().all(|line| match line {
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
        });

        Item {
            name: "no unstaged changes",
            passed,
        }
    }

    fn uncommited_changes(s: &Summary) -> Self {
        let passed = s.status.lines.iter().all(|line| match line {
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
        });

        Item {
            name: "no uncommited changes",
            passed,
        }
    }

    fn detached_head(s: &Summary) -> Self {
        let passed = s.status.branch.map_or(false, |b| b.head != Head::Detached);
        Item {
            name: "commit tracked by local ref",
            passed,
        }
    }

    fn untracked_branch(s: &Summary) -> Self {
        let passed = s.status.branch.map_or(false, |b| b.upstream.is_some());
        Item {
            name: "branch tracks remote",
            passed,
        }
    }

    fn remote_changes(s: &Summary) -> Self {
        let passed = s.status.branch.map_or(false, |b| {
            b.commits
                .map_or(false, |TrackingCounts(_, behind)| behind == 0)
        });
        Item {
            name: "all commits merged from remote",
            passed,
        }
    }

    fn unpushed_commit(s: &Summary) -> Self {
        let passed = s.status.branch.map_or(false, |b| {
            b.commits
                .map_or(false, |TrackingCounts(ahead, _)| ahead == 0)
        });
        Item {
            name: "all commits pushed to remote",
            passed,
        }
    }

    fn untagged_commit(s: &Summary) -> Self {
        let passed = if let Some(oid) = s.status.branch.map(|b| b.oid) {
            if let Oid::Commit(c) = oid {
                s.for_each_ref.iter().any(|rl| rl.object_type == Tag && rl.object_name == c)
            } else { false }
        } else { false };

        Item {
            name: "current commit is tagged",
            passed,
        }

    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in self.items() {
            writeln!(f, "{}", i)?;
        }
        Ok(())
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.passed)
    }
}
