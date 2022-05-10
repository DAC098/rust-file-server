use uuid::Uuid;

use crate::{
    http::{
        error::{Result, Error},
        Response,
        response::JsonResponseBuilder,
        RequestTuple
    },
    components::auth::SessionTuple,
    state::AppState
};

pub async fn handle_delete(state: AppState, (head, _body): RequestTuple, (user, session): SessionTuple) -> Result<Response> {
    let mut path_split = head.uri.path().split('/');
    path_split.next();

    let token: Uuid;

    if let Some(given) = path_split.next() {
        if let Ok(parsed) = given.parse() {
            token = parsed;
        } else {
            return Err(Error::new(400, "InvalidSessionId", "given session id is invalid"))
        }
    } else {
        return Err(Error::new(400, "MissingSessionId", "no session id was given"))
    }

    if token == session.token {
        return Err(Error::new(400, "InvalidSessionId", "cannot delete your active session id"));
    }

    let conn = state.db.pool.get().await?;

    let result = conn.execute(
        "delete from user_session where users_id = $1 and token = $2",
        &[&user.id, &token]
    ).await?;

    if result != 1 {
        Err(Error::new(404, "SessionIdNotFound", "given session id was not found"))
    } else {
        JsonResponseBuilder::new(200)
            .response()
    }
}