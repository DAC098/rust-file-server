use argon2::verify_encoded;
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
        response::build
    }, 
    db::ArcDBState, 
    components::auth::require_user, 
    security::argon::hash_with_default
};

#[derive(Deserialize)]
pub struct PasswordJson {
    password: String,
    new_password: String
}

pub async fn handle_post(req: Request) -> Result<Response> {
    let (head, body) = req.into_parts();
    let db = head.extensions.get::<ArcDBState>().unwrap();
    let mut conn = db.pool.get().await?;

    let user = require_user(&head.headers, &*conn).await?;
    let hash: String = {
        let res = conn.query_one(
            "select hash from users where id = $1",
            &[&user.id]
        ).await?;

        res.get(0)
    };
    let json: PasswordJson = json_from_body(body).await?;

    if !verify_encoded(hash.as_str(), json.password.as_bytes())? {
        return Err(Error {
            status: 401,
            name: "InvalidPassword".into(),
            msg: "given password is invalid".into(),
            source: None
        });
    }

    let new_hash = hash_with_default(&json.new_password)?;
    let session_duration = chrono::Duration::days(7);
    let token = uuid::Uuid::new_v4();
    let issued_on = chrono::Utc::now();
    let expires = issued_on.clone().checked_add_signed(session_duration.clone()).ok_or(Error {
        status: 500,
        name: "ServerError".into(),
        msg: "server error when creating user session".into(),
        source: None
    })?;
    let transaction = conn.transaction().await?;

    // these two could be run in parallel
    transaction.execute(
        "update users set hash = $2 where id = $1",
        &[&user.id, &new_hash]
    ).await?;
    transaction.execute(
        "update user_sessions set dropped = true where users_id = $1",
        &[&user.id]
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

    Ok(build()
        .status(200)
        .header(SET_COOKIE, session_cookie)
        .header("content-type", "application/json")
        .body(r#"{"message":"okay"}"#.into())?)
}