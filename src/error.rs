use std::{fmt::Display, string::FromUtf8Error};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// Index is not built properly
    MalformedIndex,
    /// Index is missing
    MissingIndex,
    /// On reqest for a non existing index entry
    OutOfBounds,
    UTF8Error,
    NotFound,
}

impl From<FromUtf8Error> for Error {
    fn from(_: FromUtf8Error) -> Self {
        Self::UTF8Error
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
