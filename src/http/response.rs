use hyper::{Response as HyperResponse, http::response::Builder};
use serde::Serialize;

use crate::http::error;
use super::types::{Response, Request};
use super::mime::get_accepting_default;

#[inline]
pub fn build() -> Builder {
    HyperResponse::builder()
}

pub fn okay_text_response() -> error::Result<Response> {
    Ok(HyperResponse::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body("okay".into())?)
}

/*
pub fn text_response(status: u16, data: String) -> error::Result<Response> {
    Ok(HyperResponse::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(data.into())?)
}
*/

pub fn json_response<T>(status: u16, data: &T) -> error::Result<Response>
where
    T: Serialize
{
    Ok(HyperResponse::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(serde_json::to_string(data)?.into())?)
}

pub fn okay_response(req: Request) -> error::Result<Response> {
    let accept = get_accepting_default(req.headers(), "text/plain")?;

    for mime in accept {
        if mime.type_() == "text" || mime.type_() == "*" {
            if mime.subtype() == "plain" || mime.subtype() == "*" {
                return okay_text_response();
            } else if mime.subtype() == "html" {
                return okay_text_response();
            }
        } else if mime.type_() == "application" {
            if mime.subtype() == "json" {
                return Ok(HyperResponse::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(r#"{"message": "okay"}"#.into())?)
            }
        }
    }

    okay_text_response()
}