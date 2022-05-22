use argon2::verify_encoded;
use hyper::{Body, header::SET_COOKIE};
use serde::Deserialize;
use tokio_postgres::GenericClient;

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
    db::record::{UserSession, User},
    components::{
        html::{
            response_index_html_parts,
            check_if_html_headers
        }, 
        auth::{verify_totp_code, require_session}
    }, 
    state::AppState
};

#[derive(Deserialize)]
struct LoginJson {
    username: String,
    password: String,
    totp: Option<String>
}

async fn create_session(conn: &impl GenericClient, body: Body,) -> Result<Response> {
    let login_json: LoginJson = json_from_body(body).await?;

    if let Some(user) = User::find_username(conn, &login_json.username).await? {
        if !verify_encoded(&user.hash, login_json.password.as_bytes())? {
            return Err(Error::new(401, "InvalidLogin", "invalid password given"));
        }

        if user.totp_enabled {
            if let Some(code) = login_json.totp {
                verify_totp_code(&user, code)?;
            } else {
                return Err(Error::new(400, "MissingTOTP", "requires totp code"));
            }
        }

        let session_duration = UserSession::default_duration();
        let session_record = UserSession::new(user.id.clone(), &session_duration)?;
        session_record.insert(conn).await?;

        let mut session_cookie = SetCookie::new("session_id", session_record.token.to_string());
        session_cookie.path = Some("/".into());
        session_cookie.max_age = Some(session_duration);
        session_cookie.same_site = Some(SameSite::Strict);
        session_cookie.http_only = true;

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
    let session_check = require_session(&*conn, &head.headers).await;

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
                }
            } else {
                return Err(Error::new(400, "InvalidSessionId", "given session id cannot be parsed"))
            }
        }
    }

    let mut session_cookie = SetCookie::new("session_id", "");
    session_cookie.max_age = Some(chrono::Duration::seconds(0));
    session_cookie.same_site = Some(SameSite::Strict);

    JsonResponseBuilder::new(200)
        .add_header(SET_COOKIE, session_cookie)
        .response()
}