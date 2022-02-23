use crate::{http::{Request, error::Result, Response, response}, state::AppState};

pub async fn handle_get(_state: AppState<'_>, _req: Request) -> Result<Response> {
    response::json_okay_response(200)
}

pub async fn handle_post(_state: AppState<'_>, _req: Request) -> Result<Response> {
    response::json_okay_response(200)
}

pub async fn handle_delete(_state: AppState<'_>, _req: Request) -> Result<Response> {
    response::json_okay_response(200)
}