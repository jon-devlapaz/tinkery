//! Time for every writer and reader: UTC timestamps and the operator's local date from the
//! system clock, and the one RFC 3339 parser. Callers choose their stored fraction and suffix.

use std::time::{SystemTime, UNIX_EPOCH};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Year, month and day for a count of days since 1970-01-01 (Howard Hinnant's algorithm).
fn date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = era * 400 + yoe + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// `time` in UTC as `2026-10-12T22:40:00`, with `sep` between hours, minutes and seconds, and
/// its sub-second nanoseconds. The caller adds any fraction and the zone. A time before 1970
/// reads as 1970-01-01T00:00:00.
pub(crate) fn utc(time: SystemTime, sep: char) -> (String, u32) {
    let d = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    let rest = secs % 86_400;
    let stamp = format!(
        "{}T{:02}{sep}{:02}{sep}{:02}",
        date((secs / 86_400) as i64),
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    );
    (stamp, d.subsec_nanos())
}

/// Now in whole UTC seconds as `2026-10-12T22:40:00+00:00`, the stamp receipts and ledgers use.
pub(crate) fn now() -> String {
    format!("{}+00:00", utc(SystemTime::now(), ':').0)
}

/// Today's date where the operator is, as `2026-10-12`; UTC when the zone cannot be read.
pub(crate) fn local_date() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    date((secs + utc_offset(secs)).div_euclid(86_400))
}

/// The local time zone's offset from UTC at `secs`, in seconds; 0 when it can't be read.
#[cfg(unix)]
fn utc_offset(secs: i64) -> i64 {
    let time = secs as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are valid for the call; localtime_r writes only `tm`.
    if unsafe { libc::localtime_r(&time, &mut tm) }.is_null() {
        return 0;
    }
    tm.tm_gmtoff as i64
}

#[cfg(not(unix))]
fn utc_offset(_secs: i64) -> i64 {
    0
}

/// An RFC 3339 timestamp with its own offset kept, accepting any one character between date
/// and time, a leap second and lowercase `t` or `z`; anything else is refused.
pub(crate) fn parse(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn dates_and_utc_stamps_match_known_days() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(-1), "1969-12-31");
        assert_eq!(date(11016), "2000-02-29");
        let at = |secs, nanos| UNIX_EPOCH + Duration::new(secs, nanos);
        assert_eq!(utc(at(0, 7), ':'), ("1970-01-01T00:00:00".into(), 7));
        assert_eq!(utc(at(951_782_400, 0), ':').0, "2000-02-29T00:00:00");
        assert_eq!(
            utc(at(1_791_844_800, 0), '-'),
            ("2026-10-12T22-40-00".into(), 0)
        );
        assert_eq!(
            utc(UNIX_EPOCH - Duration::from_secs(1), ':').0,
            "1970-01-01T00:00:00"
        );
        let stamp = now();
        assert!(
            stamp.ends_with("+00:00") && stamp.starts_with("20") && stamp.len() == 25,
            "receipts stay UTC in whole seconds: {stamp}"
        );
    }

    #[test]
    fn parse_reads_rfc_3339_with_offsets_and_refuses_impossible_dates() {
        let unix = |value| parse(value).map(OffsetDateTime::unix_timestamp);
        assert_eq!(unix("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix("1970-01-01T02:00:00.5+02:00"), Some(0));
        assert!(parse("2024-02-29T00:00:00Z").is_some());
        for refused in ["2025-02-29T00:00:00Z", "2026-10-11", "not a time", ""] {
            assert_eq!(parse(refused), None, "{refused}");
        }
    }

    #[test]
    fn the_seed_is_dated_where_the_operator_is() {
        // The operator's own clock says what today is; retry once across midnight.
        let today = || {
            let out = std::process::Command::new("date")
                .arg("+%F")
                .output()
                .unwrap();
            String::from_utf8(out.stdout).unwrap().trim().to_owned()
        };
        let (before, ours, after) = (today(), local_date(), today());
        assert!(
            ours == before || ours == after,
            "{ours} vs {before}/{after}"
        );
    }
}
