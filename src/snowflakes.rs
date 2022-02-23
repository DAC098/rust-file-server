use lib::snowflake::{TokioSnowflake, Result};

pub const START_TIME: i64 = 1609459200000;

#[derive(Clone)]
pub struct IdSnowflakes {
    pub fs_items: TokioSnowflake,
    pub users: TokioSnowflake
}

impl IdSnowflakes {
    pub fn new(machine_id: i64) -> Result<IdSnowflakes> {
        Ok(IdSnowflakes {
            fs_items: TokioSnowflake::new(machine_id, START_TIME)?,
            users: TokioSnowflake::new(machine_id, START_TIME)?
        })
    }
}