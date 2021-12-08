use std::fmt::{self, Display};
use std::str::Utf8Error;

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Message(Box<str>),
    Eof,
    InvalidBoolEncoding,
    InvalidChar,
    InvalidStr(Utf8Error),

    /// Unsupported error.
    ///
    /// &str is a fat pointer, but &&str is a thin pointer.
    Unsupported(&'static &'static str),
    BytesTooLong,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string().into_boxed_str())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string().into_boxed_str())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::Eof => formatter.write_str("EOF"),
            Error::InvalidBoolEncoding => formatter.write_str("InvalidBoolEncoding"),
            Error::InvalidChar => formatter.write_str("Invalid char"),
            Error::InvalidStr(err) => formatter.write_fmt(format_args!("Invalid str: {:#?}", err)),
            Error::Unsupported(s) => formatter.write_fmt(format_args!("Unsupported {}", s)),
            Error::BytesTooLong => formatter.write_str("Bytes must not be larger than u32::MAX"),
        }
    }
}

impl std::error::Error for Error {}
