use chrono::Utc;
use serde_json::json;
use tokio::fs::create_dir;

use crate::components::auth::require_session;
use crate::components::fs_items::new_resource;
use crate::db::record::{FsItem, FsItemType};
use crate::event;
use crate::http::body::file_from_body;
use crate::http::response::JsonResponseBuilder;
use crate::http::uri::QueryMap;
use crate::http::{Response, Request};
use crate::http::error::{Error, Result};
use crate::state::AppState;

pub async fn handle_post(state: AppState, req: Request) -> Result<Response> {
    let (head, body) = req.into_parts();
    let mut conn = state.db.pool.get().await?;
    let (user, _) = require_session(&*conn, &head.headers).await?;

    let context = String::new();

    let (parent, basename) = new_resource(&*conn, &user, &context).await?;

    if let Some(fs_parent) = parent {
        if fs_parent.users_id != user.id {
            // check permissions
        }

        let query_map = QueryMap::new(&head.uri);
        let post_type = if let Some(key_value) = query_map.get_value("type") {
            if let Some(existing) = key_value {
                existing
            } else {
                "".to_owned()
            }
        } else {
            "file".to_owned()
        };
        let override_existing = if let Some(key_value) = query_map.get_value_ref("override") {
            if let Some(existing) = key_value {
                existing == "1"
            } else {
                true
            }
        } else {
            false
        };
        let post_path = {
            let mut rtn = state.storage.directory.clone();

            if !fs_parent.is_root {
                rtn.push(&fs_parent.directory);
            }

            rtn.push(&fs_parent.basename);
            rtn.push(&basename);
            rtn
        };

        let fs_type = FsItemType::from(post_type.as_str());

        if fs_type == FsItemType::Unknown {
            return Err(Error::new(
                400, 
                "UnknownType", 
                format!("the given type is not valid. expect file or dir, given: \"{}\"", post_type)
            ));
        }

        let updated: bool;
        let transaction = conn.transaction().await?;

        let mut rtn_record = if let Some(mut record) = FsItem::find_basename_with_parent(
            &transaction,
            &fs_parent.id,
            &basename
        ).await? {
            if !override_existing {
                return Err(Error::new(400, "FsItemAlreadyExists", "the requested item already exists in the system"));
            } else if record.item_type == FsItemType::Dir && fs_type == FsItemType::File {
                return Err(Error::new(400, "CannotOverwriteDirectory", "you cannot overwrite a directory with a file. delete the directory first"));
            } else if record.item_type == FsItemType::File && fs_type == FsItemType::Dir {
                return Err(Error::new(400, "CannotOverwriteFile", "you cannot overwrite a file with a directory. delete the file first"));
            }

            record.modified = Some(Utc::now());

            transaction.execute(
                "update fs_items set item_exists = true, modified = $2 where id = $1",
                &[&record.id, &record.modified]
            ).await?;

            updated = true;
            record
        } else {
            if post_path.exists() {
                return Err(Error::new(500, "DatabaseFileSystemMismatch", "a file system item exists but there is no record of it"));
            }

            let directory = if fs_parent.is_root {
                fs_parent.basename
            } else {
                let mut rtn = String::with_capacity(fs_parent.directory.len() + fs_parent.basename.len() + 1);
                rtn.push_str(&fs_parent.directory);
                rtn.push('/');
                rtn.push_str(&fs_parent.basename);
                rtn
            };

            let record = FsItem {
                id: state.snowflakes.fs_items.next_id().await?,
                item_type: fs_type,
                parent: Some(fs_parent.id),
                users_id: user.id,
                directory,
                basename,
                item_size: 0,
                created: Utc::now(),
                modified: None,
                item_exists: true,
                user_data: json!({}),
                is_root: false
            };

            record.create(&transaction).await?;

            updated = false;
            record
        };

        rtn_record.item_size = match &rtn_record.item_type {
            FsItemType::File => {
                let (_, size) = file_from_body(&post_path, false, body).await?;

                size as i64
            },
            FsItemType::Dir => {
                create_dir(&post_path).await?;
                0
            },
            _ => {0}
        };

        {
            transaction.execute(
                "update fs_items set item_size = $2 where id = $1",
                &[&rtn_record.id, &rtn_record.item_size]
            ).await?;
        }

        transaction.commit().await?;

        if updated {
            state.offload.spawn(event::trigger_fs_item_updated(
                &state, 
                rtn_record.clone()
            ));
        } else {
            state.offload.spawn(event::trigger_fs_item_created(
                &state, 
                rtn_record.clone()
            ));
        }

        JsonResponseBuilder::new(200)
            .payload_response(rtn_record)
    } else {
        Err(Error::new(404, "PathNotFound", "requested path was not found"))
    }
}