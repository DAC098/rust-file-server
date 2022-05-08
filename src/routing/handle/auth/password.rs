use argon2::verify_encoded;
use futures::future::try_join;
use hyper::header::SET_COOKIE;
use serde::Deserialize;

use crate::{
    http::{
        Request,
        Response,
        error::Result,
        error::Error,
        body::json_from_body,
        cookie::{SetCookie, SameSite},
        response::JsonResponseBuilder
    },
    security::argon::hash_with_default,
    state::AppState,
    components::auth::{get_session, verify_totp_code}
};

#[derive(Deserialize)]
pub struct PasswordJson {
    password: String,
    new_password: String,
    totp: Option<String>,
}

pub async fn handle_post(state: AppState, req: Request) -> Result<Response> {
    let (head, body) = req.into_parts();
    let mut conn = state.db.pool.get().await?;
    let (user, _) = get_session(&head.headers, &*conn).await?;

    let json: PasswordJson = json_from_body(body).await?;

    if !verify_encoded(&user.hash, json.password.as_bytes())? {
        return Err(Error::new(401, "InvalidPassword", "given password is invalid"));
    }

    if user.totp_enabled {
        if let Some(code) = json.totp {
            verify_totp_code(&user, code)?;
        } else {
            return Err(Error::new(400, "MissingTOTP", "requires totp code"));
        }
    }

    let new_hash = hash_with_default(&json.new_password)?;
    let session_duration = chrono::Duration::days(7);
    let token = uuid::Uuid::new_v4();
    let issued_on = chrono::Utc::now();
    let expires = issued_on.clone()
        .checked_add_signed(session_duration.clone())
        .ok_or(Error::new(500, "ServerError", "server error when creating user session"))?;
    let transaction = conn.transaction().await?;

    // these two could be run in parallel
    try_join(
        transaction.execute(
            "update users set hash = $2 where id = $1",
            &[&user.id, &new_hash]
        ),
        transaction.execute(
            "update user_sessions set dropped = true where users_id = $1",
            &[&user.id]
        )
    ).await?;

    transaction.execute(
        "\
        insert into user_sessions (users_id, token, issued_on, expires) values \
        ($1, $2, $3, $4)",
        &[&user.id, &token, &issued_on, &expires]
    ).await?;

    transaction.commit().await?;

    let mut session_cookie = SetCookie::new("session_id".into(), token.to_string());
    session_cookie.max_age = Some(session_duration);
    session_cookie.same_site = Some(SameSite::Strict);

    JsonResponseBuilder::new(200)
        .add_header(SET_COOKIE, session_cookie)
        .response()
}