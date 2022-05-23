use uuid::Uuid;

use crate::{
    http::{
        Request,
        Response,
        error::{Result, Error},
        response::JsonResponseBuilder,
    },
    components::auth::require_session,
    state::AppState, routing::Params
};

pub async fn handle_delete(state: AppState, mut req: Request) -> Result<Response> {
    let params = req.extensions_mut().remove::<Params>().unwrap();
    let conn = state.db.pool.get().await?;
    let (user, session) = require_session(&*conn, req.headers()).await?;
    let token: Uuid;

    if let Some(given) = params.get_value_ref("session_id") {
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