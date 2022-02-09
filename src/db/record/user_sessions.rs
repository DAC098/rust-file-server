use chrono::{DateTime, Utc};
use tokio_postgres::GenericClient;
use uuid::Uuid;

use crate::http::error::Result;

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
                   token, \
                   dropped, \
                   issued_on, \
                   expires \
            from user_sessions \
            where token = $1",
            &[token]
        ).await? {
            Ok(Some(UserSession {
                users_id: record.get(0),
                token: record.get(1),
                dropped: record.get(2),
                issued_on: record.get(3),
                expires: record.get(4)
            }))
        } else {
            Ok(None)
        }
    }

}