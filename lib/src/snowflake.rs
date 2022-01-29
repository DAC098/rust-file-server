use std::{sync::Arc, fmt::{Display, Formatter, Result as FmtResult}, error::Error as StdError};

use chrono::Utc;
use tokio::sync::Mutex;

pub const SNOWFLAKE_TIMESTAMP_BITS: i64 = 42;
pub const SNOWFLAKE_MACHINE_ID_BITS: i64 = 10;
pub const SNOWFLAKE_SEQUENCE_ID_BITS: i64 = 12;

pub const MAX_TIMESTAMP: i64 = (1 << SNOWFLAKE_TIMESTAMP_BITS) - 1;
pub const MAX_MACHINE_ID: i64 = (1 << SNOWFLAKE_MACHINE_ID_BITS) - 1;
pub const MAX_SEQUENCE_ID: i64 = (1 << SNOWFLAKE_SEQUENCE_ID_BITS) - 1;

const SNOWFLAKE_TIMESTAMP_BIT_MASK: i64 = MAX_TIMESTAMP << (SNOWFLAKE_MACHINE_ID_BITS + SNOWFLAKE_SEQUENCE_ID_BITS);
const SNOWFLAKE_MACHINE_ID_BIT_MASK: i64 = MAX_MACHINE_ID << SNOWFLAKE_SEQUENCE_ID_BITS;
const SNOWFLAKE_SEQUENCE_ID_BIT_MASK: i64 = MAX_MACHINE_ID;

#[derive(Debug)]
pub enum Error {
    MachineIdTooLarge,
    StartTimeTooLarge,
    TimestampMaxReached,
    SequenceMaxReached,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Error::MachineIdTooLarge => write!(
                f, "the given machine id is too large. the value cannot be larger than {}", MAX_MACHINE_ID
            ),
            Error::StartTimeTooLarge => write!(
                f, "the requested start time is too large. the value cannot be larger than {}", MAX_TIMESTAMP
            ),
            Error::TimestampMaxReached => write!(
                f, "max timestamp amount reached"
            ),
            Error::SequenceMaxReached => write!(
                f, "max sequence amount reached"
            )
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

#[derive(Clone)]
pub struct TokioSnowflake {
    pub start_time: i64,
    pub machine_id: i64,
    sequence: Arc<Mutex<i64>>,
    prev_time: Arc<Mutex<i64>>
}

impl TokioSnowflake {

    pub fn new(machine_id: i64, start_time: i64) -> Result<TokioSnowflake> {
        if machine_id > MAX_MACHINE_ID {
            return Err(Error::MachineIdTooLarge);
        }

        if start_time > MAX_TIMESTAMP {
            return Err(Error::StartTimeTooLarge);
        }

        Ok(TokioSnowflake {
            start_time,
            machine_id,
            sequence: Arc::new( Mutex::new(1)),
            prev_time: Arc::new(Mutex::new(1))
        })
    }

    pub async fn next_id(&self) -> Result<i64> {
        let now = current_timestamp() - self.start_time;
        let mut seq_value: i64 = 1;

        if now > MAX_TIMESTAMP {
            return Err(Error::TimestampMaxReached);
        }

        {
            let mut prev_time_lock = self.prev_time.lock().await;
            let mut sequence_lock = self.sequence.lock().await;

            if now == *prev_time_lock {
                if *sequence_lock > MAX_SEQUENCE_ID {
                    return Err(Error::SequenceMaxReached);
                }

                *sequence_lock += 1;
                seq_value = sequence_lock.clone();
            } else {
                *prev_time_lock = now;
                *sequence_lock = 1;
            }
        }

        Ok((now << SNOWFLAKE_MACHINE_ID_BITS + SNOWFLAKE_SEQUENCE_ID_BITS) |
            (self.machine_id << SNOWFLAKE_SEQUENCE_ID_BITS) |
            seq_value)
    }
}

#[inline]
fn current_timestamp() -> i64 {
    Utc::now().timestamp_millis()
}

pub fn decompose(value: i64) -> (i64,i64,i64) {
    return (
        (value & SNOWFLAKE_TIMESTAMP_BIT_MASK) >> (SNOWFLAKE_MACHINE_ID_BITS + SNOWFLAKE_SEQUENCE_ID_BITS),
        (value & SNOWFLAKE_MACHINE_ID_BIT_MASK) >> SNOWFLAKE_SEQUENCE_ID_BITS,
        value & SNOWFLAKE_SEQUENCE_ID_BIT_MASK
    )
}