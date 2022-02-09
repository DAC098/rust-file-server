use std::collections::HashSet;
use std::io::ErrorKind;

use chrono::Utc;
use futures::{pin_mut, TryStreamExt};
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use tokio::fs::{File as TokioFile, create_dir, remove_file, remove_dir};
use tokio_util::codec::{FramedRead, BytesCodec};
use hyper::Body;

use crate::components::auth::{login_redirect, get_session};
use crate::components::fs_items::{new_resource, existing_resource};
use crate::components::html::{check_if_html_headers, response_index_html_parts};
use crate::db::record::{FsItem, FsItemType, User};
use crate::db::types::PoolConn;
use crate::http::body::{json_from_body, file_from_body};
use crate::http::uri::QueryMap;
use crate::http::{Request, Response};
use crate::http::{
    error::{Error, Result},
    uri,
    mime,
    response,
};
use crate::state::AppState;
use crate::storage::ArcStorageState;

#[derive(Serialize)]
struct FileItem {
    name: String,
    size: u64,
    created: Option<i64>,
    modified: Option<i64>,
    data: JsonValue
}

#[derive(Serialize)]
struct FileRenderData {
    current_path: String,
    display_path: String,
    prev_path: String,
    item: FileItem
}

async fn handle_get_info(_state: &AppState<'_>, conn: &PoolConn<'_>, _query_map: QueryMap, user: User, fs_item: FsItem) -> Result<Response> {
    match fs_item.item_type {
        FsItemType::File => {
            let json = json!({
                "message": "successful",
                "payload": fs_item
            });
        
            response::json_response(200, &json)
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

            let json = json!({
                "message": "successful",
                "payload": fs_item_json
            });

            response::json_response(200, &json)
        },
        FsItemType::Unknown => {
            Err(Error {
                status: 400,
                name: "UnknownFSType".to_owned(),
                msg: "cannot handle requested file system item".to_owned(),
                source: None
            })
        }
    }
}

async fn handle_get_download(state: &AppState<'_>, conn: &PoolConn<'_>, query_map: QueryMap, _user: User, fs_item: FsItem) -> Result<Response> {
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
                res = res.header("content-disposition", format!("attachment; filename=\"{}\"", fs_item.basename));
            }

            Ok(res.body(Body::wrap_stream(
                FramedRead::new(TokioFile::open(path).await?, BytesCodec::new())
            ))?)
        },
        FsItemType::Dir => {
            response::okay_text_response()
        },
        FsItemType::Unknown => {
            Err(Error {
                status: 400,
                name: "UnknownFSType".to_owned(),
                msg: "cannot handle requested file system item".to_owned(),
                source: None
            })
        }
    }
}

pub async fn handle_get(state: AppState<'_>, req: Request, context: String) -> Result<Response> {
    let (head, _) = req.into_parts();
    let conn = state.db.pool.get().await?;
    let user = {
        let session_check = get_session(&head.headers, &*conn).await;

        if check_if_html_headers(&head.headers)? {
            return match session_check {
                Ok(_) => response_index_html_parts(state.template),
                Err(_) => login_redirect(&head.uri)
            }
        }

        let (user, _session) = session_check?;
        user
    };

    if let Some(fs_item) = existing_resource(&*conn, &user, &context).await? {
        let query_map = uri::QueryMap::new(&head.uri);
        let mut action = "info";

        if let Some(value) = query_map.get_value("action") {
            if let Some(value) = value {
                action = value.as_str();
            } else {
                return Err(Error {
                    status: 400,
                    name: "NoActionValueSpecified".into(),
                    msg: "the action query was specified but no value was given".into(),
                    source: None
                })
            }
        }

        match action {
            "info" => handle_get_info(&state, &conn, query_map, user, fs_item).await,
            "download" => handle_get_download(&state, &conn, query_map, user, fs_item).await,
            _ => Err(Error {
                status: 400,
                name: "UnknownActionGiven".into(),
                msg: "the requested action is unknown".into(),
                source: None
            })
        }
    } else {
        Err(Error {
            status: 404,
            name: "PathNotFound".to_owned(),
            msg: "requested path was not found".to_owned(),
            source: None
        })
    }
}

