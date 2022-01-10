use std::collections::HashSet;
use std::io::ErrorKind;

use chrono::Utc;
use futures::{StreamExt, pin_mut, TryStreamExt};
use hyper::http::request::Parts;
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use tokio::fs::{File as TokioFile, create_dir, remove_file, remove_dir};
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{FramedRead, BytesCodec};
use hyper::{Body, Uri};

use crate::components::auth::require_user;
use crate::db::record::{FsItem, FsItemType, User};
use crate::db::ArcDBState;
use crate::db::types::PoolConn;
use crate::http::body::json_from_body;
use crate::http::{Request, Response};
use crate::http::{
    error::{Error, Result},
    uri,
    mime,
    response,
};
use crate::snowflakes::IdSnowflakes;
use crate::storage::ArcStorageState;
use crate::components;

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

#[inline]
fn root_strip(uri: &Uri) -> &str {
    uri.path().strip_prefix("/fs/").unwrap()
}

fn get_directory_and_basename(path: &str) -> (String, String) {
    let mut rtn = String::new();
    let mut working = String::new();

    for ch in path.chars() {
        if ch == '/' {
            rtn.push('/');
            rtn.push_str(working.as_str());
            working.clear();
        } else {
            working.push(ch);
        }
    }

    working.shrink_to_fit();

    (rtn, working)
}

