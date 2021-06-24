mod git;
mod preserves;

use clap::{App, Arg};
use preserves::{Check, Summary, CheckList, datasource::{STATUS, REFS, REMOTE}};
use anyhow::Result;
use tera::{Tera, Context};
use lazy_static::lazy_static;

lazy_static! {
  pub static ref TMPL: Tera = {
    let mut tera = Tera::default();
    tera.add_raw_template("macros", include_str!("templates/macros.txt")).expect("summary to parse");
    tera.add_raw_template("summary", include_str!("templates/summary.txt")).expect("summary to parse");
    tera.add_raw_template("statusline", include_str!("templates/statusline.txt")).expect("summary to parse");
    tera.add_raw_template("debug", include_str!("templates/debug.txt")).expect("summary to parse");
    tera
  };
}

fn main() -> Result<()> {
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
            .conflicts_with("format")
        )
        .arg(
            Arg::with_name("format")
            .long("format")
            .short("f")
            .help("choose a format for output")
            .possible_values(TMPL.get_template_names().collect::<Vec<_>>().as_slice())
            .default_value("summary")
        )
        .arg(
            Arg::with_name("checks")
            .short("checks")
            .use_delimiter(true)
            .takes_value(true)
            .multiple(true)
            .possible_values(&Check::all_tags()))
        .get_matches();

    let mut checks = if let Some(tags) = opt.values_of("checks") {
        Check::tagged_checks(tags)
    } else {
        Check::all_checks()
    };

    let reqs = checks.required_sources();

    if opt.is_present("debug") {
        println!("Required sources: {:?}", reqs)
    }

    let ls_remote = if reqs.includes(REMOTE) {
        git::ls_remote()?
    } else {
        vec![]
    };

    let status = if reqs.includes(STATUS) {
        git::status()?
    } else {
        git::Status::default()
    };

    let for_each_ref = if reqs.includes(REFS) {
        git::for_each_ref()?
    } else {
        vec![]
    };

    if opt.is_present("debug") {
        println!("{:#?}\n{:#?}\n{:#?}", status, for_each_ref, ls_remote);
    }

    let summary = Summary::new(ls_remote, status, for_each_ref, checks);

    if opt.is_present("debug") {
        println!("will exit: {}", summary.exit_status())
    }

    if !opt.is_present("quiet") {
        //print!("{}", serde_json::to_string(&summary.items())?);
        let mut context = Context::default();
        context.insert("items", &summary.items());
        print!("{}", TMPL.render(opt.value_of("format").expect("format has no value"), &context)?)
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
 *
 */
