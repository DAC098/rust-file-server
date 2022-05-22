use crate::{
    http::{
        error::Result,
        Response,
        response::JsonResponseBuilder,
        Request
    }, 
    db::record::UserSession, 
    components::{auth::{require_session, login_redirect}, html::{check_if_html_headers, response_index_html_parts}},
    state::AppState
};

pub mod session_id;

pub async fn handle_get(state: AppState, req: Request) -> Result<Response> {
    let conn = state.db.pool.get().await?;
    let session_check = require_session(&*conn, req.headers()).await;

    if check_if_html_headers(req.headers())? {
        return match session_check {
            Ok(_) => response_index_html_parts(state.template),
            Err(_) => login_redirect(req.uri())
        }
    }

    let (user, session) = session_check?;
    let user_sessions = UserSession::find_users_id(
        &*conn,
        &user.id,
        Some(&session.token)
    ).await?;

    JsonResponseBuilder::new(200)
        .payload_response(user_sessions)
}

pub async fn handle_delete(state: AppState, req: Request) -> Result<Response> {
    let conn = state.db.pool.get().await?;
    let (user, session) = require_session(&*conn, req.headers()).await?;

    conn.execute(
        "\
        delete from user_session where users_id = $1 and token != $2",
        &[&user.id, &session.token]
    ).await?;

    JsonResponseBuilder::new(200)
        .response()
}