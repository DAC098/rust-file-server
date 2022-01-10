use tokio_postgres::GenericClient;

use crate::db::types::Result;

pub struct User {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub email_verifieid: bool
}

impl User {
    pub async fn find_id(conn: &impl GenericClient, id: &i64) -> Result<Option<User>> {
        if let Some(record) = conn.query_opt(
            "select id, username, email, email_verified from users where id = $1",
            &[id]
        ).await? {
            Ok(Some(User {
                id: record.get(0),
                username: record.get(1),
                email: record.get(2),
                email_verifieid: record.get(3)
            }))
        } else {
            Ok(None)
        }
    }
}