use chrono::{DateTime, Utc, serde::ts_seconds, serde::ts_seconds_option };
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::{Serialize_repr, Deserialize_repr};
use tokio_postgres::GenericClient;

use crate::http::error::Result;

#[repr(i16)]
#[derive(Debug, PartialEq, Clone, Serialize_repr, Deserialize_repr)]
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

impl From<String> for FsItemType {
    fn from(v: String) -> Self {
        match v.as_str() {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

    pub async fn find_id(conn: &impl GenericClient, id: &i64) -> Result<Option<FsItem>> {
        if let Some(record) = conn.query_opt(
            "\
            select item_type, \
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
            where id = $1",
            &[id]
        ).await? {
            Ok(Some(Self {
                id: id.clone(),
                item_type: record.get::<usize, i16>(0).into(),
                parent: record.get(1),
                users_id: record.get(2),
                directory: record.get(3),
                basename: record.get(4),
                item_size: record.get(5),
                created: record.get(6),
                modified: record.get(7),
                item_exists: record.get(8),
                user_data: record.get(9),
                is_root: record.get(10),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_user_id_directory_basename(
        conn: &impl GenericClient, 
        users_id: &i64,
        directory: &str,
        basename: &str
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
                directory: directory.to_owned(),
                basename: basename.to_owned(),
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

    pub async fn find_path(conn: &impl GenericClient, users_id: &i64, path: &str) -> Result<Option<Self>> {
        let (directory, basename) = lib::string::get_directory_and_basename(path, true);

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
                created: record.get(5),
                modified: record.get(6),
                item_exists: record.get(7),
                user_data: record.get(8),
                is_root: record.get(9),
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
                  parent = $2 \
            order by item_type = 2, \
                     item_type = 1, \
                     basename",
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

    pub async fn create(&self, conn: &impl GenericClient) -> Result<()> {
        let item_type: i16 = self.item_type.clone().into();

        conn.execute(
            "\
            insert into fs_items (\
                id, \
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
                is_root\
            ) values \
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            &[
                &self.id,
                &item_type,
                &self.parent,
                &self.users_id,
                &self.directory,
                &self.basename,
                &self.item_size,
                &self.created,
                &self.modified,
                &self.item_exists,
                &self.user_data,
                &self.is_root
            ]
        ).await?;

        Ok(())
    }

    // pub async fn update(&self, conn: &impl GenericClient) -> Result<()> {
    //     let item_type: i16 = self.item_type.clone().into();

    //     conn.execute(
    //         "\
    //         update fs_items\
    //         set item_type = $2, \
    //             parent = $3, \
    //             users_id = $4, \
    //             directory = $5, \
    //             basename = $6, \
    //             item_size = $7, \
    //             created = $8, \
    //             modified = $9, \
    //             item_exists = $10, \
    //             user_data = $11, \
    //             is_root = $12\
    //         where id = $1",
    //         &[
    //             &self.id,
    //             &item_type,
    //             &self.parent,
    //             &self.users_id,
    //             &self.directory,
    //             &self.basename,
    //             &self.item_size,
    //             &self.created,
    //             &self.modified,
    //             &self.item_exists,
    //             &self.user_data,
    //             &self.is_root
    //         ]
    //     ).await?;

    //     Ok(())
    // }
}