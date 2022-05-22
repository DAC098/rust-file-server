use std::option::Option;

use chrono::Utc;
use hyper::{HeaderMap, Uri};
use tokio_postgres::GenericClient;
use ring::hmac;
use lib::time::unix_epoch_sec_now;
use otp::totp;

use crate::{
    http::{error::{Result, Error}, cookie::get_cookie_map, Response, response::redirect_response},
    db::record::{User, UserSession, TotpAlgorithm}
};

/*
pub struct SessionTuple (User, UserSession);

impl SessionTuple {

    pub fn try_from(req: &mut Request) -> Result<SessionTuple> {
        if let Some(ext) = req.extensions_mut().remove::<Option<SessionTuple>>() {
            if let Some(tuple) = ext {
                Ok(tuple)
            } else {
                Err(Error::new(401, "NoSession", "no session is available"))
            }
        } else {
            panic!("no session tuple is present in the request extension");
        }
    }

    pub fn try_from_user(req: &mut Request) -> Result<User> {
        if let Some(ext) = req.extensions_mut().remove::<Option<SessionTuple>>() {
            if let Some(tuple) = ext {
                Ok(tuple.0)
            } else {
                Err(Error::new(401, "NoSession", "no session is available"))
            }
        } else {
            panic!("no session tuple is present in the request extension");
        }
    }

    pub fn try_from_session(req: &mut Request) -> Result<UserSession> {
        if let Some(ext) = req.extensions_mut().remove::<Option<SessionTuple>>() {
            if let Some(tuple) = ext {
                Ok(tuple.1)
            } else {
                Err(Error::new(401, "NoSession", "no session is available"))
            }
        } else {
            panic!("no session tuple is present in the request extension");
        }
    }

    pub fn option_try_from(req: &mut Request) -> Option<SessionTuple> {
        if let Some(ext) = req.extensions_mut().remove() {
            ext
        } else {
            panic!("no session tuple is present in the request extension");
        }
    }

    pub fn option_try_from_user(req: &mut Request) -> Option<User> {
        if let Some(ext) = req.extensions_mut().remove::<Option<SessionTuple>>() {
            if let Some(tuple) = ext {
                Some(tuple.0)
            } else {
                None
            }
        } else {
            panic!("no session tuple is present in the request extension");
        }
    }

    pub fn option_try_from_user_session(req: &mut Request) -> Option<UserSession> {
        if let Some(ext) = req.extensions_mut().remove::<Option<SessionTuple>>() {
            if let Some(tuple) = ext {
                Some(tuple.1)
            } else {
                None
            }
        } else {
            panic!("no session tuple is present in the request extension");
        }
    }

    pub fn into_tuple(self) -> (User, UserSession) {
        (self.0, self.1)
    }

    pub fn into_user(self) -> User {
        self.0
    }

    pub fn into_session(self) -> UserSession {
        self.1
    }
}
*/

pub async fn get_session(conn: &impl GenericClient, headers: &HeaderMap) -> Result<Option<(User, UserSession)>> {
    if let Some(_auth) = headers.get("authorization") {
        // do something
        return Err(Error::new(400, "NotImplemented", "bot sessions are not currently enabled"));
    } else {
        let cookies = get_cookie_map(headers);
        let session_id_key = "session_id".to_owned();

        if let Some(list) = cookies.get(&session_id_key) {
            if let Ok(session_id) = &list[0].parse::<uuid::Uuid>() {
                if let Some(session) = UserSession::find_token(&*conn, session_id).await? {
                    let now = Utc::now();

                    if session.dropped || session.expires < now {
                        Ok(None)
                    } else {
                        let user = User::find_id(conn, &session.users_id).await?.unwrap();

                        Ok(Some((user, session)))
                    }
                } else {
                    Ok(None)
                }
            } else {
                Err(Error::new(400, "InvalidSessionId", "given session id cannot be parsed"))
            }
        } else {
            Ok(None)
        }
    }
}

pub async fn require_session(conn: &impl GenericClient, headers: &HeaderMap) -> Result<(User, UserSession)> {
    if let Some(tuple) = get_session(conn, headers).await? {
        Ok(tuple)
    } else {
        Err(Error::new(401, "NoSession", "no session is available"))
    }
}

pub fn login_redirect_path(path: &str) -> Result<Response> {
    let redirect_path = format!("/auth/session?jump_to={}", urlencoding::encode(path));
    redirect_response(&redirect_path)
}

pub fn login_redirect(uri: &Uri) -> Result<Response> {
    let redirect_path = if let Some(pq) = uri.path_and_query() {
        pq.as_str()
    } else {
        uri.path()
    };

    login_redirect_path(&redirect_path)
}

pub fn verify_totp_code(user: &User, code: String) -> Result<()> {
    let secret = user.totp_secret.clone()
        .unwrap();
    let step: u64 = user.totp_step.clone()
        .unwrap()
        .into();
    let digits: u32 = user.totp_digits.clone()
        .unwrap()
        .into();
    let now = unix_epoch_sec_now().unwrap();
    let prev = now - step;
    let next = now + step;

    let mut code_len: u32 = 0;

    for ch in code.chars() {
        if !ch.is_ascii_digit() {
            return Err(Error::new(400, "InvalidTOTPCode", "given code is not a valid totp code"));
        } else {
            code_len += 1;
        }
    }

    if code_len != digits {
        return Err(Error::new(400, "InvalidTOTPCode", "given code is not a valid totp code"));
    }

    let algo = match user.totp_algorithm.clone().unwrap() {
        TotpAlgorithm::SHA1 => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
        TotpAlgorithm::SHA256 => hmac::HMAC_SHA256,
        TotpAlgorithm::SHA512 => hmac::HMAC_SHA512
    };

    let prev_code = totp(algo, &secret, digits, step, prev);
    let now_code = totp(algo, &secret, digits, step, now);
    let next_code = totp(algo, &secret, digits, step, next);

    if code == prev_code || code == now_code || code == next_code {
        Ok(())
    } else {
        Err(Error::new(400, "InvalidTOTPCode", "given code is not a valid totp code"))
    }
}