use std::{ffi, io, str};

// error::Error,

use dotenv;
use regex;
use thiserror::Error;

pub type MyResult<T> = Result<T, MyError>;

#[derive(Debug, Error)]
pub enum MyError {
    #[error("{0}")]
    Message(String),

    #[error("{0}")]
    Utf8Error(#[from] str::Utf8Error),

    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("{0}")]
    FromBytesWithNulError(#[from] ffi::FromBytesWithNulError),

    #[error("{0}")]
    NulError(#[from] ffi::NulError),

    #[error("{0}")]
    Regex(#[from] regex::Error),

    #[error("dotenv: {0}")]
    Dotenv(#[from] dotenv::Error),
}

impl From<String> for MyError {
    fn from(msg: String) -> Self {
        Self::Message(msg)
    }
}
