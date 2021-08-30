mod git;
mod preserves;
mod subcommands;

use clap::{App, AppSettings, Arg, crate_authors, crate_version};
use preserves::{Check, Summary, CheckList, datasource::Group};
use tera::{Tera, Context};
use lazy_static::lazy_static;
use include_dir::{include_dir,Dir,DirEntry};
use std::path::Path;
use git::{LsRemote, GetStatus, ForEachRef};
use fake::{Fake, Faker};
use rand::rngs::StdRng;
use rand::SeedableRng;

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
    .setting(AppSettings::VersionlessSubcommands)
    .setting(AppSettings::ColoredHelp)
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
      .conflicts_with("template")
    )
    .arg(
      Arg::with_name("format")
      .long("format")
      .short("f")
      .help(format!("choose a format for output [included: {}]",
          TMPL.get_template_names()
          .filter(|&n| n != "macros")
          .collect::<Vec<_>>().as_slice().join(", ")).as_ref())
      .default_value("summary")
    )
    .arg(
      Arg::with_name("template")
      .long("template")
      .short("T")
      .help("provide a template source directory")
      .takes_value(true)
    )
    .arg(
      Arg::with_name("json")
      .long("json")
      .short("j")
      .help("emits json rather than human readable report")
      .conflicts_with("format")
      .conflicts_with("template")
      .conflicts_with("quiet")
    )
    .arg(
      Arg::with_name("example")
      .long("example")
      .help("generates example output for template development")
      .conflicts_with("checks")
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

    let summary = if opt.is_present("example") {
      let seed = [
        1, 0, 0, 0, 23, 0, 0, 0, 200, 1, 0, 0, 210, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
      ];
      let ref mut r = StdRng::from_seed(seed);
      Summary::new(
        fake::vec![_; 5..20],
        Faker.fake_with_rng(r),
        fake::vec![_; 5..20],
        Check::all_checks()
      )
    } else {
      let ls_remote = collect(LsRemote, reqs, 128);
      let status = collect(GetStatus, reqs, 129);
      let for_each_ref = collect(ForEachRef, reqs, 130);

      if opt.is_present("debug") {
        println!("{:#?}\n{:#?}\n{:#?}", status, for_each_ref, ls_remote);
      }

      Summary::new(ls_remote, status, for_each_ref, checks)
    };

    if opt.is_present("debug") {
      println!("will exit: {}", summary.exit_status())
    }

    if !opt.is_present("quiet") {
        let mut context = Context::default();
        context.insert("items", &summary.items());
        context.insert("status", &summary.status);
      if opt.is_present("json") {
        println!("{}", context.into_json());
      } else {
        //println!("status: {}", serde_json::to_string(&summary.status)?);
        //println!("items: {}", serde_json::to_string(&summary.items())?);
        let body = if let Some(tdir) = opt.value_of("template") {
          let tpath = Path::new(tdir).join("**");
          let t = Tera::new(
            tpath.to_str()
            .ok_or("couldn't convert path to utf8")
            .unwrap_or_else(&error_status(133))
          ).unwrap_or_else(&error_status(132));
          t.render(opt.value_of("format").expect("format has no value"), &context)
            .unwrap_or_else(&error_status(131))
        } else {
          TMPL.render(opt.value_of("format").expect("format has no value"), &context)
            .unwrap_or_else(&error_status(131))
        };

        print!("{}", body);
      }
    }

    std::process::exit(summary.exit_status())
}

fn collect<T>( provider: impl git::Provider<Data = T>, reqs: Group, errcode: i32,) -> T {
  provider.collect(reqs).unwrap_or_else(&error_status(errcode))
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
