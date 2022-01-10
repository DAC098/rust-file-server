use hyper::{Method, Body};
use tokio::fs::File as TokioFile;
use tokio_util::codec::{FramedRead, BytesCodec};

use crate::{http::{Request, Response, error::Result, error::Error, response, mime::mime_type_from_ext}, storage::ArcStorageState};

pub async fn handle_req(req: Request) -> Result<Response> {
    let (mut head, _) = req.into_parts();
    let storage = head.extensions.remove::<ArcStorageState>().unwrap();

    if let Some(web_static) = storage.web_static.as_ref() {
        if head.method != Method::GET {
            return Err(Error {
                status: 405,
                name: "MethodNotAllowed".into(),
                msg: "requested method is not accepted by this resource".into(),
                source: None
            })
        }

        let req_path = head.uri.path().strip_prefix("/").unwrap_or("");
        let mut file_path = web_static.clone();
        file_path.push(&req_path);

        if file_path.exists() && file_path.is_file() {
            return Ok(response::build()
                .status(200)
                .header("content-type", mime_type_from_ext(file_path.extension()).to_string())
                .body(Body::wrap_stream(
                    FramedRead::new(TokioFile::open(file_path).await?, BytesCodec::new())
                ))?);
        }
    }

    Err(Error {
        status: 404,
        name: "NotFound".into(),
        msg: "the requested resource was not found".into(),
        source: None
    })
}