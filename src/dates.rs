use actix_web::cookie::time::Duration;
use chrono::{DateTime, NaiveDateTime, Utc};


fn i128_into_i64(n: i128) -> Option<i64> {
    if n > i64::MAX as i128 {
        None
    } else {
        Some(n as i64)
    }
}

pub(crate) fn add_duration_to_current(duration: &Duration) -> Option<DateTime<Utc>> {
    let offset = match i128_into_i64(duration.whole_milliseconds()) {
        Some(o) => o,
        None => {
            return None;
        }
    };
    let naive_date = NaiveDateTime::from_timestamp_millis(Utc::now().timestamp_millis() + offset)?;
    Some(DateTime::<Utc>::from_utc(naive_date, Utc))
}