pub async fn handle_get(req: Request) -> Result<Response> {
    let (mut head, _) = req.into_parts();
    let stripped_root = root_strip(&head.uri);

    let db = head.extensions.remove::<ArcDBState>().unwrap();
    let conn = db.pool.get().await?;
    let user = components::auth::require_user(&head.headers, &*conn).await?;

    if let Some(fs_item) = FsItem::find_path(&*conn, &user.id, stripped_root).await? {
        let query_map = uri::QueryMap::new(&head.uri);
        let wants_download = query_map.has_key("download");

        match fs_item.item_type {
            FsItemType::File => {
                if wants_download {
                    handle_get_file_download(head, user, fs_item).await
                } else {
                    handle_get_file_info(head, fs_item).await
                }
            },
            FsItemType::Dir => {
                if wants_download {
                    handle_get_dir_download(head, fs_item).await
                } else {
                    handle_get_dir_info(head, conn, user, fs_item).await
                }
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
    } else {
        Err(Error {
            status: 404,
            name: "PathNotFound".to_owned(),
            msg: "requested path was not found".to_owned(),
            source: None
        })
    }
}

async fn handle_get_dir_download(head: Parts, _fs_item: FsItem) -> Result<Response> {
    let storage = head.extensions.get::<ArcStorageState>().unwrap();
    let tmp_file_name_opt =  storage.get_tmp_file("tar");

    if tmp_file_name_opt.is_none() {
        return Err(Error::internal_server_error_no_error());
    }

    let tmp_file_name = tmp_file_name_opt.unwrap();
    let _tmp_file = TokioFile::create(&tmp_file_name).await?;

    response::okay_text_response()
}

async fn handle_get_file_download(head: Parts, user: User, fs_item: FsItem) -> Result<Response> {
    let storage = head.extensions.get::<ArcStorageState>().unwrap();
    let mut path = storage.directory.clone();
    path.push(&user.username);
    path.push(&fs_item.directory);
    path.push(&fs_item.basename);

    let mime = mime::mime_type_from_ext(path.extension());

    Ok(response::build()
        .status(200)
        .header("content-type", mime.to_string())
        .header("content-disposition", format!("attachment; filename=\"{}\"", fs_item.basename))
        .body(Body::wrap_stream(
            FramedRead::new(TokioFile::open(path).await?, BytesCodec::new())
        ))?)
}

async fn handle_get_dir_info(head: Parts, conn: PoolConn<'_>, user: User, fs_item: FsItem) -> Result<Response> {
    let accepting = mime::get_accepting_default(&head.headers, "text/plain")?;
    let dir_items = FsItem::find_dir_contents(&*conn, &user.id, &Some(fs_item.id)).await?;

    for accept in accepting {
        if accept.type_() == "application" {
            if accept.subtype() == "json" {
                return response::json_response(200, &dir_items);
            }
        }
    }

    Ok(response::build()
        .status(400)
        .header("content-type", "text/plain")
        .body("unknown accept header value".into())?)
}

async fn handle_get_file_info(head: Parts, fs_item: FsItem) -> Result<Response> {
    let accepting = mime::get_accepting_default(&head.headers, "text/plain")?;

    for accept in accepting {
        if accept.type_() == "application" {
            if accept.subtype() == "json" {
                return response::json_response(200, &fs_item);
            }
        }
    }

    Err(Error {
        status: 400,
        name: "UnknownAcceptHeader".to_owned(),
        msg: "unknown accept header value".to_owned(),
        source: None
    })
}

pub async fn handle_post(req: Request) -> Result<Response> {
    let (mut head, mut body) = req.into_parts();
    let fs_root = root_strip(&head.uri);

    if fs_root == "" {
        return Err(Error {
            status: 400,
            name: "CannotPostRoot".to_owned(),
            msg: "you cannot post the root directory".to_owned(),
            source: None
        });
    }

    let (mut directory, mut basename) = get_directory_and_basename(fs_root);
    basename = basename.trim().to_owned();

    if basename.is_empty() {
        return Err(Error {
            status: 400,
            name: "InvalidBasename".into(),
            msg: "basename cannot be empty and leading/trailing whitespace will be removed".into(),
            source: None
        });
    }

    let db = head.extensions.remove::<ArcDBState>().unwrap();
    let mut conn = db.pool.get().await?;
    let user = require_user(&head.headers, &*conn).await?;

    if directory.is_empty() {
        directory = user.id.to_string();
    } else {
        directory = format!("{}/{}", user.id.to_string(), directory);
    }

    if let Some(fs_parent) = FsItem::find_path(&*conn, &user.id, &directory.as_str()).await? {
        let mut post_type = "file";
        let mut override_existing = false;
        let post_path = {
            let storage = head.extensions.remove::<ArcStorageState>().unwrap();
            let mut rtn = storage.directory.clone();

            if !fs_parent.is_root {
                rtn.push(&directory);
            }

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

        if let Some(record) = FsItem::find_basename_with_parent(&*conn, &fs_parent.id, &basename).await? {
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
        } else if post_path.exists() {
            if !override_existing {
                return Err(Error {
                    status: 500,
                    name: "DatabaseFileSystemMismatch".into(),
                    msg: "a file system item exists but there is no record of it".into(),
                    source: None
                });
            } else if post_path.is_dir() && fs_type == FsItemType::File {
                // return Err(Error {
                //     status: 400,
                //     name: "CannotOverwriteDirectory".into(),
                //     msg: "you cannot overwrite a directory with a file. delete the directory first".into(),
                //     source: None
                // });
            } else if post_path.is_file() && fs_type == FsItemType::Dir {
                // return Err(Error {
                //     status: 400,
                //     name: "CannotOverwriteFile".into(),
                //     msg: "you cannot overwrite a file with a directory. delete the file first".into(),
                //     source: None
                // });
            }
        }

        let transaction = conn.transaction().await?;

        let new_fs_item = {
            let id = {
                let mut snowflakes = head.extensions.remove::<IdSnowflakes>().unwrap();
                snowflakes.fs_items.next_id().await?
            };
            let created = Utc::now();
            let modified = created.clone();
            let user_data = json!({});
            let parent_id = Some(fs_parent.id);
            let fs_type_ref: i16 = fs_type.clone().into();

            transaction.execute(
                "\
                insert into fs_items (id, item_type, parent, users_id, directory, basename, created, modified, user_data) values \
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[
                    &id,
                    &fs_type_ref,
                    &parent_id, 
                    &user.id, 
                    &directory, 
                    &basename, 
                    &created, 
                    &modified, 
                    &user_data,
                ]
            ).await?;

            FsItem {
                id,
                item_type: fs_type,
                parent: Some(fs_parent.id),
                users_id: user.id,
                directory,
                basename,
                created,
                modified,
                user_data,
                is_root: false
            }
        };

        match &new_fs_item.item_type {
            FsItemType::File => {
                let mut file = TokioFile::create(post_path).await?;

                while let Some(chunk) = body.next().await {
                    let bytes = chunk?;
                    file.write(&bytes).await?;
                }
            },
            FsItemType::Dir => {
                create_dir(&post_path).await?;
            },
            _ => {}
        }

        transaction.commit().await?;

        let json = json!({
            "msg": "successful",
            "payload": new_fs_item
        });

        response::json_response(200, &json)
    } else {
        Err(Error {
            status: 404,
            name: "DirectoryNotFound".into(),
            msg: "the given parent directory was not found".into(),
            source: None
        })
    }
}

pub async fn handle_delete(req: Request) -> Result<Response> {
    let (mut head, _) = req.into_parts();
    let fs_path = root_strip(&head.uri);
    let db = head.extensions.remove::<ArcDBState>().unwrap();
    let mut conn = db.pool.get().await?;

    let user = require_user(&head.headers, &*conn).await?;
    
    if let Some(fs_item) = FsItem::find_path(&*conn, &user.id, &fs_path).await? {
        if fs_item.is_root {
            return Err(Error {
                status: 400,
                name: "CannotDeleteRoot".into(),
                msg: "you cannot delete your root directory".into(),
                source: None
            });
        }

        let storage = head.extensions.remove::<ArcStorageState>().unwrap();
        let mut fs_path = storage.directory.clone();
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

                let mut record_path = storage.directory.clone();
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

        let rtn_json = json!({
            "message": "successful"
        });
        response::json_response(200, &rtn_json)
    } else {
        Err(Error {
            status: 404,
            name: "FsItemNotFound".into(),
            msg: "the requested item was not found".into(),
            source: None
        })
    }
}

async fn handle_put_upload_action(storage: ArcStorageState, mut conn: PoolConn<'_>, mut fs_item: FsItem, mut body: Body) -> Result<Response> {
    if fs_item.is_root {
        return Err(Error {
            status: 400,
            name: "CannotPutRoot".into(),
            msg: "you cannot update your roo".into(),
            source: None
        });
    }

    let transaction = conn.transaction().await?;
    let modified = Utc::now();

    transaction.execute(
        "update fs_item set modified = $2 where $1",
        &[&fs_item.id, &modified]
    ).await?;

    let file_path = {
        let mut path = storage.directory.clone();
        path.push(&fs_item.directory);
        path.push(&fs_item.basename);
        path
    };
    let mut file = TokioFile::create(&file_path).await?;

    while let Some(chunk) = body.next().await {
        let bytes = chunk?;
        file.write(&bytes).await?;
    }

    transaction.commit().await?;

    fs_item.modified = modified;

    let rtn_json = json!({
        "message": "successful",
        "payload": fs_item
    });
    response::json_response(200, &rtn_json)
}

async fn handle_put_user_data_action(mut conn: PoolConn<'_>, mut fs_item: FsItem, body: Body) -> Result<Response> {
    let json: JsonValue = json_from_body(body).await?;
    let transaction = conn.transaction().await?;

    transaction.query_one(
        "update fs_items set user_data = $2 where id = $1",
        &[&fs_item.id, &json]
    ).await?;

    transaction.commit().await?;

    fs_item.user_data = json;

    let rtn_json = json!({
        "message": "successful",
        "payload": fs_item
    });
    response::json_response(200, &rtn_json)
}

pub async fn handle_put(req: Request) -> Result<Response> {
    let (mut head, body) = req.into_parts();
    let fs_path = root_strip(&head.uri);
    let storage = head.extensions.remove::<ArcStorageState>().unwrap();
    let db = head.extensions.remove::<ArcDBState>().unwrap();
    let conn = db.pool.get().await?;
    let user = require_user(&head.headers, &*conn).await?;

    if let Some(fs_item) = FsItem::find_path(&*conn, &user.id, &fs_path).await? {
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
            "upload" => handle_put_upload_action(storage, conn, fs_item, body).await,
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
            name: "FsItemNotFound".into(),
            msg: "the requested fs item was not found".into(),
            source: None
        })
    }
}