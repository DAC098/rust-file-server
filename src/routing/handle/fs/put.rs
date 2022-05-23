use chrono::Utc;
use serde_json::Value as JsonValue;
use hyper::Body;

use crate::components::auth::require_session;
use crate::components::fs_items::{existing_resource, SearchOptions};
use crate::db::record::{FsItem, FsItemType};
use crate::db::types::PoolConn;
use crate::event;
use crate::http::body::{json_from_body, file_from_body};
use crate::http::response::JsonResponseBuilder;
use crate::http::{Response, Request};
use crate::http::error::{Error, Result};
use crate::http::uri;
use crate::routing::Params;
use crate::state::AppState;

async fn handle_put_upload_action(state: &AppState, conn: PoolConn<'_>, mut fs_item: FsItem, body: Body) -> Result<Response> {
    if fs_item.is_root {
        return Err(Error::new(400, "CannotPutRoot", "you cannot update your root directory"));
    }

    let file_path = {
        let mut path = state.storage.directory.clone();
        path.push(&fs_item.directory);
        path.push(&fs_item.basename);
        path
    };

    let (_, size) = file_from_body(&file_path, true, body).await?;

    {
        let item_size = size as i64;
        let modified = Utc::now();

        conn.execute(
            "\
            update fs_item \
            set modified = $2, \
                item_size = $3, \
                item_exists = true \
            where $1",
            &[&fs_item.id, &modified, &item_size]
        ).await?;

        fs_item.modified = Some(modified);
    }

    state.offload.spawn(event::trigger_fs_item_updated(
        state,
        fs_item.clone()
    ));

    JsonResponseBuilder::new(200)
        .payload_response(fs_item)
}

async fn handle_put_user_data_action(state: &AppState, conn: PoolConn<'_>, mut fs_item: FsItem, body: Body) -> Result<Response> {
    let json: JsonValue = json_from_body(body).await?;

    conn.execute(
        "update fs_items set user_data = $2 where id = $1",
        &[&fs_item.id, &json]
    ).await?;

    fs_item.user_data = json;

    state.offload.spawn(event::trigger_fs_item_updated(
        state, 
        fs_item.clone()
    ));

    JsonResponseBuilder::new(200)
        .payload_response(fs_item)
}

pub async fn handle_put(state: AppState, req: Request) -> Result<Response> {
    let (mut head, body) = req.into_parts();
    let params = head.extensions.remove::<Params>().unwrap();
    let conn = state.db.pool.get().await?;

    let (user, _) = require_session(&*conn, &head.headers).await?;
    let query_map = uri::QueryMap::new(&head.uri);
    let context = params.get_value_ref("context").unwrap();
    let mut search_options = SearchOptions::new(user.id);
    search_options.pull_from_query_map(&query_map)?;

    if let Some(fs_item) = existing_resource(&*conn, context, search_options).await? {
        if fs_item.users_id != user.id {
            // permissions check
        }

        let query_map = uri::QueryMap::new(&head.uri);
        let action = if let Some(action) = query_map.get_value("action") {
            if let Some(action_value) = action {
                action_value
            } else {
                "upload".into()
            }
        } else {
            "upload".into()
        };

        match action.as_str() {
            "upload" => {
                if fs_item.item_type == FsItemType::Dir {
                    Err(Error::new(400, "InvalidAction", "cannot upload a file as a directory"))
                } else {
                    handle_put_upload_action(&state, conn, fs_item, body).await
                }
            },
            "user_data" => handle_put_user_data_action(&state, conn, fs_item, body).await,
            _ => Err(Error::new(400, "UnknownAction", format!("requested action is unknown: \"{}\"", action)))
        }
    } else {
        Err(Error::new(404, "PathNotFound", "requested path was not found"))
    }
}