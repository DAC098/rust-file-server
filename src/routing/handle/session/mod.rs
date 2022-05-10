use crate::{
    http::{
        error::Result,
        Response,
        response::JsonResponseBuilder,
        RequestTuple
    }, 
    db::record::UserSession, 
    components::auth::SessionTuple,
    state::AppState
};

pub mod session_id;

pub async fn handle_get(state: AppState, (_head, _body): RequestTuple, (user, session): SessionTuple) -> Result<Response> {
    let conn = state.db.pool.get().await?;
    let user_sessions = UserSession::find_users_id(
        &*conn,
        &user.id,
        Some(&session.token)
    ).await?;

    JsonResponseBuilder::new(200)
        .payload_response(user_sessions)
}

pub async fn handle_delete(state: AppState, (_head, _body): RequestTuple, (user, session): SessionTuple) -> Result<Response> {
    let conn = state.db.pool.get().await?;

    conn.execute(
        "\
        delete from user_session where users_id = $1 and token != $2",
        &[&user.id, &session.token]
    ).await?;

    JsonResponseBuilder::new(200)
        .response()
}