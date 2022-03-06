use serde::{Deserialize, Serialize};
use tokio_postgres::GenericClient;
use uuid::Uuid;

use crate::http::error;

#[derive(Debug, Deserialize, Serialize)]
pub struct EventListener {
    pub id: Uuid,
    pub event_name: String,
    pub endpoint: String,
    pub ref_table: String,
    pub ref_id: i64,
    pub users_id: i64
}

impl EventListener {

    pub async fn find_user_id(conn: &impl GenericClient, users_id: &i64) -> error::Result<Vec<EventListener>> {
        Ok(conn.query(
            "\
            select id, \
                   event_name, \
                   endpoint, \
                   ref_table, \
                   ref_id \
            from event_listeners\
            where users_id = $1",
            &[users_id]
        ).await?
            .iter()
            .map(|v| EventListener {
                id: v.get(0),
                event_name: v.get(1),
                endpoint: v.get(2),
                ref_table: v.get(3),
                ref_id: v.get(4),
                users_id: users_id.clone()
            })
            .collect()
        )
    }

    pub async fn insert(&self, conn: &impl GenericClient) -> error::Result<()> {
        conn.execute(
            "\
            insert into event_listeners (id, event_name, endpoint, ref_table, ref_id, users_id) values \
            ($1, $2, $3, $4, $5, $6)",
            &[&self.id, &self.event_name, &self.endpoint, &self.ref_table, &self.ref_id, &self.users_id]
        ).await?;

        Ok(())
    }
}