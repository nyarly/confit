use std::process::{Command, Output};

pub enum Error {
    FailToExec,
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::FailToExec
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn ls_remote() -> Result<Output> {
    Ok(Command::new("git").arg("ls-remote").output()?)
}

pub fn status() -> Result<Output> {
    Ok(Command::new("git")
        .arg("status")
        .arg("--branch")
        .arg("--porcelain=v2")
        .output()?)
}

pub fn for_each_ref() -> Result<Output> {
    Ok(Command::new("git")
       .arg("for-each-ref")
       .arg("--shell") // escapes fields
       .arg("--format")
       .arg("%(objectname) %(*objectname) %(objecttype) %(refname) %(upstream) %(upstream:remotename) %(upstream:track) %(creator)")
       .output()?
       )
}
