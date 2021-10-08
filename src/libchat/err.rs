use std::{
    error::Error,
    ffi::{FromBytesWithNulError, NulError},
    fmt,
    str::Utf8Error,
};

pub type MyResult<T> = Result<T, MyError>;

#[derive(Debug)]
pub enum MyError {
    Simple(String),
}

impl Error for MyError {}

/**
 * Implementations for conversion from another type/error
 */

impl From<&'static str> for MyError {
    fn from(err: &'static str) -> Self {
        Self::Simple(err.to_string())
    }
}

impl From<FromBytesWithNulError> for MyError {
    fn from(err: FromBytesWithNulError) -> Self {
        Self::Simple(err.to_string())
    }
}

impl From<NulError> for MyError {
    fn from(err: NulError) -> Self {
        Self::Simple(err.to_string())
    }
}

impl From<Utf8Error> for MyError {
    fn from(err: Utf8Error) -> Self {
        Self::Simple(err.to_string())
    }
}

/**
 * Display
 */

impl fmt::Display for MyError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use MyError::*;
        match self {
            Simple(msg) => write!(fmt, "{}", msg),
        }
    }
}
