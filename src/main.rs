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
use rand::{Rng,rngs::StdRng};
use rand::SeedableRng;
use getrandom::getrandom;

lazy_static! {
  pub static ref TEMPLATE_DIR: Dir<'static> = include_dir!("src/templates");
  pub static ref TEMPLATES: Vec<(String, &'static str)> = {
    TEMPLATE_DIR.find("**/*.txt").expect("static dir")
      .filter_map(|entry| match entry { DirEntry::File(f) => Some(f), _ => None })
      .map(|f| {
        let tpath = f.path().with_extension("");
        let tname = tpath.to_str().expect("utf8 pathname").into();
        let body = f.contents_utf8().expect(&*format!("{} contents", tname));
        (tname, body)
      }).collect()
  };
  pub static ref TMPL: Tera = {
    let mut tera = Tera::default();
    tera.add_raw_templates((*TEMPLATES).clone()).expect(&*format!("templates to parse"));
    tera
  };
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
      Arg::with_name("seed-file")
      .long("seed-file")
      .help("reads (or writes) the example seed from (or to) a file")
      .requires("example")
      .takes_value(true)
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
      let mut r = load_rng(opt.value_of("seed-file"));
      Summary::new(
        (Faker, 0..10).fake_with_rng(&mut r),
        Faker.fake_with_rng(&mut r),
        (Faker, 0..10).fake_with_rng(&mut r),
        Check::all_checks()
      )
    } else {
      Summary::new(
        collect(LsRemote, reqs, 128),
        collect(GetStatus, reqs, 129),
        collect(ForEachRef, reqs, 130),
        checks
      )
    };

    if opt.is_present("debug") {
      println!("{:#?}\n{:#?}\n{:#?}", summary.status, summary.for_each_ref, summary.ls_remote);
    }

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

fn load_rng(seedpath: Option<&str>) -> impl Rng {
  let ref mut seed = [0; 32];
  use std::fs::File;
  use std::io::{Read,Write};
  match seedpath {
    None => getrandom(seed).unwrap_or_else(&error_status(131)),
    Some(path) => {
      match File::open(path) {
        Ok(mut f) => f.read(seed).unwrap_or_else(&error_status(132)),
        Err(_) => {
          getrandom(seed).unwrap_or_else(&error_status(133));
          let mut f = File::create(path).unwrap_or_else(&error_status(134));
          f.write(seed).unwrap_or_else(&error_status(135))
        }
      };
    }
  }

  StdRng::from_seed(*seed)
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