pub async fn handle_post(state: AppState<'_>, req: Request, context: String) -> Result<Response> {
    let (head, body) = req.into_parts();
    let mut conn = state.db.pool.get().await?;
    let (user, _) = get_session(&head.headers, &*conn).await?;
    let (parent, basename) = new_resource(&*conn, &user, &context).await?;

    if let Some(fs_parent) = parent {
        let mut post_type = "file";
        let mut override_existing = false;
        let post_path = {
            let mut rtn = state.storage.directory.clone();

            if !fs_parent.is_root {
                rtn.push(&fs_parent.directory);
            }

            rtn.push(&fs_parent.basename);
            rtn.push(&basename);
            rtn
        };

        for (key, value) in uri::query_iter(&head.uri) {
            if key == "type" {
                post_type = value.unwrap_or("noop");
            } else if key == "override" {
                if let Some(value) = value {
                    override_existing = value == "1";
                } else {
                    override_existing = true;
                }
            }
        }

        let fs_type: FsItemType = post_type.into();

        if fs_type == FsItemType::Unknown {
            return Err(Error {
                status: 400,
                name: "UnknownType".into(),
                msg: format!("the given type is not valid. expect file or dir, given: \"{}\"", post_type).into(),
                source: None
            });
        }

        let transaction = conn.transaction().await?;

        let mut rtn_record = if let Some(mut record) = FsItem::find_basename_with_parent(
            &transaction,
            &fs_parent.id,
            &basename
        ).await? {
            if !override_existing {
                return Err(Error {
                    status: 400,
                    name: "FsItemAlreadyExists".into(),
                    msg: "the requested item already exists in the system".into(),
                    source: None
                });
            } else if record.item_type == FsItemType::Dir && fs_type == FsItemType::File {
                return Err(Error {
                    status: 400,
                    name: "CannotOverwriteDirectory".into(),
                    msg: "you cannot overwrite a directory with a file. delete the directory first".into(),
                    source: None
                });
            } else if record.item_type == FsItemType::File && fs_type == FsItemType::Dir {
                return Err(Error {
                    status: 400,
                    name: "CannotOverwriteFile".into(),
                    msg: "you cannot overwrite a file with a directory. delete the file first".into(),
                    source: None
                });
            }

            record.modified = Some(Utc::now());

            transaction.execute(
                "update fs_items set item_exists = true, modified = $2 where id = $1",
                &[&record.id, &record.modified]
            ).await?;

            record
        } else {
            if post_path.exists() {
                return Err(Error {
                    status: 500,
                    name: "DatabaseFileSystemMismatch".into(),
                    msg: "a file system item exists but there is no record of it".into(),
                    source: None
                });
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

        response::json_payload_response(200, rtn_record)
    } else {
        Err(Error {
            status: 404,
            name: "PathNotFound".to_owned(),
            msg: "requested path was not found".to_owned(),
            source: None
        })
    }
}

pub async fn handle_delete(state: AppState<'_>, req: Request, context: String) -> Result<Response> {
    let (head, _) = req.into_parts();
    let mut conn = state.db.pool.get().await?;
    let (user, _) = get_session(&head.headers, &*conn).await?;

    if let Some(fs_item) = existing_resource(&*conn, &user, &context).await? {
        if fs_item.is_root {
            return Err(Error {
                status: 400,
                name: "CannotDeleteRoot".into(),
                msg: "you cannot delete your root directory".into(),
                source: None
            });
        }

        let mut fs_path = state.storage.directory.clone();
        fs_path.push(&fs_item.directory);
        fs_path.push(&fs_item.basename);

        let transaction = conn.transaction().await?;

        if fs_item.item_type == FsItemType::File {
            transaction.execute(
                "delete from fs_items where id = $1",
                &[&fs_item.id]
            ).await?;

            match remove_file(fs_path).await {
                Ok(()) => {},
                Err(error) => {
                    match error.kind() {
                        ErrorKind::NotFound => {},
                        _ => {
                            return Err(error.into())
                        }
                    }
                }
            };
        } else {
            let row_stream = transaction.query_raw(
                "\
                with recursive dir_tree as ( \
                    select fs_root.id, \
                           fs_root.item_type, \
                           fs_root.parent, \
                           fs_root.directory, \
                           fs_root.basename, \
                           1 as level \
                    from fs_items fs_root \
                    where id = $1 \
                    union \
                    select fs_contents.id, \
                           fs_contents.item_type, \
                           fs_contents.parent, \
                           fs_contents.directory, \
                           fs_contents.basename, \
                           dir_tree.level + 1 as level \
                    from fs_items fs_contents \
                    inner join dir_tree on dir_tree.id = fs_contents.parent \
                ) \
                select * \
                from dir_tree \
                order by level desc, \
                         parent, \
                         item_type, \
                         basename",
                &[&fs_item.id]
            ).await?;

            pin_mut!(row_stream);

            // let mut is_directory = Vec::<i64>::new();
            // let mut not_directory = Vec::<i64>::new();
            let mut dont_delete = HashSet::<i64>::new();
            let mut marked_delete = Vec::<i64>::new();

            while let Some(row) = row_stream.try_next().await? {
                let row_id: i64 = row.get(0);
                let row_parent: i64 = row.get(2);
                let row_item_type: FsItemType = row.get::<usize, i16>(1).into();
                let row_directory: String = row.get(3);
                let row_basename: String = row.get(4);

                if dont_delete.contains(&row_id) {
                    dont_delete.insert(row_parent);
                    continue;
                }

                let mut record_path = state.storage.directory.clone();
                record_path.push(&row_directory);
                record_path.push(&row_basename);

                match row_item_type {
                    FsItemType::File => {
                        match remove_file(record_path).await {
                            Ok(()) => {
                                marked_delete.push(row_id);
                            },
                            Err(error) => {
                                match error.kind() {
                                    // ErrorKind::IsADirectory => {
                                    //     is_directory.push(row_id);
                                    // },
                                    ErrorKind::NotFound => {
                                        marked_delete.push(row_id);
                                    },
                                    ErrorKind::PermissionDenied => {
                                        dont_delete.insert(row_parent);
                                    },
                                    _ => {
                                        return Err(error.into());
                                    }
                                }
                            }
                        }
                    },
                    FsItemType::Dir => {
                        match remove_dir(record_path).await {
                            Ok(()) => marked_delete.push(row.get(0)),
                            Err(error) => {
                                match error.kind() {
                                    // ErrorKind::NotADirectory => {
                                    //     not_directory.push(row_id);
                                    // },
                                    ErrorKind::NotFound => {
                                        marked_delete.push(row_id);
                                    },
                                    ErrorKind::PermissionDenied => {
                                        dont_delete.insert(row_parent);
                                    },
                                    _ => {
                                        return Err(error.into());
                                    }
                                }
                            }
                        };
                    },
                    _ => {}
                }
            }

            transaction.execute(
                "delete from fs_items where id = any(($1))",
                &[&marked_delete]
            ).await?;
        }

        transaction.commit().await?;

        response::json_okay_response(200)
    } else {
        Err(Error {
            status: 404,
            name: "PathNotFound".to_owned(),
            msg: "requested path was not found".to_owned(),
            source: None
        })
    }
}

async fn handle_put_upload_action(storage: ArcStorageState, conn: PoolConn<'_>, mut fs_item: FsItem, body: Body) -> Result<Response> {
    if fs_item.is_root {
        return Err(Error {
            status: 400,
            name: "CannotPutRoot".into(),
            msg: "you cannot update your root directory".into(),
            source: None
        });
    }

    let file_path = {
        let mut path = storage.directory.clone();
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

    response::json_payload_response(200, fs_item)
}

async fn handle_put_user_data_action(conn: PoolConn<'_>, mut fs_item: FsItem, body: Body) -> Result<Response> {
    let json: JsonValue = json_from_body(body).await?;

    conn.execute(
        "update fs_items set user_data = $2 where id = $1",
        &[&fs_item.id, &json]
    ).await?;

    fs_item.user_data = json;

    response::json_payload_response(200, fs_item)
}

pub async fn handle_put(state: AppState<'_>, req: Request, context: String) -> Result<Response> {
    let (head, body) = req.into_parts();
    let conn = state.db.pool.get().await?;
    let (user, _) = get_session(&head.headers, &*conn).await?;

    if let Some(fs_item) = existing_resource(&*conn, &user, &context).await? {
        let query_map = uri::QueryMap::new(&head.uri);
        let default_action = "upload".to_owned();
        let action = if let Some(action) = query_map.get_value("action") {
            if let Some(action_value) = action {
                action_value
            } else {
                &default_action
            }
        } else {
            &default_action
        };

        match action.as_str() {
            "upload" => {
                if fs_item.item_type == FsItemType::Dir {
                    Err(Error {
                        status: 400,
                        name: "InvalidAction".into(),
                        msg: "cannot upload a file as a directory".into(),
                        source: None
                    })
                } else {
                    handle_put_upload_action(state.storage, conn, fs_item, body).await
                }
            },
            "user_data" => handle_put_user_data_action(conn, fs_item, body).await,
            _ => Err(Error {
                status: 400,
                name: "UnknownAction".into(),
                msg: format!("requested action is unknown: \"{}\"", *action),
                source: None
            })
        }
    } else {
        Err(Error {
            status: 404,
            name: "PathNotFound".to_owned(),
            msg: "requested path was not found".to_owned(),
            source: None
        })
    }
}