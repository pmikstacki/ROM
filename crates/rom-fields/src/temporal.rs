use crate::scalar::{invalid, string_field};
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};

string_field!(
    Date,
    "rom.date",
    date,
    "Gregorian date, year 0001 through 9999, encoded as YYYY-MM-DD."
);
string_field!(
    Time,
    "rom.time",
    time,
    "Time without a timezone, with nanosecond precision and no leap seconds."
);
string_field!(
    DateTime,
    "rom.datetime",
    datetime,
    "RFC3339 input with a known offset; canonical UTC with exactly nine fractional digits."
);

fn digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
}
fn date(value: &str) -> rom::Result<String> {
    if !value.is_ascii()
        || value.len() != 10
        || &value[4..5] != "-"
        || &value[7..8] != "-"
        || !digits(&value[..4])
        || !digits(&value[5..7])
        || !digits(&value[8..])
        || &value[..4] == "0000"
    {
        return Err(invalid("rom.date"));
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| invalid("rom.date"))?;
    Ok(value.into())
}
fn time(value: &str) -> rom::Result<String> {
    if !value.is_ascii()
        || value.len() < 8
        || &value[2..3] != ":"
        || &value[5..6] != ":"
        || !digits(&value[..2])
        || !digits(&value[3..5])
        || !digits(&value[6..8])
        || (value.len() > 8 && (value.len() > 18 || &value[8..9] != "." || !digits(&value[9..])))
        || &value[6..8] == "60"
    {
        return Err(invalid("rom.time"));
    }
    NaiveTime::parse_from_str(value, "%H:%M:%S%.f").map_err(|_| invalid("rom.time"))?;
    let canonical = value.trim_end_matches('0');
    Ok(if value.len() == 8 {
        value.into()
    } else {
        canonical.trim_end_matches('.').into()
    })
}
fn datetime(value: &str) -> rom::Result<String> {
    if !value.is_ascii() || value.len() < 20 || &value[10..11] != "T" || value.ends_with("-00:00") {
        return Err(invalid("rom.datetime"));
    }
    let offset_start = if value.ends_with('Z') {
        value.len() - 1
    } else {
        value
            .len()
            .checked_sub(6)
            .ok_or_else(|| invalid("rom.datetime"))?
    };
    if offset_start < 19 {
        return Err(invalid("rom.datetime"));
    }
    let offset = &value[offset_start..];
    if offset != "Z"
        && (offset.len() != 6
            || !matches!(offset.as_bytes()[0], b'+' | b'-')
            || &offset[3..4] != ":"
            || !digits(&offset[1..3])
            || !digits(&offset[4..6]))
    {
        return Err(invalid("rom.datetime"));
    }
    date(&value[..10]).map_err(|_| invalid("rom.datetime"))?;
    time(&value[11..offset_start]).map_err(|_| invalid("rom.datetime"))?;
    let utc = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| invalid("rom.datetime"))?
        .with_timezone(&chrono::Utc);
    if !(1..=9999).contains(&utc.year()) {
        return Err(invalid("rom.datetime"));
    }
    Ok(format!(
        "{}.{:09}Z",
        utc.format("%Y-%m-%dT%H:%M:%S"),
        utc.nanosecond()
    ))
}
