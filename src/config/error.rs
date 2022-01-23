use std::{fmt, error};
use std::convert::{From};
use std::ffi::{OsString};

#[derive(Debug)]
pub enum Error {
    InvalidConfig(String),

    UnknownExtension,
    InvalidExtension(OsString),

    InvalidFile(OsString),
    FileNotFound(String),

    InvalidIpAddr(String),

    JsonError(serde_json::Error),
    YamlError(serde_yaml::Error),

    IOError(std::io::Error),
    FMTError(std::fmt::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidConfig(msg) => write!(f, "{}", msg),
            Error::UnknownExtension => write!(f, "unknown file extension given"),
            Error::InvalidExtension(ext) => write!(f, "invalid file extension given. \"{:?}\"", ext),
            Error::InvalidFile(file_path) => write!(f, "invalid file given: \"{:?}\"", file_path),
            Error::FileNotFound(file_path) => write!(f, "requested file was not found: \"{:?}\"", file_path),
            Error::InvalidIpAddr(ip) => write!(f, "invalid ipv4 or ipv6 address given. ip: \"{}\"", ip),
            Error::JsonError(err) => {
                match err.classify() {
                    serde_json::error::Category::Io => write!(
                        f, "json io error"
                    ),
                    serde_json::error::Category::Syntax => write!(
                        f, "json syntax error {}:{}", err.line(), err.column()
                    ),
                    serde_json::error::Category::Data => write!(
                        f, "json data error"
                    ),
                    serde_json::error::Category::Eof => write!(
                        f, "json eof error"
                    )
                }
            },
            Error::YamlError(err) => {
                write!(f, "yaml error {}", err.to_string())
            },
            Error::IOError(err) => write!(f, "std::io::Error {:?}", err),
            Error::FMTError(err) => write!(f, "std::fmt::Error {:?}", err),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::JsonError(err) => Some(err),
            Error::YamlError(err) => Some(err),
            Error::IOError(err) => Some(err),
            _ => None
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::JsonError(error)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(error: serde_yaml::Error) -> Self {
        Error::YamlError(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IOError(error)
    }
}

impl From<std::fmt::Error> for Error {
    fn from(error: std::fmt::Error) -> Self {
        Error::FMTError(error)
    }
}