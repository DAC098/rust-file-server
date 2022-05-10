use chrono::{DateTime, Utc, Duration};
use serde::Serialize;
use tokio_postgres::GenericClient;
use uuid::Uuid;

use crate::{
    http::error::{Result, Error}, 
    db::query::QueryParams
};

#[derive(Serialize)]
pub struct UserSession {
    pub users_id: i64,
    pub token: Uuid,
    pub dropped: bool,
    pub issued_on: DateTime<Utc>,
    pub expires: DateTime<Utc>,
}

impl UserSession {

    pub fn default_duration() -> Duration {
        Duration::days(7)
    }

    pub fn new(users_id: i64, duration: &Duration) -> Result<UserSession> {
        let issued_on = Utc::now();
        let expires = issued_on.clone()
            .checked_add_signed(duration.clone())
            .ok_or(Error::default())?;

        Ok(UserSession {
            users_id,
            token: Uuid::new_v4(),
            dropped: false,
            issued_on,
            expires
        })
    }

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

    pub async fn find_users_id(conn: &impl GenericClient, users_id: &i64, exclude_token: Option<&Uuid>) -> Result<Vec<UserSession>> {
        let mut query_slice = QueryParams::with_capacity(2);
        query_slice.push(users_id);

        if let Some(token) = exclude_token {
            query_slice.push(token);
        }

        Ok(conn.query(
            "\
            select token, \
                   dropped, \
                   issued_on, \
                   expires \
            from user_sessions \
            where users_id = $1",
            query_slice.slice()
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

    pub async fn insert(&self, conn: &impl GenericClient) -> Result<()> {
        conn.execute(
            "\
            insert into user_sessions (users_id, token, dropped, issued_on, expires) values \
            ($1, $2, $3, $4, $5)",
            &[&self.users_id, &self.token, &self.dropped, &self.issued_on, &self.expires]
        ).await?;

        Ok(())
    }
}