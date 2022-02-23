use uuid::Uuid;

pub struct EventListener {
    id: Uuid,
    event_name: String,
    endpoint: String,
    ref_table: String,
    ref_id: i64
}