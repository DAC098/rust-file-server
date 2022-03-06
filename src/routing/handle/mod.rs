use crate::{http::{Request, error::Result, Response, response::{redirect_response, JsonResponseBuilder}}, components::{html::check_if_html_headers, auth::get_session}, db::ArcDBState};

pub mod fs;
pub mod sync;
pub mod listeners;
pub mod auth;
pub mod admin;
pub mod _static_;

pub async fn handle_get(req: Request) -> Result<Response> {
    let (mut head, _) = req.into_parts();
    let db = head.extensions.remove::<ArcDBState>().unwrap();
    let conn = db.pool.get().await?;
    let session = get_session(&head.headers, &*conn).await;

    if check_if_html_headers(&head.headers)? {
        match session {
            Ok(_) => redirect_response("/fs/"),
            Err(_) => redirect_response("/auth/session")
        }
    } else {
        session?;

        JsonResponseBuilder::new(200)
            .response()
    }
}