use crate::http::error::Error as HttpError;

#[derive(Debug)]
pub enum Error {
    General,
    TimestampOverflow,
    Argon2Error(argon2::Error)
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Argon2Error(err) => Some(err),
            _ => None
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::General => write!(f, "security error"),
            Error::TimestampOverflow => write!(f, "timestamp overflow"),
            Error::Argon2Error(err) => write!(f, "Argon2 error: {}", err),
        }
    }
}

impl From<argon2::Error> for Error {
    fn from(error: argon2::Error) -> Self {
        Error::Argon2Error(error)
    }
}

impl From<Error> for HttpError {
    fn from(error: Error) -> Self {
        HttpError::internal_server_error(Some(error))
    }
}