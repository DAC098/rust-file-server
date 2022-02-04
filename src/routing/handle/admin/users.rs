use serde::Deserialize;
use serde_json::json;
use tokio::fs::create_dir;

use crate::{http::{Request, Response, error::{Result, Error}, response::json_response, body::json_from_body}, db::record::{User, FsItemType}, components::auth::get_session, security::argon::hash_with_default, state::AppState};

#[derive(Deserialize)]
struct NewUserJson {
    username: String,
    password: String,
    email: Option<String>
}

pub async fn handle_post(app: AppState<'_>, req: Request) -> Result<Response> {
    let (head, body) = req.into_parts();
    let mut conn = app.db.pool.get().await?;
    let (_user, _session) = get_session(&head.headers, &*conn).await?;
    let new_user: NewUserJson = json_from_body(body).await?;

    let existing = User::find_username_or_optional_email(&*conn, &new_user.username, &new_user.email).await?;

    if existing.len() != 0 {
        for record in existing {
            if record.username == new_user.username {
                return Err(Error {
                    status: 400,
                    name: "UsernameInUse".into(),
                    msg: "the requested username is already in use".into(),
                    source: None
                })
            } else {
                return Err(Error {
                    status: 400,
                    name: "EmailInUse".into(),
                    msg: "the requested email is already in use".into(),
                    source: None
                })
            }
        }
    }

    let transaction = conn.transaction().await?;
    let user = {
        let id = app.snowflakes.users.next_id().await?;
        let hash = hash_with_default(&new_user.password)?;
        let email_verified = false;

        transaction.query(
            "\
            insert into users (id, username, hash, email, email_verified) values \
            ($1, $2, $3, $4, $5)", 
            &[&id, &new_user.username, &hash, &new_user.email, &email_verified]
        ).await?;

        User {
            id,
            username: new_user.username,
            email: new_user.email,
            email_verified
        }
    };

    {
        let id = app.snowflakes.fs_items.next_id().await?;
        let item_type: i16 = FsItemType::Dir.into();
        let parent = None::<i64>;
        let directory = "".to_owned();
        let basename = user.id.to_string();
        let created = chrono::Utc::now();
        let modified = None::<chrono::DateTime<chrono::Utc>>;
        let is_root = true;

        transaction.query(
            "\
            insert into fs_items (id, item_type, parent, users_id, directory, basename, created, modified, is_root) values \
            ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            &[&id, &item_type, &parent, &user.id, &directory, &basename, &created, &modified, &is_root]
        ).await?;
    }

    {
        let mut root_path = app.storage.directory.clone();
        root_path.push(user.id.to_string());

        create_dir(root_path).await?;
    }

    transaction.commit().await?;

    let json = json!({"message":"successful","payload": user});
    json_response(200, &json)
}