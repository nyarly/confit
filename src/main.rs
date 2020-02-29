mod git;
mod preserves;

use clap::{App, Arg};

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
        .get_matches();

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

    let summary = preserves::Summary::new(ls_remote, status, for_each_ref);

    if opt.is_present("debug") {
        print!("will exit: {}", summary.exit_status())
    }

    if !opt.is_present("quiet") {
        print!("{}", summary)
    }

    std::process::exit(summary.exit_status())
}

/*
 * Args:
 * color output
 * tracking scenarios
 * tracking selections
 *
 */
