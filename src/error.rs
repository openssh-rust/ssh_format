use std::fmt::{self, Display};
use std::str::Utf8Error;

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Message(String),
    TrailingBytes,
    Eof,
    DeserializeAnyNotSupported,
    InvalidBoolEncoding,
    InvalidChar,
    InvalidStr(Utf8Error),
    Unsupported,
    SeqMustHaveLen,
    SeqLenTooLong,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::TrailingBytes => formatter.write_str("Trailing bytes"),
            Error::Eof => formatter.write_str("EOF"),
            Error::DeserializeAnyNotSupported =>
                formatter.write_str("deserialize_any is not supported"),
            Error::InvalidBoolEncoding => formatter.write_str("InvalidBoolEncoding"),
            Error::InvalidChar => formatter.write_str("Invalid char"),
            Error::InvalidStr(err) =>
                formatter.write_fmt(format_args!("Invalid str: {:#?}", err)),
            Error::Unsupported => formatter.write_str("Unsupported"),
            Error::SeqMustHaveLen =>
                formatter.write_str("When serializing sequence, length must be provided"),
            Error::SeqLenTooLong =>
                formatter.write_str("Sequence length must not be larger than u32"),
        }
    }
}

impl std::error::Error for Error {}
