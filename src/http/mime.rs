use std::{ffi::OsStr, collections::HashMap};

use hyper::HeaderMap;
use mime::Mime;
use lazy_static::lazy_static;

lazy_static! {
    static ref EXT_MIME_MAP: HashMap<&'static OsStr, &'static str> = {
        let mut m: HashMap<&'static OsStr, &'static str> = HashMap::new();
        // image mime types
        m.insert(OsStr::new("jpg"), "image/jpeg");
        m.insert(OsStr::new("jpeg"), "image/jpeg");
        m.insert(OsStr::new("png"), "image/png");
        m.insert(OsStr::new("gif"), "image/gif");
        m.insert(OsStr::new("svg"), "image/svg+xml");
        m.insert(OsStr::new("webp"),"image/webp");
        m.insert(OsStr::new("ico"), "image/x-icon");

        // text mime types
        m.insert(OsStr::new("css"), "text/css");
        m.insert(OsStr::new("html"), "text/html");
        m.insert(OsStr::new("txt"), "text/plain");

        // application mime types
        m.insert(OsStr::new("js"), "application/javascript");
        m.insert(OsStr::new("json"), "application/json");
        m
    };
}

use crate::http::error;

pub fn get_accepting_default<S>(
    headers: &HeaderMap, default: S
) -> error::Result<Vec<mime::Mime>>
where
    S: AsRef<str>
{
    if let Some(accept) = headers.get("accept") {
        let mut rtn: Vec<mime::Mime> = Vec::new();

        for item in accept.to_str()?.split(",") {
            rtn.push(item.parse()?);
        }

        Ok(rtn)
    } else {
        Ok(vec!(default.as_ref().parse()?))
    }
}

pub fn mime_type_from_ext(ext: Option<&OsStr>) -> Mime {
    if let Some(ext) = ext {
        if let Some(mime_str) = EXT_MIME_MAP.get(ext) {
            (*mime_str).parse().unwrap()
        } else {
            mime::APPLICATION_OCTET_STREAM
        }
    } else {
        mime::APPLICATION_OCTET_STREAM
    }
}