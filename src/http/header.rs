use crate::http::{error::Result, error::Error};
use hyper::HeaderMap;

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