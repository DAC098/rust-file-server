use tokio_postgres::GenericClient;

use crate::{db::record::{User, FsItem}, http::error::{Result, Error}};

#[derive(Debug)]
pub enum ContextType {
    Id(i64),
    Path(String)
}

pub fn validate_basename(mut basename: String) -> Result<String> {
    basename = basename.trim().to_owned();

    log::debug!("basename: {:?}", basename);

    if basename.is_empty() {
        return Err(Error::new(400, "InvalidBasename", "basename caonnot be empty and leading/trailing whitespace will be removed"))
    }

    Ok(basename)
}

pub fn join_id_and_path(users_id: &i64, context: &str) -> String {
    let id_str = users_id.to_string();

    if context.len() == 0 {
        id_str
    } else {
        let mut rtn = String::with_capacity(id_str.len() + 1 + context.len());
        rtn.push_str(&id_str);
        rtn.push('/');
        rtn.push_str(context);
        rtn
    }
}

pub fn existing_context(user: &User, context: &str) -> ContextType {
    if let Ok(id) = context.parse::<i64>() {
        ContextType::Id(id)
    } else {
        ContextType::Path(join_id_and_path(&user.id, context))
    }
}

pub fn new_context(user: &User, context: &str) -> (ContextType, String) {
    if let Some((parent, basename)) = context.rsplit_once('/') {
        (existing_context(user, parent), basename.into())
    } else {
        (existing_context(user, ""), context.into())
    }
}

pub async fn existing_resource(conn: &impl GenericClient, user: &User, context: &str) -> Result<Option<FsItem>> {
    match existing_context(user, context) {
        ContextType::Id(id) => {
            FsItem::find_id(conn, &user.id, &id).await
        },
        ContextType::Path(path) => {
            FsItem::find_path(conn, &user.id, &path).await
        }
    }
}

pub async fn new_resource(conn: &impl GenericClient, user: &User, context: &str) -> Result<(Option<FsItem>, String)> {
    log::debug!("context: {:?}", context);
    let (existing, mut basename) = new_context(user, context);

    log::debug!("context_type: {:?} basename: {:?}", existing, basename);

    basename = validate_basename(basename)?;

    match existing {
        ContextType::Id(id) => {
            let record = FsItem::find_id(conn, &user.id, &id).await?;

            Ok((record, basename))
        },
        ContextType::Path(path) => {
            let record = FsItem::find_path(conn, &user.id, &path).await?;

            Ok((record, basename))
        }
    }
}