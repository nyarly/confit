
pub(crate) mod write_templates {
  use clap::{App, SubCommand, Arg, ArgMatches};
  use std::path::Path;
  use std::fs::File;
  use std::io::Write;
  use crate::{template_files, error_status};

  pub(crate) fn def() -> App<'static, 'static> {
    SubCommand::with_name("write-templates")
      .about("Writes the default templates out to a given directory")
      .long_about(include_str!("about-write-templates.txt"))
      .arg(Arg::with_name("directory")
        .required(true))
  }

  pub(crate) fn run(args: &ArgMatches) {
    let dirname = args.value_of("directory").expect("directory is required");
    let dir = Path::new(dirname);
    if !dir.is_dir() {
      println!("{} is not a directory!", dirname);
      std::process::exit(1)
    }

    template_files(|name, body| {
      let tpath = dir.join(name);
      println!("{:?}", tpath);
      let mut tfile = File::create(tpath).unwrap_or_else(&error_status(1));
      tfile.write(body.as_bytes()).unwrap_or_else(&error_status(1));
    })
  }
}
