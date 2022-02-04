use std::{path::PathBuf, fs::Metadata};

use chrono::{DateTime, Utc};
use hyper::Uri;
use serde_json::json;
use tokio::fs::{ReadDir, read_dir, metadata};
use tokio_postgres::GenericClient;

use crate::{state::AppState, http::{Request, Response, error::{Result, Error}, response::json_response}, components::auth::get_session, db::record::{FsItem, FsItemType}, storage::ArcStorageState};

#[inline]
fn root_strip(uri: &Uri) -> &str {
    uri.path().strip_prefix("/sync/").unwrap()
}

fn file_record_path(id: &i64, path: &str) -> String {
    let id_str = id.to_string();

    if path.len() == 0 {
        id_str
    } else {
        let mut rtn = String::with_capacity(id_str.len() + 1 + path.len());
        rtn.push_str(id_str.as_str());
        rtn.push('/');
        rtn.push_str(path);
        rtn
    }
}

struct WorkItem {
    iter: ReadDir,
    id: i64
}

pub async fn handle_put(state: AppState<'_>, req: Request) -> Result<Response> {
    let (head, _) = req.into_parts();
    let mut conn = state.db.pool.get().await?;
    let (user, _) = get_session(&head.headers, &*conn).await?;
    let find_path = file_record_path(&user.id, root_strip(&head.uri));

    if let Some(fs_item) = FsItem::find_path(&*conn, &user.id, &find_path).await? {
        let transaction = conn.transaction().await?;

        if fs_item.item_type == FsItemType::Dir {
            let mut found_ids: Vec<i64> = Vec::with_capacity(1);
            let mut fs_path = state.storage.directory.clone();
            fs_path.push(&fs_item.directory);
            fs_path.push(&fs_item.basename);

            if fs_path.exists() {
                let mut working_queue = Vec::with_capacity(1);
                working_queue.push(WorkItem {
                    iter: read_dir(&fs_path).await?,
                    id: fs_item.id
                });
                found_ids.push(fs_item.id.clone());

                while let Some(mut working) = working_queue.pop() {
                    while let Some(entry) = working.iter.next_entry().await? {
                        let entry_path = entry.path();

                        if entry_path.is_dir() {
                            let id = sync_dir(
                                &state, 
                                &transaction, 
                                &user.id, 
                                &working.id, 
                                &entry_path
                            ).await?;

                            found_ids.push(id.clone());
                            working_queue.push(working);
                            working_queue.push(WorkItem {
                                iter: read_dir(&entry_path).await?,
                                id
                            });

                            break;
                        } else if entry_path.is_file() {
                            let id = sync_file(
                                &state, 
                                &transaction, 
                                &user.id, 
                                &working.id, 
                                &entry_path
                            ).await?;

                            found_ids.push(id);
                        } else {
                            // unknown file type
                        }
                    }
                }

                transaction.execute(
                    "update fs_items set item_exists = true where id = $1",
                    &[&fs_item.id]
                ).await?;
            }

            transaction.execute(
                "\
                with recursive dir_tree as ( \
                    select fs_root.id, \
                           fs_root.parent, \
                           1 as level \
                    from fs_items fs_root \
                    where id = $1 \
                    union \
                    select fs_contents.id, \
                           fs_contents.parent, \
                           dir_tree.level + 1 as level \
                    from fs_items fs_contents \
                    inner join dir_tree on dir_tree.id = fs_contents.parent \
                ) \
                update fs_items \
                set item_exists = false \
                from dir_tree \
                where dir_tree.id = fs_items.id and \
                      dir_tree.id <> all($2)",
                &[&fs_item.id, &found_ids]
            ).await?;
        } else {
            let mut fs_path = state.storage.directory.clone();
            fs_path.push(&fs_item.directory);
            fs_path.push(&fs_item.basename);

            if fs_path.exists() {
                let md = metadata(&fs_path).await?;

                sync_known_file(&transaction, &md, &fs_item).await?;
            } else {
                transaction.execute(
                    "update fs_items set item_exists = false where id = $1",
                    &[&fs_item.id]
                ).await?;
            }
        }

        transaction.commit().await?;

        let json = json!({"message": "okay"});
    
        json_response(200, &json)
    } else {
        Err(Error {
            status: 404,
            name: "PathNotFound".into(),
            msg: "requested path was not found".into(),
            source: None
        })
    }
}

