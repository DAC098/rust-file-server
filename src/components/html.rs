use std::str::FromStr;

use hyper::{HeaderMap, http::request::Parts};
use serde_json::json;

use crate::{http::{error::{Result, Error}, Response, response::build}, template::ArcTemplateState};

pub fn check_if_html_headers(headers: &HeaderMap) -> Result<bool> {
    if let Some(value) = headers.get("accept") {
        for mime in value.to_str()?.split(",").map(|v| mime::Mime::from_str(v)) {
            if let Ok(mime) = mime {
                if mime.type_() == "text" && mime.subtype() == "html" {
                    return Ok(true)
                }
            } else {
                return Err(Error {
                    status: 400,
                    name: "InvalidAcceptHeader".into(),
                    msg: "given failed to parse given accept header value".into(),
                    source: None
                });
            }
        }
    }

    Ok(false)
}

pub fn response_index_html_parts(mut parts: Parts) -> Result<Response> {
    let template = parts.extensions.remove::<ArcTemplateState>().unwrap();
    let template_data = json!({});
    let render = template.render("page/index", &template_data)?;

    Ok(build()
        .status(200)
        .header("content-type", "text/html")
        .body(render.into())?)
}