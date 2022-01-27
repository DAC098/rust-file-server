use serde_json::json;

use crate::{http::{Request, error::Result, Response, response::{redirect_response, json_response}}, components::{html::check_if_html_headers, auth::RetrieveSession}, db::ArcDBState};

pub mod fs;
pub mod auth;
pub mod admin;
pub mod _static_;

pub async fn handle_get(req: Request) -> Result<Response> {
    let (mut head, _) = req.into_parts();
    let db = head.extensions.remove::<ArcDBState>().unwrap();
    let conn = db.pool.get().await?;
    let session = RetrieveSession::get(&head.headers, &*conn).await?;

    if check_if_html_headers(&head.headers)? {
        match session {
            RetrieveSession::Success(_) => redirect_response("/fs/"),
            _ => redirect_response("/auth/session")
        }
    } else {
        let rtn = json!({
            "message": "okay"
        });

        json_response(200, &rtn)
    }
}