mod git;
use clap::{App, Arg};
use std::fmt;

struct Summary {
    status: git::Status,
    ls_remote: Vec<git::RefPair>,
    for_each_ref: Vec<git::RefLine>,
}

struct Item {
    name: &'static str,
    passed: bool,
}

impl Summary {
    fn new(
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
            Item::modified_files(self),
            Item::uncommited_changes(self),
            /*
             * untracked files
             * detached head
             * untracked branch
             * unpushed commit
             * untagged commit
             * remote changes (unpulled)
             */
        ]
    }

    fn exit_status(&self) -> i32 {
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

use git::parse::status::{LineStatus, StatusLine::*, StatusPair};
impl Item {
    fn modified_files(s: &Summary) -> Self {
        let passed = s.status.lines.iter().all(|line| match line {
            One {
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
            } if *m != LineStatus::Unmodified => false,
            _ => true,
        });

        Item {
            name: "no uncommited changes",
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

fn main() -> Result<(), git::Error> {
    let opt = App::new("Confit")
        .version("0.1")
        .author("Judson Lester <nyarly@gmail.com>")
        .about("makes sure your work is properly preserved in git")
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .help("outputs debug data"),
        )
        .arg(
            Arg::with_name("quiet")
                .long("quiet")
                .short("q")
                .help("suppress normal state summary; scripts can rely on the status code"),
        )
        .get_matches();

    let ls_remote = git::ls_remote()?;
    let status = git::status()?;
    let for_each_ref = git::for_each_ref()?;

    if opt.is_present("debug") {
        println!("{:#?}\n{:#?}\n{:#?}", status, for_each_ref, ls_remote);
    }

    let summary = Summary::new(ls_remote, status, for_each_ref);

    if !opt.is_present("quiet") {
        print!("{}", summary)
    }

    std::process::exit(summary.exit_status())
}

/*
 * Args:
 * network access
 * color output
 *
 * Tracking violations:
 * unstaged changes
 * uncommitted changes
 * untracked files
 * untracked branch
 * unpushed commit
 * untagged commit
 * remote changes (unpulled)
 */
