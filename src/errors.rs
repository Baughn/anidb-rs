extern crate rusqlite;

use std::io;
use std::str;
use std::fmt;
use std::num;
use std::error::Error;
use std::result;

pub type Result<T> = result::Result<T, AnidbError>;

#[derive(Debug)]
pub enum AnidbError {
    Io(io::Error),
    Utf8Error(str::Utf8Error),
    ParseIntError(num::ParseIntError),
    StaticError(&'static str),
    ErrorCode(i32, String),
    Error(String),
    SqliteError(rusqlite::Error),
    NoSuchFile,
}

impl fmt::Display for AnidbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AnidbError::Io(ref err) => err.fmt(f),
            AnidbError::Utf8Error(ref err) => err.fmt(f),
            AnidbError::ParseIntError(ref err) => err.fmt(f),
            AnidbError::StaticError(ref err) => err.fmt(f),
            AnidbError::ErrorCode(size, ref string) => write!(f, "Error {} - {}", size, string),
            AnidbError::Error(ref string) => write!(f, "{}", string),
            AnidbError::SqliteError(ref err) => err.fmt(f),
            AnidbError::NoSuchFile => write!(f, "No such file"),
        }
    }
}

impl Error for AnidbError  {
    fn description(&self) -> &str {
        match *self {
            AnidbError::Io(ref err) => err.description(),
            AnidbError::Utf8Error(ref err) => err.description(),
            AnidbError::ParseIntError(ref err) => err.description(),
            AnidbError::StaticError(err) => err,
            AnidbError::ErrorCode(_size, ref _string) => "Error Code",
            AnidbError::Error(ref string) => string.as_str(),
            AnidbError::SqliteError(ref err) => err.description(),
            AnidbError::NoSuchFile => "No such file",
        }
    }
}

impl From<io::Error> for AnidbError {
    fn from(err: io::Error) -> AnidbError {
        AnidbError::Io(err)
    }
}

impl From<str::Utf8Error> for AnidbError {
    fn from(err: str::Utf8Error) -> AnidbError {
        AnidbError::Utf8Error(err)
    }
}

impl From<num::ParseIntError> for AnidbError {
    fn from(err: num::ParseIntError) -> AnidbError {
        AnidbError::ParseIntError(err)
    }
}

impl From<rusqlite::Error> for AnidbError {
    fn from(err: rusqlite::Error) -> AnidbError {
        AnidbError::SqliteError(err)
    }
}
