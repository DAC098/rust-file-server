use crate::http;

#[derive(Debug)]
pub enum ErrorKind {
    General,
}

impl ErrorKind {
    fn as_str(&self) -> &str {
        match self {
            Self::General => "General"
        }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

type BoxDynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    source: Option<BoxDynError>
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(kind: ErrorKind, message: String, source: Option<BoxDynError>) -> Error {
        Error {kind, message, source}
    }

    pub fn with_source<T>(source: T) -> Error
    where
        T: Into<BoxDynError>
    {
        let mut rtn = Error::default();
        rtn.source = Some(source.into());
        rtn
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)?;

        if let Some(err) = &self.source {
            write!(f, "\n{}", err)?;
        }

        Ok(())
    }
}

impl Default for Error {
    fn default() -> Error {
        Error {
            kind: ErrorKind::General,
            message: "database error".into(),
            source: None
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| &**e as _)
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(error: tokio_postgres::Error) -> Self {
        Self::with_source(error)
    }
}

impl From<Error> for http::error::Error {
    fn from(error: Error) -> http::error::Error {
        http::error::Error::with_source(error)
    }
}