fn path_to_str<'a>(path: &'a PathBuf) -> Result<&'a str> {
    path.to_str().ok_or(Error {
        status: 400,
        name: "NonUtf8Path".into(),
        msg: "encountered a file system path that cannot be converted to utf-8".into(),
        source: None
    })
}

fn get_directory_and_basename(app: &AppState<'_>, fs_path: &PathBuf) -> Result<(String, String)> {
    let storage_str = path_to_str(&app.storage.directory)?;
    let working = path_to_str(fs_path)?;

    Ok(lib::string::get_directory_and_basename(match working.strip_prefix(storage_str) {
        Some(stripped) => {
            if let Some(stripped_leading) = stripped.strip_prefix("/") {
                stripped_leading
            } else {
                stripped
            }
        },
        None => working
    }, true))
}

async fn sync_known_file(conn: &impl GenericClient, md: &Metadata, item: &FsItem) -> Result<()> {
    let mut updated = false;
    let mut created_value = item.created;
    let mut modified_value = item.modified;
    let mut item_size_value = item.item_size;

    if let Ok(created) = md.created() {
        let stamp = created.into();

        if created_value != stamp {
            updated = true;
            created_value = stamp;
        }
    }

    if let Ok(modified) = md.modified() {
        let stamp = modified.into();

        if let Some(modify) = modified_value {
            if modify != stamp {
                updated = true;
                modified_value = Some(stamp);
            } else {
                modified_value = Some(modify);
            }
        } else {
            updated = true;
            modified_value = Some(stamp);
        }
    }

    let md_size = md.len() as i64;

    if md_size != item_size_value {
        updated = true;
        item_size_value = md_size;
    }

    if updated {
        conn.execute(
            "update fs_items set created = $2, modified = $3, item_size = $4, item_exists = true where id = $1", 
            &[&item.id, &created_value, &modified_value, &item_size_value]
        ).await?;
    }

    Ok(())
}

async fn sync_file(app: &AppState<'_>, conn: &impl GenericClient, users_id: &i64, parent: &i64, file_path: &PathBuf) -> Result<i64> {
    let md = metadata(&file_path).await?;
    let (directory, basename) = get_directory_and_basename(app, file_path)?;

    if let Some(item) = FsItem::find_user_id_directory_basename(conn, users_id, &directory, &basename).await? {
        sync_known_file(conn, &md, &item).await?;
        Ok(item.id)
    } else {
        let id = app.snowflakes.fs_items.next_id().await?;
        let item_type: i16 = FsItemType::File.into();
        let created: DateTime<Utc> = if let Ok(c) = md.created() {
            c.into()
        } else {
            Utc::now()
        };
        let modified: Option<DateTime<Utc>> = if let Ok(m) = md.modified() {
            Some(m.into())
        } else {
            None
        };

        conn.execute(
            "\
            insert into fs_items (id, item_type, parent, users_id, directory, basename, created, modified) values \
            ($1, $2, $3, $4, $5, $6, $7, $8)", 
            &[&id, &item_type, parent, users_id, &directory, &basename, &created, &modified]
        ).await?;

        Ok(id)
    }
}

async fn sync_dir(app: &AppState<'_>, conn: &impl GenericClient, users_id: &i64, parent: &i64, dir_path: &PathBuf) -> Result<i64> {
    let (directory, basename) = get_directory_and_basename(app, dir_path)?;

    if let Some(fs_item) = FsItem::find_user_id_directory_basename(conn, users_id, &directory, &basename).await? {
        conn.execute(
            "update fs_items set item_exists = true where id = $1",
            &[&fs_item.id]
        ).await?;

        Ok(fs_item.id)
    } else {
        let md = metadata(dir_path).await?;
        let id = app.snowflakes.fs_items.await_next_id().await?;
        let item_type: i16 = FsItemType::Dir.into();
        let created: DateTime<Utc> = if let Ok(c) = md.created() {
            c.into()
        } else {
            Utc::now()
        };

        conn.execute(
            "\
            insert into fs_items (id, item_type, parent, users_id, directory, basename, created) values \
            ($1, $2, $3, $4, $5, $6, $7)",
            &[&id, &item_type, parent, users_id, &directory, &basename, &created]
        ).await?;

        Ok(id)
    }
}