use std::{fmt, error};
use std::convert::{From};

use crate::config::error as config_error;

#[derive(Debug)]
pub enum Error {
    ConfigError(config_error::Error),
    IOError(std::io::Error),

    HyperError(hyper::Error),
    NotifyError(notify::Error),
    PostgresError(tokio_postgres::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ConfigError(err) => write!(f, "{}", err),
            Error::IOError(err) => write!(f, "{:?}", err),
            Error::HyperError(err) => write!(f, "{:?}", err),
            Error::NotifyError(err) => write!(f, "{:?}", err),
            Error::PostgresError(err) => write!(f, "{:?}", err),
            _ => write!(f, "application error")
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::ConfigError(err) => err.source(),
            Error::IOError(err) => Some(err),
            Error::HyperError(err) => Some(err),
            Error::NotifyError(err) => Some(err),
            Error::PostgresError(err) => Some(err),
            _ => None
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Error {
        Error::IOError(error)
    }
}

impl From<config_error::Error> for Error {
    fn from(error: config_error::Error) -> Error {
        Error::ConfigError(error)
    }
}

impl From<hyper::Error> for Error {
    fn from(error: hyper::Error) -> Error {
        Error::HyperError(error)
    }
}

impl From<notify::Error> for Error {
    fn from(error: notify::Error) -> Error {
        Error::NotifyError(error)
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(error: tokio_postgres::Error) -> Error {
        Error::PostgresError(error)
    }
}