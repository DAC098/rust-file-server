use argon2::verify_encoded;
use hyper::{Body, header::SET_COOKIE};
use serde::Deserialize;
use serde_json::json;
use tokio_postgres::GenericClient;

use crate::{
    http::{
        Request, 
        error::Result,
        error::Error,
        Response, 
        response::{build, redirect_response, json_response}, 
        cookie::{get_cookie_map, SetCookie, SameSite}, 
        body::json_from_body
    }, 
    db::record::UserSession, components::{html::{response_index_html_parts, check_if_html_headers}, auth::get_session}, state::AppState
};

#[derive(Deserialize)]
struct LoginJson {
    username: String,
    password: String
}

async fn create_session(conn: &impl GenericClient, body: Body,) -> Result<Response> {
    let login_json: LoginJson = json_from_body(body).await?;

    if let Some(user_record) = conn.query_opt(
        "select hash, id from users where username = $1",
        &[&login_json.username]
    ).await? {
        if !verify_encoded(user_record.get(0), login_json.password.as_bytes())? {
            return Err(Error {
                status: 401,
                name: "InvalidLogin".into(),
                msg: "invalid password given".into(),
                source: None
            });
        }

        let session_duration = chrono::Duration::days(7);
        let user_id: i64 = user_record.get(1);
        let token = uuid::Uuid::new_v4();
        let issued_on = chrono::Utc::now();
        let expires = issued_on.clone().checked_add_signed(session_duration.clone()).ok_or(Error {
            status: 500,
            name: "ServerError".into(),
            msg: "server error when creating user session".into(),
            source: None
        })?;

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

        Ok(build()
            .status(200)
            .header(SET_COOKIE, session_cookie)
            .header("content-type", "application/json")
            .body(r#"{"message":"okay"}"#.into())?)
    } else {
        Err(Error {
            status: 404,
            name: "UsernameNotFound".into(),
            msg: "requested username was not found".into(),
            source: None
        })
    }
}

pub async fn handle_get(state: AppState<'_>,req: Request) -> Result<Response> {
    let (head, _) = req.into_parts();
    let conn = state.db.pool.get().await?;
    let session_check = get_session(&head.headers, &*conn).await;

    if check_if_html_headers(&head.headers)? {
        match session_check {
            Ok(_) => redirect_response("/fs/"),
            Err(_) => response_index_html_parts(head)
        }
    } else {
        session_check?;

        let json = json!({"message": "noop"});
        json_response(200, &json)
    }
}

pub async fn handle_post(state: AppState<'_>, req: Request) -> Result<Response> {
    let (head, body) = req.into_parts();
    let mut conn = state.db.pool.get().await?;

    if let Some(_auth) = head.headers.get("authorization") {
        // do something
        return Err(Error {
            status: 401,
            name: "IncorrectLoginMethod".into(),
            msg: "only non-bot accounts can login using this method".into(),
            source: None
        });
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
                return Err(Error {
                    status: 400,
                    name: "InvalidSessionId".into(),
                    msg: "given session id cannot be parsed as an integer".into(),
                    source: None
                });
            }
        }
    }

    create_session(&*conn, body).await
}

pub async fn handle_delete(state: AppState<'_>, req: Request) -> Result<Response> {
    let (head, _) = req.into_parts();
    let conn = state.db.pool.get().await?;

    if let Some(_auth) = head.headers.get("authorization") {
        // do something
        return Err(Error {
            status: 401,
            name: "IncorrectLoginMethod".into(),
            msg: "only non-bot accounts can login using this method".into(),
            source: None
        });
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
    
                    Ok(build()
                        .status(200)
                        .header(SET_COOKIE, session_cookie)
                        .header("content-type", "application/json")
                        .body(r#"{"message":"okay"}"#.into())?)
                } else {
                    Err(Error {
                        status: 404,
                        name: "SessionNotFound".into(),
                        msg: "given session id cannot be found".into(),
                        source: None
                    })
                }
            } else {
                Err(Error {
                    status: 400,
                    name: "InvalidSessionId".into(),
                    msg: "given session id cannot be parsed".into(),
                    source: None
                })
            }
        } else {
            Err(Error {
                status: 400,
                name: "NoSessionIdGiven".into(),
                msg: "no session id was given".into(),
                source: None
            })
        }
    }
}