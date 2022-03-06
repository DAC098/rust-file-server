use futures::Future;
use crate::http::RequestTuple;
use crate::state::AppState;
use crate::http::{error::Result, Request, Response};

use super::auth::{get_session, login_redirect, SessionTuple};
use super::html::{check_if_html_headers, response_index_html_parts};

pub async fn html_wrapper_pass<F, Fut, P>(state: AppState, req: Request, pass: P, handle: F) -> Result<Response>
where
    F: FnOnce(AppState, RequestTuple, SessionTuple, P) -> Fut,
    Fut: Future<Output = Result<Response>>
{
    let session_info = {
        let conn = state.db.pool.get().await?;
        let session_check = get_session(req.headers(), &*conn).await;

        if check_if_html_headers(req.headers())? {
            return match session_check {
                Ok(_) => response_index_html_parts(state.template),
                Err(_) => login_redirect(req.uri())
            }
        }

        session_check?
    };

    handle(state, req.into_parts(), session_info, pass).await
}

pub async fn html_wrapper<F, Fut>(state: AppState, req: Request, handle: F) -> Result<Response>
where
    F: FnOnce(AppState, RequestTuple, SessionTuple) -> Fut,
    Fut: Future<Output = Result<Response>>
{
    let session_info = {
        let conn = state.db.pool.get().await?;
        let session_check = get_session(req.headers(), &*conn).await;

        if check_if_html_headers(req.headers())? {
            return match session_check {
                Ok(_) => response_index_html_parts(state.template),
                Err(_) => login_redirect(req.uri())
            }
        }

        session_check?
    };

    handle(state, req.into_parts(), session_info).await
}

pub async fn auth_wrapper_pass<F, Fut, P>(state: AppState, req: Request, pass: P, handle: F) -> Result<Response>
where
    F: FnOnce(AppState, RequestTuple, SessionTuple, P) -> Fut,
    Fut: Future<Output = Result<Response>>
{
    let session_info = {
        let conn = state.db.pool.get().await?;
        get_session(req.headers(), &*conn).await?
    };

    handle(state, req.into_parts(), session_info, pass).await
}

pub async fn auth_wrapper<F, Fut>(state: AppState, req: Request, handle: F) -> Result<Response>
where
    F: FnOnce(AppState, RequestTuple, SessionTuple) -> Fut,
    Fut: Future<Output = Result<Response>>
{
    let session_info = {
        let conn = state.db.pool.get().await?;
        get_session(req.headers(), &*conn).await?
    };

    handle(state, req.into_parts(), session_info).await
}