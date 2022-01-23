use chrono::Utc;
use hyper::{HeaderMap, Uri};
use tokio_postgres::GenericClient;

use crate::{
    http::{error::{Result, Error}, cookie::get_cookie_map, Response, response::redirect_response},
    db::record::{User, UserSession}
};

pub enum RetrieveSession {
    NoSessionId,
    InvalidSessionId,
    SessionNotFound,
    SessionEnded,
    Success(User)
}

impl RetrieveSession {

    pub async fn get(
        headers: &HeaderMap,
        conn: &impl GenericClient
    ) -> Result<RetrieveSession> {
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
                            Ok(RetrieveSession::SessionEnded)
                        } else {
                            Ok(RetrieveSession::Success(
                                User::find_id(conn, &session.users_id).await?.unwrap()
                            ))
                        }
                    } else {
                        Ok(RetrieveSession::SessionNotFound)
                    }
                } else {
                    Ok(RetrieveSession::InvalidSessionId)
                }
            } else {
                Ok(RetrieveSession::NoSessionId)
            }
        }
    }

    pub fn try_into_user(self) -> Result<User> {
        match self {
            RetrieveSession::NoSessionId => Err(Error {
                status: 400,
                name: "NoSessionIdGiven".into(),
                msg: "no session id was given".into(),
                source: None
            }),
            RetrieveSession::InvalidSessionId => Err(Error {
                status: 400,
                name: "InvalidSessionId".into(),
                msg: "given session id cannot be parsed".into(),
                source: None
            }),
            RetrieveSession::SessionNotFound => Err(Error {
                status: 404,
                name: "SessionNotFound".into(),
                msg: "given session id cannot be found".into(),
                source: None
            }),
            RetrieveSession::SessionEnded => Err(Error {
                status: 401,
                name: "SessionEnded".into(),
                msg: "this session has been dropped or expired".into(),
                source: None
            }),
            RetrieveSession::Success(user) => Ok(user)
        }
    }

    pub fn successful(&self) -> bool {
        match self {
            RetrieveSession::Success(_) => true,
            _ => false
        }
    }
}

pub fn login_redirect(current: &Uri) -> Result<Response> {
    let redirect_path = format!("/auth/session?jump_to={}", urlencoding::encode(
        if let Some(pq) = current.path_and_query() {
            pq.as_str()
        } else {
            current.path()
        }
    ));

    redirect_response(&redirect_path)
}