mod git;
mod preserves;

use clap::{App, Arg};
use preserves::{Check, Summary};

fn main() -> Result<(), git::Error> {
    let opt = App::new("Confit")
        .version("0.1")
        .author("Judson Lester <nyarly@gmail.com>")
        .about("makes sure your work is properly preserved in git")
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .help("outputs debug data")
        )
        .arg(
            Arg::with_name("quiet")
                .long("quiet")
                .short("q")
                .help("suppress normal state summary; scripts can rely on the status code")
        )
        .arg(
            Arg::with_name("local only")
            .long("local")
            .short("l")
            .help("do not access remote repos")
            )
        .arg(
            Arg::with_name("checks")
            .short("checks")
            .use_delimiter(true)
            .takes_value(true)
            .multiple(true)
            .possible_values(&Check::all_tags()))
        .get_matches();

    let checks = if let Some(tags) = opt.values_of("checks") {
        Check::tagged_checks(tags)
    } else {
        Check::all_checks()
    };

    let ls_remote = if opt.is_present("local only") {
        vec![]
    } else {
        git::ls_remote()?
    };

    let status = git::status()?;
    let for_each_ref = git::for_each_ref()?;

    if opt.is_present("debug") {
        println!("{:#?}\n{:#?}\n{:#?}", status, for_each_ref, ls_remote);
    }

    let summary = Summary::new(ls_remote, status, for_each_ref, checks);

    if opt.is_present("debug") {
        print!("will exit: {}", summary.exit_status())
    }

    if !opt.is_present("quiet") {
        print!("{}", summary)
    }

    std::process::exit(summary.exit_status())
}

/* Stages of execution:
 * Choose Checks
 * Hassle Git for data
 *   - only need data that active checks care about
 * Interpret data with Checks
 *   - only run requested checks
 * Format interpretations
 *   - many options here
 * Exit status per interpretations
 *   - only active
 */

/*
 * Args:
 * color output
 * tracking scenarios
 * tracking selections
 *
 */
