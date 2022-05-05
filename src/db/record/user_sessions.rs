use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio_postgres::GenericClient;
use uuid::Uuid;

use crate::http::error::Result;

#[derive(Serialize)]
pub struct UserSession {
    pub users_id: i64,
    pub token: Uuid,
    pub dropped: bool,
    pub issued_on: DateTime<Utc>,
    pub expires: DateTime<Utc>,
}

impl UserSession {

    pub async fn find_token(conn: &impl GenericClient, token: &Uuid) -> Result<Option<UserSession>> {
        if let Some(record) = conn.query_opt(
            "\
            select users_id, \
                   dropped, \
                   issued_on, \
                   expires \
            from user_sessions \
            where token = $1",
            &[token]
        ).await? {
            Ok(Some(UserSession {
                users_id: record.get(0),
                token: token.clone(),
                dropped: record.get(1),
                issued_on: record.get(2),
                expires: record.get(3)
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_users_id(conn: &impl GenericClient, users_id: &i64) -> Result<Vec<UserSession>> {
        Ok(conn.query("\
            select token, \
                   dropped, \
                   issued_on, \
                   expires \
            from user_sessions \
            where users_id = $1",
            &[users_id]
        )
            .await?
            .iter()
            .map(|row| UserSession {
                users_id: users_id.clone(),
                token: row.get(0),
                dropped: row.get(1),
                issued_on: row.get(2),
                expires: row.get(3)
            })
            .collect())
    }

}