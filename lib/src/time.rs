use std::{
    time:: {
        SystemTime,
        Duration, UNIX_EPOCH
    }
};

use chrono::{DateTime, Utc, NaiveDateTime};

pub fn format_duration(duration: &Duration) -> String {
    if duration.as_secs() != 0 {
        format!("{}s", duration.as_secs())
    } else if duration.as_millis() != 0 {
        format!("{}ms", duration.as_millis())
    } else if duration.as_micros() != 0 {
        format!("{}μs", duration.as_micros())
    } else {
        format!("{}ns", duration.as_nanos())
    }
}

const NSEC_SECONDS: i64 = 1000000000;
const MSEC_SECONDS: i64 = 1000;

pub fn timestamp_from_nsecs(ts: i64) -> DateTime<Utc> {
    println!("seconds: {}", ts / NSEC_SECONDS);
    println!("nanos  : {}", ts % NSEC_SECONDS);

    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(
        ts / NSEC_SECONDS, 
        (ts % NSEC_SECONDS) as u32
    ), Utc)
}

pub  fn timestamp_from_msecs(ts: i64) -> DateTime<Utc> {
    println!("seconds: {}", ts / MSEC_SECONDS);
    println!("milis  : {}", ts % MSEC_SECONDS);

    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(
        ts / MSEC_SECONDS, ((ts % MSEC_SECONDS) * 1000000) as u32
    ), Utc)
}

// pulled from chrono::DateTime<Utc> From<SystemTime>
pub fn unix_epoch_systemtime(time: &SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(dur) => dur.as_secs() as i64,
        Err(e) => {
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());

            if nsec == 0 {
                -sec
            } else {
                -sec - 1
            }
        }
    }
}

pub fn unix_epoch_sec_now() -> Option<u64> {
    let now = SystemTime::now();

    match now.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            Some(duration.as_secs())
        },
        Err(_error) => {
            None
        }
    }
}

pub fn unix_epoch_millis_now() -> Option<u64> {
    let now = SystemTime::now();

    match now.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            if let Some(sec) = duration.as_secs().checked_mul(1000) {
                let millis: u64 = duration.subsec_millis().into();

                Some(sec + millis)
            } else {
                None
            }
        },
        Err(_error) => {
            None
        }
    }
}