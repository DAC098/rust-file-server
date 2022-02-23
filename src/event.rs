use std::time::Duration;

use chrono::Utc;
use futures::{Future, stream::FuturesUnordered, StreamExt};
use serde::Serialize;
use serde_json::json;

use crate::{state::AppState, db::record::FsItem, http::error::Result};

pub mod name {
    pub const FS_ITEM_CREATED: &str = "fs_item:created";
    pub const FS_ITEM_UPDATED: &str = "fs_item:updated";
    pub const FS_ITEM_DELETED: &str = "fs_item:deleted";
    pub const FS_ITEM_SYNCED: &str = "fs_item:synced";
}

async fn send_requests<D>(list: impl Iterator<Item = String>, data: D) -> Result<()>
where
    D: Serialize
{
    let serial_data = serde_json::to_string(&data)?;
    let mut outbound_requests = FuturesUnordered::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    for url in list {
        outbound_requests.push(client.post(url)
            .header("content-type", "application/json")
            .header("content-length", serial_data.len())
            .body(serial_data.clone())
            .send());

        if outbound_requests.len() == 10 {
            while let Some(res) = outbound_requests.next().await {
                if let Err(err) = res {
                    log::error!("response error: {}", err);
                }
            }

            outbound_requests.clear();
        }
    }

    if outbound_requests.len() > 0 {
        while let Some(res) = outbound_requests.next().await {
            if let Err(err) = res {
                log::error!("response error: {}", err);
            }
        }
    }

    Ok(())
}

async fn error_wrapper(fut: impl Future<Output = Result<()>>) -> () {
    let result = fut.await;

    if let Err(err) = result {
        log::error!("given event failed. error: {}", err);
    }
}

pub fn trigger_fs_item_created(state: &AppState<'_>, data: FsItem) -> impl Future<Output = ()> {
    let db = state.db.clone();

    error_wrapper(async move {
        let conn = db.pool.get().await?;
        let result = conn.query(
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
                inner join dir_tree on dir_tree.parent = fs_contents.id \
                where fs_contents.item_type = 2 \
            ) \
            select event_listeners.endpoint \
            from dir_tree \
            join event_listeners on ( \
                ref_table = 'fs_items' and \
                ref_id = dir_tree.id \
            )",
            &[&data.id]
        ).await?;
        let iter = result.iter().map(|v| v.get(0));

        let payload = json!({
            "event": name::FS_ITEM_CREATED,
            "timestamp": Utc::now(),
            "payload": data
        });

        send_requests(iter, payload).await?;

        Ok(())
    })
}

pub fn trigger_fs_item_updated(state: &AppState<'_>, data: FsItem) -> impl Future<Output = ()> {
    let db = state.db.clone();

    error_wrapper(async move {
        let conn = db.pool.get().await?;
        let result =  conn.query(
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
                inner join dir_tree on dir_tree.parent = fs_contents.id \
                where fs_contents.item_type = 2 \
            ) \
            select event_listeners.endpoint \
            from dir_tree \
            join event_listeners on ( \
                ref_table = 'fs_items' and \
                ref_id = dir_tree.id \
            )",
            &[&data.id]
        ).await?;
        let iter = result.iter().map(|v| v.get::<usize, String>(0));

        let payload = json!({
            "event": name::FS_ITEM_UPDATED,
            "timestamp": Utc::now(),
            "payload": data
        });

        send_requests(
            iter,
            payload
        ).await?;

        Ok(())
    })
}

pub fn trigger_fs_item_deleted(state: &AppState<'_>, data: Vec<i64>) -> impl Future<Output = ()> {
    let db = state.db.clone();

    error_wrapper(async move {
        let conn = db.pool.get().await?;
        Ok(())
    })
}

pub fn trigger_fs_item_synced(state: &AppState<'_>, data: FsItem) -> impl Future<Output = ()> {
    let db = state.db.clone();

    error_wrapper(async move {
        Ok(())
    })
}