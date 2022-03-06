use std::{path::PathBuf, fmt::Write};

use hyper::{Method, Body};
use tokio::fs::File as TokioFile;
use tokio_util::codec::{FramedRead, BytesCodec};

use crate::{http::{Request, Response, error::Result, error::Error, response, mime::mime_type_from_ext}, state::AppState};

pub async fn handle_req(state: AppState, req: Request) -> Result<Response> {
    let (head, _) = req.into_parts();
    let lookup = head.uri.path().to_owned();
    let mut to_send: Option<PathBuf> = None;

    if let Some(file_path) = state.storage.static_resources.files.get(&lookup) {
        to_send = Some(file_path.clone())
    } else {
        for (key, path) in state.storage.static_resources.directories.iter() {
            if lookup.starts_with(key.as_str()) {
                let stripped = lookup.strip_prefix(key.as_str()).unwrap();
                let mut sanitized = String::with_capacity(stripped.len());
                let mut first = true;

                for value in stripped.split("/") {
                    if value == ".." || value == "." || value.len() == 0 {
                        return Err(Error::new(
                            400, 
                            "MalformedResourcePath", 
                            format!("resource path given contains invalid segments. \"..\", \".\", and \"\" are not allowed in the path")
                        ))
                    }

                    if first {
                        first = false;
                    } else {
                        sanitized.write_char('/')?;
                    }

                    sanitized.write_str(value)?;
                }

                let mut file_path = path.clone();
                file_path.push(sanitized);

                to_send = Some(file_path);
                break;
            }
        }
    }

    if let Some(file_path) = to_send {
        if head.method != Method::GET {
            return Err(Error::new(
                405,
                "MethodNotAllowed",
                "requested method is not accepted by this resource"
            ))
        }

        if file_path.exists() && file_path.is_file() {
            return Ok(response::build()
                .status(200)
                .header("content-type", mime_type_from_ext(file_path.extension()).to_string())
                .body(Body::wrap_stream(
                    FramedRead::new(TokioFile::open(file_path).await?, BytesCodec::new())
                ))?);
        }
    }

    Err(Error::new(404, "NotFound", "the requested resource was not found"))
}