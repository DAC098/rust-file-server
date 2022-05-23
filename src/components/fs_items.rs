use tokio_postgres::GenericClient;

use crate::{db::record::FsItem, http::{error::{Result, Error}, uri::QueryMap}};

pub fn validate_basename(basename: &str) -> Result<String> {
    let trim = basename.trim();

    if trim.is_empty() {
        return Err(Error::new(400, "InvalidBasename", "basename caonnot be empty and leading/trailing whitespace will be removed"))
    }

    Ok(trim.to_owned())
}

pub fn parse_new_context<'a>(context: &'a str) -> (&'a str, Option<&'a str>) {
    if let Some((parent, basename)) = context.rsplit_once('/') {
        (basename, Some(parent))
    } else {
        (context, None)
    }
}

pub struct SearchOptions {
    pub users_id: i64,
    pub is_path: Option<bool>
}

impl SearchOptions {

    pub fn new(users_id: i64) -> Self {
        SearchOptions { users_id, is_path: None }
    }

    pub fn pull_from_query_map(&mut self, query_map: &QueryMap) -> Result<()> {
        if let Some(is_path) = query_map.get_value_ref("is_path") {
            if let Some(value) = is_path {
                self.is_path = Some(value == "1");
            } else {
                self.is_path = Some(true)
            }
        }

        if let Some(users_id) = query_map.get_value_ref("users_id") {
            if let Some(value) = users_id {
                if let Ok(p) = value.parse() {
                    self.users_id = p;
                } else {
                    return Err(Error::new(400, "InvalidId", "given users id is not a valid integer"))
                }
            } else {
                return Err(Error::new(400, "InvalidId", "no users id was specified"))
            }
        }

        Ok(())
    }
}

pub async fn existing_resource(conn: &impl GenericClient, context: &str, options: SearchOptions) -> Result<Option<FsItem>> {
    if let Some(is_path) = options.is_path {
        if is_path {
            FsItem::find_path(conn, &options.users_id, context).await
        } else {
            if let Ok(id) = context.parse::<i64>() {
                FsItem::find_id(conn, &id).await
            } else {
                Err(Error::new(400, "InvalidId", "given context id is not a valid integer"))
            }
        }
    } else {
        if let Ok(id) = context.parse::<i64>() {
            let record = FsItem::find_id(conn, &id).await?;
    
            if record.is_some() {
                return Ok(record)
            }
        }

        FsItem::find_path(conn, &options.users_id, context).await
    }
}

pub async fn new_resource(conn: &impl GenericClient, context: &str, options: SearchOptions) -> Result<(Option<FsItem>, String)> {
    let fallback_context = "";
    let (basename, existing) = parse_new_context(context);
    let valid = validate_basename(basename)?;
    let record = existing_resource(
        conn, 
        existing.unwrap_or(fallback_context), 
        options
    ).await?;

    Ok((record, valid))
}