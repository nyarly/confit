pub mod exec;
pub mod parse;

pub use parse::ls_remote::RefPair;
pub use parse::status::Status;
pub use parse::for_each_ref::RefLine;

use fake::{Fake, Faker};
use crate::preserves::datasource::{self,Group};

#[derive(Debug)]
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
    Error::Parse(format!("{}", e))
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

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use Error::*;
    match self {
      Exec => write!(f, "problem executing git"),
      Utf8 => write!(f, "utf8 translation error"),
      LsRemote(s) => write!(f, "ls-remote parse error: {}", s),
      Status(s) => write!(f, "status parse error: {}", s),
      ForEachRef(s) => write!(f, "for-each-ref parse error: {}", s),
      Parse(s) => write!(f, "parse error: {}", s),
    }
  }
}

impl std::error::Error for Error {}

type Result<O> = std::result::Result<O, Error>;

pub trait Provider {
  type Data;

  fn get(&self) -> Result<Self::Data>;

  fn empty(&self) -> Self::Data;

  fn provides(&self) -> Group;

  fn collect(&self, reqs: Group) -> Result<Self::Data> {
    if reqs.includes(self.provides()) {
      self.get()
    } else {
      Ok(self.empty())
    }
  }

  fn example(&self) -> Self::Data {
    self.empty()
  }
}

pub struct LsRemote;

impl Provider for LsRemote {
  type Data = Vec<RefPair>;

  fn get(&self) -> Result<Self::Data> {
    exec_and_parse(exec::ls_remote, parse::ls_remote, Error::LsRemote)
  }

  fn empty(&self) -> Self::Data {
    vec![]
  }

  fn provides(&self) -> Group {
    datasource::REMOTE
  }
}

pub struct GetStatus;

impl Provider for GetStatus {
  type Data = Status;

  fn get(&self) -> Result<Self::Data> {
    exec_and_parse(exec::status, parse::status, Error::Status)
  }

  fn empty(&self) -> Self::Data {
    Status::default()
  }

  fn provides(&self) -> Group {
    datasource::STATUS
  }

  fn example(&self) -> Self::Data {
    Faker.fake()
  }
}

pub struct ForEachRef;

impl Provider for ForEachRef {
  type Data = Vec<RefLine>;

  fn get(&self) -> Result<Self::Data> {
    exec_and_parse(exec::for_each_ref, parse::for_each_ref, Error::ForEachRef)
  }

  fn empty(&self) -> Self::Data {
    vec![]
  }

  fn provides(&self) -> Group {
    datasource::REFS
  }
}

// collect(LsRemote, reqs).unwrap_or_exit(128)

fn exec_and_parse<O, E, X, P>(exec: X, parse: P, e: E) -> Result<O>
where
    X: FnOnce() -> exec::Result<std::process::Output>,
    P: FnOnce(&str) -> parse::Result<&str, O>,
    E: FnOnce(String) -> Error,
{
  let out = exec()?;

  if out.status.success() {
    //println!("{}", String::from_utf8_lossy(&out.stdout));
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
