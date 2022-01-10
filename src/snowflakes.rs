use lib::snowflake::{TokioSnowflake, Result};

#[derive(Clone)]
pub struct IdSnowflakes {
    pub fs_items: TokioSnowflake,
    pub users: TokioSnowflake
}

impl IdSnowflakes {
    pub fn new(machine_id: i64) -> Result<IdSnowflakes> {
        Ok(IdSnowflakes {
            fs_items: TokioSnowflake::new(machine_id, 1609459200000)?,
            users: TokioSnowflake::new(machine_id, 1609459200000)?
        })
    }
}