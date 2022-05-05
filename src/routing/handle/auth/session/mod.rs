use argon2::verify_encoded;
use hyper::{Body, header::SET_COOKIE};
use serde::Deserialize;
use tokio_postgres::GenericClient;

pub mod session_id;
pub mod check;

use crate::{
    http::{
        Request,
        error::Result,
        error::Error,
        Response,
        response::{redirect_response, JsonResponseBuilder},
        cookie::{get_cookie_map, SetCookie, SameSite},
        body::json_from_body
    }, 
    db::record::UserSession, components::{html::{response_index_html_parts, check_if_html_headers}, auth::get_session}, state::AppState
};

#[derive(Deserialize)]
struct LoginJson {
    username: String,
    password: String,
    totp: Option<String>
}

async fn create_session(conn: &impl GenericClient, body: Body,) -> Result<Response> {
    let login_json: LoginJson = json_from_body(body).await?;

    if let Some(user_record) = conn.query_opt(
        "select hash, id from users where username = $1",
        &[&login_json.username]
    ).await? {
        if !verify_encoded(user_record.get(0), login_json.password.as_bytes())? {
            return Err(Error::new(401, "InvalidLogin", "invalid password given"));
        }

        let session_duration = chrono::Duration::days(7);
        let user_id: i64 = user_record.get(1);
        let token = uuid::Uuid::new_v4();
        let issued_on = chrono::Utc::now();
        let expires = issued_on.clone()
            .checked_add_signed(session_duration.clone())
            .ok_or(Error::new(500, "ServerError", "server error when creating user session"))?;

        conn.execute(
            "\
            insert into user_sessions (users_id, token, issued_on, expires) values \
            ($1, $2, $3, $4)",
            &[&user_id, &token, &issued_on, &expires]
        ).await?;

        let mut session_cookie = SetCookie::new("session_id".into(), token.to_string());
        session_cookie.path = Some("/".into());
        session_cookie.max_age = Some(session_duration);
        session_cookie.same_site = Some(SameSite::Strict);

        JsonResponseBuilder::new(200)
            .add_header(SET_COOKIE, session_cookie)
            .response()
    } else {
        Err(Error::new(404, "UsernameNotFound", "requested username was not found"))
    }
}

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
        let (user, _user_session) = session_check?;

        let session_list = UserSession::find_users_id(&*conn, &user.id).await?;

        JsonResponseBuilder::new(200)
            .payload_response(session_list)
    }
}

pub async fn handle_post(state: AppState, req: Request) -> Result<Response> {
    let (head, body) = req.into_parts();
    let mut conn = state.db.pool.get().await?;

    if let Some(_auth) = head.headers.get("authorization") {
        // do something
        return Err(Error::new(401, "IncorrectLoginMethod", "only non-bot accounts can login using this method"));
    } else {
        let cookies = get_cookie_map(&head.headers);
        let session_id_key = "session_id".to_owned();

        if let Some(list) = cookies.get(&session_id_key) {
            if let Ok(session_id) = &list[0].parse::<uuid::Uuid>() {
                if let Some(_session) = UserSession::find_token(&*conn, session_id).await? {
                    let transaction = conn.transaction().await?;
                    transaction.execute(
                        "update user_sessions set dropped = true where token = $1",
                        &[session_id]
                    ).await?;
                    
                    let res = create_session(&transaction, body).await?;

                    transaction.commit().await?;

                    return Ok(res);
                }
            } else {
                return Err(Error::new(400, "InvalidSessionId", "given session id cannot be parsed as an integer"));
            }
        }
    }

    create_session(&*conn, body).await
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
    
                    let mut session_cookie = SetCookie::new("session_id".into(), session_id.to_string());
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