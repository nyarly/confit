mod git;
mod preserves;
mod subcommands;

use clap::{App, Arg, crate_authors, crate_version};
use preserves::{Check, Summary, CheckList, datasource::{STATUS, REFS, REMOTE}};
use tera::{Tera, Context};
use lazy_static::lazy_static;
use include_dir::{include_dir,Dir,DirEntry};

lazy_static! {
  pub static ref TEMPLATES: Dir<'static> = include_dir!("src/templates");
  pub static ref TMPL: Tera = {
    let mut tera = Tera::default();
    template_files(|tname, body| {
      tera.add_raw_template(tname, body).expect(&*format!("{} to parse", tname));
    });
    tera
  };
}

fn template_files(mut func: impl FnMut(&str, &str)) {
  for entry in TEMPLATES.find("**/*.txt").expect("static dir") {
    if let DirEntry::File(f) = entry {
      let tpath = f.path().with_extension("");
      let tname = tpath.to_str().expect("utf8 pathname");
      let body = f.contents_utf8().expect(&*format!("{} contents", tname));
      func(tname, body)
    }
  }
}

fn main() -> ! {
  let opt = App::new("Confit")
    //.version(option_env!("CARGO_PKG_VERSION").unwrap_or("dev"))
    .version(crate_version!())
    .author(crate_authors!(", "))
    .about("makes sure your work is properly preserved in git")
    .long_about(include_str!("about.txt"))
    .after_help(include_str!("after.txt"))
    .subcommand(subcommands::write_templates::def())
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
      .possible_values(TMPL.get_template_names()
        .filter(|&n| n != "macros")
        .collect::<Vec<_>>().as_slice())
      .default_value("summary")
    )
    .arg(
      Arg::with_name("checks")
      .long("checks")
      .short("c")
      .use_delimiter(true)
      .takes_value(true)
      .multiple(true)
      .possible_values(&Check::all_tags()))
    .get_matches();

    if let (name, Some(sub_opt)) = opt.subcommand() {
      match name {
        "write-templates" => subcommands::write_templates::run(sub_opt),
        _ => {
          println!("Unknown subcommand: {}", name);
        } //?
      }

      std::process::exit(0)
    }

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
      git::ls_remote().unwrap_or_else(&error_status(128))
    } else {
      vec![]
    };

    let status = if reqs.includes(STATUS) {
      git::status().unwrap_or_else(&error_status(129))
    } else {
      git::Status::default()
    };

    let for_each_ref = if reqs.includes(REFS) {
      git::for_each_ref().unwrap_or_else(&error_status(130))
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
      //println!("status: {}", serde_json::to_string(&summary.status)?);
      //println!("items: {}", serde_json::to_string(&summary.items())?);
      let mut context = Context::default();
      context.insert("items", &summary.items());
      context.insert("status", &summary.status);
      print!("{}",
        TMPL.render(opt.value_of("format").expect("format has no value"), &context)
        .unwrap_or_else(&error_status(131))
      )
    }

    std::process::exit(summary.exit_status())
}

fn error_status<T, E: core::fmt::Debug>(n: i32) -> impl Fn(E) -> T {
  return move |e: E| {
    println!("{:?}", e);
    std::process::exit(n)
  }
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
