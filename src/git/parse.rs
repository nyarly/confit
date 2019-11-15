extern crate nom;

use nom::{
    bytes::complete::{take_till1, take_while_m_n},
    combinator::map,
    IResult,
};
use std::{error::Error, ffi::OsString, fmt::{Debug, Display, self}};

pub mod status;
pub mod ls_remote;

#[derive(Debug)]
pub enum Err<I> {
    Trailing(I),
    Failed(nom::Err<(I, nom::error::ErrorKind)>),
    Incomplete(nom::Err<(I, nom::error::ErrorKind)>),
}

impl<I: Display + Debug> Display for Err<I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Err::*;

        match self {
            Trailing(s) => write!(f, "Trailing: {}", s),
            Failed(nom::Err::Error((rest, kind))) =>  write!(f, "{}: {}", kind.description(), rest),
            Failed(nom::Err::Failure((rest, kind))) =>  write!(f, "{}: {}", kind.description(), rest),
            Incomplete(nom::Err::Incomplete(nom::Needed::Size(x))) =>  write!(f, "Incomplete: needs {}", x),
            Incomplete(nom::Err::Incomplete(nom::Needed::Unknown)) =>  write!(f, "Incomplete, but don't know what's needed"),
            otherwise => write!(f, "Unexpected error: {:?}", otherwise)
            }
    }
}

impl<I: Display + Debug> Error for Err<I> {}

impl<I> From<nom::Err<(I, nom::error::ErrorKind)>> for Err<I> {
    fn from(ne: nom::Err<(I, nom::error::ErrorKind)>) -> Err<I> {
        match ne {
            e @ nom::Err::Error(_) => Err::Failed(e),
            e @ nom::Err::Failure(_) => Err::Failed(e),
            e @ nom::Err::Incomplete(_) => Err::Incomplete(e)
        }
    }
}

type Result<I, O> = std::result::Result<O, Err<I>>;

fn settle_parse_result<I: Default + Eq,O>(nom_result: IResult<I,O>) -> Result<I,O> {
    match nom_result {
        Ok((i, v)) if i == I::default() => Ok(v),
        Ok((extra, _)) => Err(Err::Trailing(extra)),
        Err(e) => Err(Err::from(e))
    }
}

fn is_hex_digit(c: char) -> bool {
    c.is_digit(16)
}

fn sha(input: &str) -> IResult<&str, &str> {
    take_while_m_n(40, 40, is_hex_digit)(input)
}

fn filepath(input: &str) -> IResult<&str, OsString> {
    map(take_till1(end_of_path), OsString::from)(input)
}

fn end_of_path(input: char) -> bool {
    match input {
        '\t' | '\n' => true,
        _ => false,
    }
}

/*
    match many0(terminated(status_line, tag("\n")))(input) {
        Ok(("", v)) => Ok(v),
k        Ok((extra, _)) => Err(format!("Trailing: {}", extra)),
        Err(nom::Err::Error((rest, kind))) =>  Err(format!("{}: {}", kind.description(), rest)),
        Err(nom::Err::Failure((rest, kind))) =>  Err(format!("{}: {}", kind.description(), rest)),
        Err(nom::Err::Incomplete(nom::Needed::Size(x))) =>  Err(format!("Incomplete: needs {}", x)),
        Err(nom::Err::Incomplete(nom::Needed::Unknown)) =>  Err(format!("Incomplete, but don't know what's needed"))
    }
*/
