use chrono::Utc;
use hyper::{HeaderMap, Uri};
use tokio_postgres::GenericClient;

use crate::{
    http::{error::{Result, Error}, cookie::get_cookie_map, Response, response::redirect_response},
    db::record::{User, UserSession}
};

pub async fn get_session(headers: &HeaderMap, conn: &impl GenericClient) -> Result<(User, UserSession)> {
    if let Some(_auth) = headers.get("authorization") {
        // do something
        return Err(Error {
            status: 400,
            name: "NotImplemented".into(),
            msg: "bot sessions are not currently enabled".into(),
            source: None
        });
    } else {
        let cookies = get_cookie_map(headers);
        let session_id_key = "session_id".to_owned();

        if let Some(list) = cookies.get(&session_id_key) {
            if let Ok(session_id) = &list[0].parse::<uuid::Uuid>() {
                if let Some(session) = UserSession::find_token(&*conn, session_id).await? {
                    let now = Utc::now();

                    if session.dropped || session.expires < now {
                        Err(Error {
                            status: 401,
                            name: "SessionEnded".into(),
                            msg: "this session has been dropped or expired".into(),
                            source: None
                        })
                    } else {
                        Ok((
                            User::find_id(conn, &session.users_id).await?.unwrap(),
                            session
                        ))
                    }
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

pub fn login_redirect_path(path: &str) -> Result<Response> {
    let redirect_path = format!("/auth/session?jump_to={}", urlencoding::encode(path));
    redirect_response(&redirect_path)
}

pub fn login_redirect(current: &Uri) -> Result<Response> {
    let redirect_path = if let Some(pq) = current.path_and_query() {
        pq.as_str()
    } else {
        current.path()
    };

    login_redirect_path(&redirect_path)
}