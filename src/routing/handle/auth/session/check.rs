use crate::{
    http::{
        Request, 
        error::Result,
        Response,
        response::JsonResponseBuilder
    }, 
    components::auth::get_session, state::AppState
};

pub async fn handle_get(state: AppState, req: Request) -> Result<Response> {
    let (head, _) = req.into_parts();
    let conn = state.db.pool.get().await?;
    let session_check = get_session(&head.headers, &*conn).await;

    JsonResponseBuilder::new(200)
        .payload_response(session_check.is_ok())
}