//! UTC timestamps without a date-time dependency, and the injectable clock.
//!
//! Records use `YYYY-MM-DDTHH:MM:SSZ`. Only the CLI entry points read the system clock; the
//! queue and join logic take a [`Clock`] so their tests are deterministic.

use anyhow::{Result, bail};

/// Source of the current time in milliseconds since the Unix epoch.
pub trait Clock {
    /// Now, in Unix milliseconds.
    fn now_millis(&self) -> i64;
}

/// The real clock (used only by CLI entry points).
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0)
    }
}

// Howard Hinnant's civil-from-days and days-from-civil algorithms.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Format Unix seconds as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_utc(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn num(s: &str, what: &str) -> Result<i64> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        bail!("bad {what} `{s}`");
    }
    Ok(s.parse()?)
}

/// Parse `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM:SSZ` or `…±HH:MM` into Unix seconds.
pub fn parse_utc(s: &str) -> Result<i64> {
    let s = s.trim();
    let (date, time) = match s.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let parts: Vec<&str> = date.split('-').collect();
    let [y, m, d] = parts.as_slice() else {
        bail!("bad date `{s}` (want YYYY-MM-DD[THH:MM:SSZ])");
    };
    let (y, m, d) = (num(y, "year")?, num(m, "month")?, num(d, "day")?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        bail!("bad date `{s}`");
    }
    let mut secs = days_from_civil(y, m, d) * 86400;
    if let Some(t) = time {
        let (clock, offset) = if let Some(c) = t.strip_suffix('Z') {
            (c, 0)
        } else if let Some(i) = t.rfind(['+', '-']) {
            let (c, o) = t.split_at(i);
            let sign = if o.starts_with('-') { -1 } else { 1 };
            let (oh, om) = o[1..].split_once(':').unwrap_or((&o[1..], "0"));
            (
                c,
                sign * (num(oh, "offset")? * 3600 + num(om, "offset")? * 60),
            )
        } else {
            bail!("timestamp `{s}` needs a zone (Z or ±HH:MM)");
        };
        let hms: Vec<&str> = clock.split(':').collect();
        let [h, mi, se] = hms.as_slice() else {
            bail!("bad time in `{s}`");
        };
        let se = se.split('.').next().unwrap_or("0");
        secs += num(h, "hour")? * 3600 + num(mi, "minute")? * 60 + num(se, "second")?;
        secs -= offset;
    }
    Ok(secs)
}

/// The `YYYY-MM-DD` part of a timestamp produced by [`format_utc`].
pub fn date_of(secs: i64) -> String {
    format_utc(secs)[..10].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_and_parse_round_trip() {
        for secs in [0, 951782400, 1_700_000_000, 1_793_000_000, 4_102_444_800] {
            assert_eq!(parse_utc(&format_utc(secs)).unwrap(), secs);
        }
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951782400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn parse_offsets_and_dates() {
        let z = parse_utc("2026-11-12T10:00:00Z").unwrap();
        assert_eq!(parse_utc("2026-11-12T11:00:00+01:00").unwrap(), z);
        assert_eq!(parse_utc("2026-11-12T05:30:00-04:30").unwrap(), z);
        assert_eq!(parse_utc("2026-11-12").unwrap(), z - 10 * 3600);
        assert!(parse_utc("2026-13-01").is_err());
        assert!(parse_utc("2026-11-12T10:00:00").is_err());
        assert!(parse_utc("yesterday").is_err());
    }
}
