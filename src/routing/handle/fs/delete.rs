use std::collections::HashSet;
use std::io::ErrorKind;

use futures::{pin_mut, TryStreamExt};
use tokio::fs::{remove_file, remove_dir};

use crate::components::auth::require_session;
use crate::components::fs_items::existing_resource;
use crate::db::record::FsItemType;
use crate::event;
use crate::http::response::JsonResponseBuilder;
use crate::http::{Response, Request};
use crate::http::error::{Error, Result};
use crate::state::AppState;

pub async fn handle_delete(state: AppState, req: Request) -> Result<Response> {
    let mut conn = state.db.pool.get().await?;
    let (user, _) = require_session(&*conn, req.headers()).await?;
    let context = String::new();

    if let Some(fs_item) = existing_resource(&*conn, &user, &context).await? {
        if fs_item.is_root {
            return Err(Error::new(400, "CannotDeleteRoot", "you cannot delete your root directory"));
        }

        if fs_item.users_id != user.id {
            // permissions check
        }

        let mut fs_path = state.storage.directory.clone();
        fs_path.push(&fs_item.directory);
        fs_path.push(&fs_item.basename);

        let mut deleted_records: Vec<i64> = Vec::new();
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

            deleted_records.push(fs_item.id);
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

            deleted_records = marked_delete;
        }

        transaction.commit().await?;

        state.offload.spawn(event::trigger_fs_item_deleted(
            &state,
            deleted_records
        ));

        JsonResponseBuilder::new(204)
            .response()
    } else {
        Err(Error::new(404, "PathNotFound", "requested path was not found"))
    }
}