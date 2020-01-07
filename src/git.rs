pub mod exec;
pub mod parse;

pub enum Error {
    Exec,
    Utf8,
    LsRemote(String),
    Status(String),
    ForEachRef(String),
    Parse(String),
}

impl From<parse::Err<&str>> for Error {
    fn from(e: parse::Err<&str>) -> Self {
        Error::Parse(format!("{:?}", e))
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_: std::string::FromUtf8Error) -> Self {
        Error::Utf8
    }
}

impl From<exec::Error> for Error {
    fn from(_: exec::Error) -> Self {
        Error::Exec
    }
}

type Result<O> = std::result::Result<O, Error>;

pub fn ls_remote() -> Result<Vec<parse::ls_remote::RefPair>> {
    exec_and_parse(exec::ls_remote, parse::ls_remote, Error::LsRemote)
}

pub fn status() -> Result<Vec<parse::status::StatusLine>> {
    exec_and_parse(exec::status, parse::status, Error::Status)
}

pub fn for_each_ref() -> Result<Vec<parse::for_each_ref::RefLine>> {
    exec_and_parse(exec::for_each_ref, parse::for_each_ref, Error::ForEachRef)
}

fn exec_and_parse<O, E, X, P>(exec: X, parse: P, e: E) -> Result<O>
where
    X: FnOnce() -> exec::Result<std::process::Output>,
    P: FnOnce(&str) -> parse::Result<&str, O>,
    E: FnOnce(String) -> Error,
{
    let out = exec()?;

    if out.status.success() {
        Ok(parse(&String::from_utf8(out.stdout)?)?)
    } else {
        Err(e(String::from_utf8_lossy(&out.stderr).into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_accepts_process_output() {
        let stdout: Vec<u8> = include_str!("git/parse/testdata/mezzo-ls-remote").into();
        parse::ls_remote::parse(&String::from_utf8(stdout).unwrap()).unwrap();
    }
}
