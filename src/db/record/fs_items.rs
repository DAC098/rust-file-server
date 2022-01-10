use chrono::{DateTime, Utc, serde::ts_seconds, };
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::GenericClient;

use crate::db::types::Result;

#[repr(i16)]
#[derive(PartialEq, Clone, Serialize, Deserialize)]
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
    #[serde(with = "ts_seconds")]
    pub created: DateTime<Utc>,
    #[serde(with = "ts_seconds")]
    pub modified: DateTime<Utc>,
    pub user_data: Value,
    pub is_root: bool
}



impl FsItem {
    pub async fn find_path(conn: &impl GenericClient, users_id: &i64, path: &str) -> Result<Option<Self>> {
        let mut first = true;
        let mut first_dir = true;
        let mut directory = String::new();
        let mut basename = String::new();
        let path_split_iter = path.split('/');

        for item in path_split_iter {
            if first {
                basename = item.to_owned();
                first = false;
            } else {
                if first_dir {
                    first_dir = false;
                } else {
                    directory.push('/');
                }

                directory.push_str(&basename);
                basename = item.to_owned();
            }
        }

        if let Some(record) = conn.query_opt(
            "\
            select id, \
                   item_type, \
                   parent, \
                   users_id, \
                   created, \
                   modified, \
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
                users_id: record.get(3),
                directory,
                basename,
                created: record.get(4),
                modified: record.get(5),
                user_data: record.get(6),
                is_root: record.get(7),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_basename_with_parent(conn: &impl GenericClient, parent: &i64, basename: &String) -> Result<Option<FsItem>> {
        if let Some(record) = conn.query_opt(
            "\
            select id, \
                   item_type, \
                   users_id, \
                   directory, \
                   created, \
                   modified, \
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
                directory: record.get(3),
                basename: basename.clone(),
                created: record.get(4),
                modified: record.get(5),
                user_data: record.get(6),
                is_root: record.get(7),
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
                   created, \
                   modified, \
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
            created: row.get(6),
            modified: row.get(7),
            user_data: row.get(8),
            is_root: row.get(9),
        })
        .collect())
    }
}