use chrono::{DateTime, Utc, serde::ts_seconds, serde::ts_seconds_option };
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::{Serialize_repr, Deserialize_repr};
use tokio_postgres::GenericClient;

use crate::db::types::Result;

#[repr(i16)]
#[derive(PartialEq, Clone, Serialize_repr, Deserialize_repr)]
pub enum FsItemType {
    Unknown = 0,
    File = 1,
    Dir = 2,
}

impl From<&str> for FsItemType {
    fn from(v: &str) -> Self {
        match v {
            "file" => FsItemType::File,
            "dir" => FsItemType::Dir,
            _ => FsItemType::Unknown
        }
    }
}

impl From<i16> for FsItemType {
    fn from(v: i16) -> Self {
        match v {
            1 => FsItemType::File,
            2 => FsItemType::Dir,
            _ => FsItemType::Unknown
        }
    }
}

impl From<FsItemType> for i16 {
    fn from(v: FsItemType) -> Self {
        v as i16
    }
}

#[derive(Serialize, Deserialize)]
pub struct FsItem {
    pub id: i64,
    pub item_type: FsItemType,
    pub parent: Option<i64>,
    pub users_id: i64,
    pub directory: String,
    pub basename: String,
    pub item_size: i64,
    #[serde(with = "ts_seconds")]
    pub created: DateTime<Utc>,
    #[serde(with = "ts_seconds_option")]
    pub modified: Option<DateTime<Utc>>,
    pub item_exists: bool,
    pub user_data: Value,
    pub is_root: bool
}



impl FsItem {

    pub async fn find_user_id_directory_basename(
        conn: &impl GenericClient, 
        users_id: &i64,
        directory: &String,
        basename: &String
    ) -> Result<Option<FsItem>> {
        if let Some(record) = conn.query_opt(
            "\
            select id, \
                   item_type, \
                   parent, \
                   item_size, \
                   created, \
                   modified, \
                   item_exists, \
                   user_data, \
                   is_root \
            from fs_items \
            where users_id = $1 and \
                  directory = $2 and \
                  basename = $3",
            &[users_id, &directory, &basename]
        ).await? {
            Ok(Some(Self {
                id: record.get(0),
                item_type: record.get::<usize, i16>(1).into(),
                parent: record.get(2),
                users_id: users_id.clone(),
                directory: directory.clone(),
                basename: basename.clone(),
                item_size: record.get(3),
                created: record.get(4),
                modified: record.get(5),
                item_exists: record.get(6),
                user_data: record.get(7),
                is_root: record.get(8),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_path(conn: &impl GenericClient, users_id: &i64, path: &String) -> Result<Option<Self>> {
        let (directory, basename) = lib::string::get_directory_and_basename(&path, true);

        FsItem::find_user_id_directory_basename(conn, users_id, &directory, &basename).await
    }

    pub async fn find_basename_with_parent(conn: &impl GenericClient, parent: &i64, basename: &String) -> Result<Option<FsItem>> {
        if let Some(record) = conn.query_opt(
            "\
            select id, \
                   item_type, \
                   users_id, \
                   item_size, \
                   directory, \
                   created, \
                   modified, \
                   item_exists, \
                   user_data, \
                   is_root \
            from fs_items \
            where parent = $1 and \
                  basename = $2",
            &[parent, basename]
        ).await? {
            Ok(Some(Self {
                id: record.get(0),
                item_type: record.get::<usize, i16>(1).into(),
                parent: Some(parent.clone()),
                users_id: record.get(2),
                item_size: record.get(3),
                directory: record.get(4),
                basename: basename.clone(),
                created: record.get(4),
                modified: record.get(5),
                item_exists: record.get(6),
                user_data: record.get(7),
                is_root: record.get(8),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_dir_contents(
        conn: &impl GenericClient,
        users_id: &i64,
        parent: &Option<i64>
    ) -> Result<Vec<Self>> {
        Ok(conn.query(
            "\
            select id, \
                   item_type, \
                   parent, \
                   users_id, \
                   directory, \
                   basename, \
                   item_size, \
                   created, \
                   modified, \
                   item_exists, \
                   user_data, \
                   is_root \
            from fs_items \
            where users_id = $1 and \
                  parent = $2",
            &[users_id, parent]
        ).await?
        .iter()
        .map(|row| Self {
            id: row.get(0),
            item_type: row.get::<usize, i16>(1).into(),
            parent: parent.clone(),
            users_id: row.get(3),
            directory: row.get(4),
            basename: row.get(5),
            item_size: row.get(6),
            created: row.get(7),
            modified: row.get(8),
            item_exists: row.get(9),
            user_data: row.get(10),
            is_root: row.get(11),
        })
        .collect())
    }

    pub async fn update_item_exists(conn: &impl GenericClient, id: &i64, exists: bool) -> Result<()> {
        conn.execute(
            "update fs_items set item_exists = $2 where id = $1",
            &[id, &exists]
        ).await?;
        Ok(())
    }
}