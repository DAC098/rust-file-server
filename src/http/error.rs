use std::{fmt, error as std_error};
use std::convert::From;
use std::borrow::Borrow;

use lib;

#[derive(Debug)]
pub struct Error {
    pub status: u16,
    pub name: String,
    pub msg: String,
    pub source: Option<Box<dyn std_error::Error>>
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {

    #[inline]
    fn option_box_error<E>(prev: Option<E>) -> Option<Box<dyn std_error::Error>>
    where
        E: Into<Box<dyn std_error::Error>> 
    {
        if let Some(err) = prev {
            Some(err.into())
        } else {
            None
        }
    }

    pub fn internal_server_error<E>(
        prev: Option<E>
    ) -> Error
    where
        E: Into<Box<dyn std_error::Error>> 
    {
        Error {
            status: 500,
            name: "InternalServerError".to_owned(),
            msg: "server error when responding to request".to_owned(),
            source: Self::option_box_error(prev)
        }
    }
}

impl std_error::Error for Error {
    fn source(&self) -> Option<&(dyn std_error::Error + 'static)> {
        if let Some(err) = self.source.as_ref() {
            Some(err.borrow())
        } else {
            None
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.msg)?;

        if let Some(err) = &self.source {
            write!(f, "\n{:?}", err)?;
        }

        Ok(())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<std::fmt::Error> for Error {
    fn from(error: std::fmt::Error) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<mime::FromStrError> for Error {
    fn from(error: mime::FromStrError) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<hyper::http::Error> for Error {
    fn from(error: hyper::http::Error) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<hyper::Error> for Error {
    fn from(error: hyper::Error) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<handlebars::RenderError> for Error {
    fn from(error: handlebars::RenderError) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<hyper::header::ToStrError> for Error {
    fn from(error: hyper::header::ToStrError) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<bb8::RunError<tokio_postgres::Error>> for Error {
    fn from(error: bb8::RunError<tokio_postgres::Error>) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(error: tokio_postgres::Error) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<lib::snowflake::Error> for Error {
    fn from(error: lib::snowflake::Error) -> Self {
        Self::internal_server_error(Some(error))
    }
}

impl From<argon2::Error> for Error {
    fn from(error: argon2::Error) -> Self {
        Self::internal_server_error(Some(error))
    }
}