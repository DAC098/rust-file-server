//use crate::http::{error::Result, error::Error};
use hyper::{HeaderMap, header::ToStrError};
/*
pub fn get_header<'a>(header: &'a HeaderMap, key: &str) -> Result<Option<&'a str>> {
    if let Some(value) = header.get(key) {
        match value.to_str() {
            Ok(v) => Ok(Some(v)),
            Err(err) => Err(Error {
                status: 400,
                name: "InvalidHeaderValue".into(),
                msg: format!("requested header value contains invalid characters. header: {}", key),
                source: Some(err.into())
            })
        }
    } else {
        Ok(None)
    }
}
*/
pub fn copy_header_value(headers: &HeaderMap, key: &str) -> Option<std::result::Result<String, ToStrError>> {
    if let Some(value) = headers.get(key) {
        Some(value.to_str().map(|v| v.to_owned()))
    } else {
        None
    }
}