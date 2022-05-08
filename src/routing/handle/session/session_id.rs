use hyper::header::SET_COOKIE;

use crate::{
    http::{
        Request, 
        error::Result,
        error::Error,
        Response,
        response::{redirect_response, JsonResponseBuilder}, 
        cookie::{get_cookie_map, SetCookie, SameSite}
    }, 
    db::record::UserSession, components::{html::{response_index_html_parts, check_if_html_headers}, auth::get_session}, state::AppState
};

pub async fn handle_get(state: AppState, req: Request) -> Result<Response> {
    let (head, _) = req.into_parts();
    let conn = state.db.pool.get().await?;
    let session_check = get_session(&head.headers, &*conn).await;

    if check_if_html_headers(&head.headers)? {
        match session_check {
            Ok(_) => redirect_response("/auth/session"),
            Err(_) => response_index_html_parts(state.template)
        }
    } else {
        session_check?;

        JsonResponseBuilder::new(200)
            .set_message("noop")
            .response()
    }
}

pub async fn handle_delete(state: AppState, req: Request) -> Result<Response> {
    let (head, _) = req.into_parts();
    let conn = state.db.pool.get().await?;

    if let Some(_auth) = head.headers.get("authorization") {
        // do something
        return Err(Error::new(401, "IncorrectLoginMethod", "only non-bot accounts can login using this method"));
    } else {
        let cookies = get_cookie_map(&head.headers);
        let session_id_key = "session_id".to_owned();

        if let Some(list) = cookies.get(&session_id_key) {
            if let Ok(session_id) = &list[0].parse::<uuid::Uuid>() {
                if let Some(_session) = UserSession::find_token(&*conn, session_id).await? {
                    conn.execute(
                        "update user_sessions set dropped = true where token = $1",
                        &[session_id]
                    ).await?;
    
                    let mut session_cookie = SetCookie::new("session_id", session_id.to_string());
                    session_cookie.max_age = Some(chrono::Duration::seconds(0));
                    session_cookie.same_site = Some(SameSite::Strict);
    
                    JsonResponseBuilder::new(200)
                        .add_header(SET_COOKIE, session_cookie)
                        .response()
                } else {
                    Err(Error::new(404, "SessionNotFound", "given session id cannot be found"))
                }
            } else {
                Err(Error::new(400, "InvalidSessionId", "given session id cannot be parsed"))
            }
        } else {
            Err(Error::new(400, "NoSessionIdGiven", "no session id was given"))
        }
    }
}