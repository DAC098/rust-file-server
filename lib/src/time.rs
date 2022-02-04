use std::{
    time:: {
        SystemTime,
        Duration, UNIX_EPOCH
    }
};

use chrono::{DateTime, Utc, NaiveDateTime};

pub fn systemtime_to_utc_seconds(time: SystemTime) -> i64 {
    chrono::DateTime::<chrono::Utc>::from(time).timestamp()
}

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