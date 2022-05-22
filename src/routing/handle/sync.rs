use std::{path::PathBuf, fs::Metadata};

use chrono::{DateTime, Utc};
use serde_json::json;
use tokio::fs::{ReadDir, read_dir, metadata};
use tokio_postgres::GenericClient;

use crate::{
    state::AppState, 
    http::{
        Request, 
        Response, 
        error::{Result, Error}, 
        response::JsonResponseBuilder
    }, 
    components::{
        auth::require_session, 
        fs_items::existing_resource
    }, 
    db::record::{FsItem, FsItemType}, 
    event
};

struct WorkItem {
    iter: ReadDir,
    id: i64
}

pub async fn handle_put(state: AppState, req: Request) -> Result<Response> {
    let mut conn = state.db.pool.get().await?;
    let (user, _) = require_session(&*conn, req.headers()).await?;

    let context = String::new();

    if let Some(fs_item) = existing_resource(&*conn, &user, &context).await? {
        let mut created_items: u64 = 0;
        let mut updated_items: u64 = 0;
        let mut missing_items: u64 = 0;
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
                            let (id, created, updated) = sync_dir(
                                &state, 
                                &transaction, 
                                &user.id, 
                                &working.id, 
                                &entry_path
                            ).await?;

                            if created {
                                created_items += 1;
                            } else if updated {
                                updated_items += 1;
                            }

                            found_ids.push(id.clone());
                            working_queue.push(working);
                            working_queue.push(WorkItem {
                                iter: read_dir(&entry_path).await?,
                                id
                            });

                            break;
                        } else if entry_path.is_file() {
                            let (id, created, updated) = sync_file(
                                &state, 
                                &transaction, 
                                &user.id, 
                                &working.id, 
                                &entry_path
                            ).await?;

                            if created {
                                created_items += 1;
                            } else if updated {
                                updated_items += 1;
                            }

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

            missing_items = transaction.execute(
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
        } else if fs_item.item_type == FsItemType::File {
            let mut fs_path = state.storage.directory.clone();
            fs_path.push(&fs_item.directory);
            fs_path.push(&fs_item.basename);

            if fs_path.exists() {
                let md = metadata(&fs_path).await?;

                if sync_known_file(&transaction, &md, &fs_item).await? {
                    updated_items += 1;
                }
            } else {
                missing_items = transaction.execute(
                    "update fs_items set item_exists = false where id = $1",
                    &[&fs_item.id]
                ).await?;
            }
        }

        transaction.commit().await?;

        state.offload.spawn(event::trigger_fs_item_synced(
            &state,
            fs_item
        ));
    
        JsonResponseBuilder::new(200)
            .payload_response(json!({
                "created": created_items,
                "updated": updated_items,
                "missing": missing_items
            }))
    } else {
        Err(Error::new(404, "PathNotFound", "requested path was not found"))
    }
}

fn path_to_str<'a>(path: &'a PathBuf) -> Result<&'a str> {
    path.to_str().ok_or(Error::new(400, "NonUtf8Path", "encountered a file system path that cannot be converted to utf-8"))
}

fn get_directory_and_basename(app: &AppState, fs_path: &PathBuf) -> Result<(String, String)> {
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

async fn sync_known_file(conn: &impl GenericClient, md: &Metadata, item: &FsItem) -> Result<bool> {
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

    Ok(updated)
}

async fn sync_file(app: &AppState, conn: &impl GenericClient, users_id: &i64, parent: &i64, file_path: &PathBuf) -> Result<(i64, bool, bool)> {
    let md = metadata(&file_path).await?;
    let (directory, basename) = get_directory_and_basename(app, file_path)?;

    if let Some(item) = FsItem::find_user_id_directory_basename(conn, users_id, &directory, &basename).await? {
        Ok((item.id, false, sync_known_file(conn, &md, &item).await?))
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

        Ok((id, true, false))
    }
}

async fn sync_dir(app: &AppState, conn: &impl GenericClient, users_id: &i64, parent: &i64, dir_path: &PathBuf) -> Result<(i64, bool, bool)> {
    let (directory, basename) = get_directory_and_basename(app, dir_path)?;

    if let Some(fs_item) = FsItem::find_user_id_directory_basename(conn, users_id, &directory, &basename).await? {
        conn.execute(
            "update fs_items set item_exists = true where id = $1",
            &[&fs_item.id]
        ).await?;

        Ok((fs_item.id, false, true))
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

        Ok((id, true, false))
    }
}