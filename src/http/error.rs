use std::fmt;
use std::convert::From;
//use std::borrow::Borrow;

use lib;

#[derive(Debug)]
pub struct MessageError(String);

impl std::error::Error for MessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    pub status: u16,
    pub name: String,
    pub msg: String,
    pub source: Option<BoxDynError>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn with_source<T>(source: T) -> Error
    where
        T: Into<BoxDynError>
    {
        let mut rtn = Self::default();
        rtn.source = Some(source.into());
        rtn
    }
}

impl Default for Error {
    fn default() -> Error {
        Error {
            status: 500,
            name: "InternalServerError".to_owned(),
            msg: "server error when responding to request".to_owned(),
            source: None
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
        // not sure how to return the source error if this is ever called
        // but
        // if let Some(err) = self.source.as_ref() {
        //     Some(err.borrow())
        // } else {
        //     None
        // }
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

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Self::with_source(MessageError(msg))
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Self::with_source(MessageError(msg.to_owned()))
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<std::fmt::Error> for Error {
    fn from(error: std::fmt::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<mime::FromStrError> for Error {
    fn from(error: mime::FromStrError) -> Self {
        Self::with_source(error)
    }
}

impl From<hyper::http::Error> for Error {
    fn from(error: hyper::http::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<hyper::Error> for Error {
    fn from(error: hyper::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<hyper::header::ToStrError> for Error {
    fn from(error: hyper::header::ToStrError) -> Self {
        Self::with_source(error)
    }
}

impl From<handlebars::RenderError> for Error {
    fn from(error: handlebars::RenderError) -> Self {
        Self::with_source(error)
    }
}

impl From<bb8::RunError<tokio_postgres::Error>> for Error {
    fn from(error: bb8::RunError<tokio_postgres::Error>) -> Self {
        Self::with_source(error)
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(error: tokio_postgres::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<lib::snowflake::Error> for Error {
    fn from(error: lib::snowflake::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<argon2::Error> for Error {
    fn from(error: argon2::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::with_source(error)
    }
}