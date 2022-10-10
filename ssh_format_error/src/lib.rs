use std::{
    error,
    fmt::{self, Display},
    io,
    str::Utf8Error,
};

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
#[non_exhaustive]
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
    TooLong,

    IoError(io::Error),
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => f.write_str(msg),
            Error::Eof => f.write_str("EOF"),
            Error::InvalidBoolEncoding => f.write_str("InvalidBoolEncoding"),
            Error::InvalidChar => f.write_str("Invalid char"),
            Error::InvalidStr(err) => write!(f, "Invalid str: {:#?}", err),
            Error::Unsupported(s) => write!(f, "Unsupported {}", s),
            Error::TooLong => f.write_str("Bytes must not be larger than u32::MAX"),
            Error::IoError(io_error) => write!(f, "Io error: {}", io_error),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use Error::*;

        match self {
            InvalidStr(utf8_err) => Some(utf8_err),
            IoError(io_error) => Some(io_error),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(io_error: io::Error) -> Self {
        match io_error.kind() {
            io::ErrorKind::UnexpectedEof => Error::Eof,
            _ => Error::IoError(io_error),
        }
    }
}
