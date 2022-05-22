use crate::{http::{Request, error::Result, Response, response::{redirect_response, JsonResponseBuilder}}, components::{html::check_if_html_headers, auth::get_session}, state::AppState};

pub mod ping;

pub mod auth;
pub mod admin;

pub mod session;
pub mod fs;
pub mod sync;
pub mod listeners;
pub mod _static_;

pub async fn handle_get(mut req: Request) -> Result<Response> {
    let state = AppState::from(&mut req);
    let (head, _) = req.into_parts();
    let conn = state.db.pool.get().await?;
    let session = get_session(&*conn, &head.headers).await?;

    if check_if_html_headers(&head.headers)? {
        if session.is_some() {
            redirect_response("/fs/")
        } else {
            redirect_response("/auth/session")
        }
    } else {
        JsonResponseBuilder::new(200)
            .response()
    }
}