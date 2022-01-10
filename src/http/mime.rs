use std::ffi::OsStr;

use hyper::HeaderMap;
use mime::Mime;

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
    (if let Some(ext) = ext {
        if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
            "image/jpeg"
        } else if ext.eq_ignore_ascii_case("png") {
            "image/png"
        } else if ext.eq_ignore_ascii_case("gif") {
            "image/gif"
        } else if ext.eq_ignore_ascii_case("svg") {
            "image/svg+xml"
        } else if ext.eq_ignore_ascii_case("webp") {
            "image/webp"
        } else {
            "application/octet-stream"
        }
    } else {
        "application/octet-stream"
    }).parse().unwrap()
}