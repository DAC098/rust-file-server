use std::str::FromStr;

use hyper::HeaderMap;
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
                return Err(Error::new(400, "InvalidAcceptHeader", "given failed to parse given accept header value"));
            }
        }
    }

    Ok(false)
}

pub fn response_index_html_parts(template: ArcTemplateState) -> Result<Response> {
    let template_data = json!({});
    let render = template.render("page/index", &template_data)?;

    Ok(build()
        .status(200)
        .header("content-type", "text/html")
        .body(render.into())?)
}