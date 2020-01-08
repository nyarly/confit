mod git;

fn main() -> Result<(), git::Error> {
    let ls_remote = git::ls_remote()?;
    let status = git::status()?;
    let for_each_ref = git::for_each_ref()?;

    println!("{:#?}\n{:#?}\n{:#?}", status, for_each_ref, ls_remote);

    Ok(())
}

/*
 * Args:
 * output control
 *
 * Tracking violations:
 * untracked branch
 */
