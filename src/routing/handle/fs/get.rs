use tokio::fs::File as TokioFile;
use tokio_util::codec::{FramedRead, BytesCodec};
use hyper::Body;

use crate::components::auth::{require_session, login_redirect};
use crate::components::fs_items::existing_resource;
use crate::components::html::{check_if_html_headers, response_index_html_parts};
use crate::db::record::{FsItem, FsItemType, User};
use crate::db::types::PoolConn;
use crate::http::response::JsonResponseBuilder;
use crate::http::uri::QueryMap;
use crate::http::{Response, Request};
use crate::http::error::{Error, Result};
use crate::http::{mime, response};
use crate::state::AppState;

async fn handle_get_info(_state: &AppState, conn: &PoolConn<'_>, _query_map: QueryMap, user: User, fs_item: FsItem) -> Result<Response> {
    match fs_item.item_type {
        FsItemType::File => {
            JsonResponseBuilder::new(200)
                .set_message("successful")
                .payload_response(fs_item)
        },
        FsItemType::Dir => {
            let dir_items = FsItem::find_dir_contents(
                &**conn,
                &user.id,
                &Some(fs_item.id)
            ).await?;
            let mut fs_item_json = serde_json::to_value(fs_item)?;
            fs_item_json.as_object_mut().unwrap().insert(
                "contents".into(),
                serde_json::to_value(dir_items)?
            );

            JsonResponseBuilder::new(200)
                .set_message("successful")
                .payload_response(fs_item_json)
        },
        FsItemType::Unknown => {
            Err(Error::new(400, "UnknownFsType", "cannot handle requested file system item"))
        }
    }
}

async fn handle_get_download(state: &AppState, conn: &PoolConn<'_>, query_map: QueryMap, _user: User, fs_item: FsItem) -> Result<Response> {
    let mut path = state.storage.directory.clone();
    path.push(&fs_item.directory);
    path.push(&fs_item.basename);

    if !path.exists() {
        if fs_item.item_exists {
            FsItem::update_item_exists(&**conn, &fs_item.id, false).await?;
        }
    } else {
        if !fs_item.item_exists {
            FsItem::update_item_exists(&**conn, &fs_item.id, true).await?;
        }
    }

    match fs_item.item_type {
        FsItemType::File => {
            let mime = mime::mime_type_from_ext(path.extension());
            let mut res = response::build()
                .status(200)
                .header("content-type", mime.to_string());

            if query_map.has_key("attachment") {
                let mut header_value = String::with_capacity(23 + fs_item.basename.len());
                header_value.push_str("attachment; filename=\"");
                header_value.push_str(&fs_item.basename);
                header_value.push('"');

                res = res.header("content-disposition", header_value);
            }

            Ok(res.body(Body::wrap_stream(
                FramedRead::new(TokioFile::open(path).await?, BytesCodec::new())
            ))?)
        },
        FsItemType::Dir => {
            response::okay_text_response()
        },
        FsItemType::Unknown => {
            Err(Error::new(400, "UnknownFSType", "cannot handle requested file system item"))
        }
    }
}

pub async fn handle_get(state: AppState, req: Request) -> Result<Response> {
    let conn = state.db.pool.get().await?;
    let session_tuple = require_session(&*conn, req.headers()).await;

    if check_if_html_headers(req.headers())? {
        return match session_tuple {
            Ok(_) => response_index_html_parts(state.template),
            Err(_) => login_redirect(req.uri())
        }
    }

    let (user, _) = session_tuple?;
    let query_map = QueryMap::new(req.uri());

    let context = String::new();

    if let Some(fs_item) = existing_resource(&*conn, &user, &context).await? {
        if fs_item.users_id != user.id {
            // check permissions
        }

        let mut action = "info";

        if let Some(value) = query_map.get_value_ref("action") {
            if let Some(value) = value {
                action = value.as_str();
            } else {
                return Err(Error::new(400, "NoActionValueSpecified", "the action query was specified but no value was given"))
            }
        }

        match action {
            "info" => handle_get_info(&state, &conn, query_map, user, fs_item).await,
            "download" => handle_get_download(&state, &conn, query_map, user, fs_item).await,
            _ => Err(Error::new(400, "UnknownActionGiven", "the requested action is unknown"))
        }
    } else {
        Err(Error::new(404, "PathNotFound", "requested path was not found"))
    }
}