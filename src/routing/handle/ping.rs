use crate::http::{Request, error::Result, Response, response::JsonResponseBuilder};

pub async fn handle_get(_req: Request) -> Result<Response> {
    JsonResponseBuilder::new(200)
        .set_message("pong")
        .response()
}