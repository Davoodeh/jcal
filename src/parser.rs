//! Holds date and time parsers.

use jelal::{IYear, Month, UMonth, UMonthDay, UWeekday, Weekday};
use jiff::{Zoned, fmt::strtime::BrokenDownTime};

use crate::{GREGORIAN_MONTHS, JALALI_MONTHS, WEEKDAYS, posix};

/// Parse a stirng with multiple strategies to see if one makes sense.
///
/// If given a `now`, the basis of relative times will be set. The timezone to that value is also
/// the default timezone if given. If ommited, a new `now` will be called.
///
/// This supports both POSIX format and POSIX timezone.
///
/// This is as close as it gets to `parse_datetime`.
// TODO `now` should be a &Zoned instead of owned
pub fn parse_datetime(mut s: &str, now: Option<Zoned>) -> Result<Zoned, jiff::Error> {
    let mut now = now.unwrap_or_else(|| Zoned::now());

    // split the timezone here since posix parser doesn't support it.
    // This also relaxes whitespaces inside quotes:
    // https://github.com/uutils/parse_datetime/issues/240
    if let (Some(tz), rest) = posix::parse_timezone(s) {
        s = rest;
        now = now.with_time_zone(tz);
    }

    let posix = {
        posix::DateTime::parse_loose(s, false, now.month() as u8, now.day() as u8)
            .or_else(|_| posix::DateTime::parse_loose(s, true, now.month() as u8, now.day() as u8))
    };

    // first try posix and then go for relative, else absolute
    match posix {
        Ok(tm) => {
            let second_is_none = tm.second.is_none();
            tm.to_datetime(now.year()).and_then(|i| {
                match second_is_none {
                    // reset the second to what it was before if forcefully was set to 0
                    true => i.with().second(now.second()).build().unwrap(),
                    false => i,
                }
                .to_zoned(now.time_zone().clone())
            })
        }
        Err(_) => {
            let tz = now.time_zone().clone();
            let parsed = parse_datetime::parse_datetime_at_date(now.clone(), s)
                .or_else(|_| parse_datetime::parse_datetime(s))
                .map_err(|e| jiff::Error::from_args(format_args!("{}", e)))?;
            Ok(parsed.with_time_zone(tz))
        }
    }
}

/// Parse a triplet of "%Y/%m/%d".
// TODO retire this and add it under the `date.rs` file
fn parse_ymd_raw(s: &str) -> Result<(i16, i8, i8), jiff::Error> {
    let tm = BrokenDownTime::parse("%Y/%m/%d", s)?;
    Ok((tm.year().unwrap(), tm.month().unwrap(), tm.day().unwrap()))
}

/// Parse a Jalali date in "%Y/%m/%d" format.
pub fn parse_ymd_jalali(s: &str) -> Result<jelal::Date, jiff::Error> {
    let (y, m, d) = parse_ymd_raw(s)?;
    let date_raw = (y as IYear, m as UMonth, d as UMonthDay); // safe
    Ok(jelal::Date::from(date_raw))
}

/// Match prefix of strings if uniquely identifiable without casing (ASCII only).
///
/// This is only used for easier parsing of names and values with minor extra checkes for constant
/// changing if ever any of the constants needed a tweak. So ignore this entirely if looking for the
/// actual calendar code.
struct IgnoreCasePrefixMatch<const N: usize> {
    /// How many characters this matching index need before being uniquely matched.
    common_prefixes: [usize; N],
    /// Given values.
    values: [&'static str; N],
}

impl<const N: usize> IgnoreCasePrefixMatch<N> {
    /// Create an instance or panic.
    pub const fn new(list: [&'static str; N]) -> Self {
        // basically useless so prohibit it.
        assert!(N > 0, "cannot initialize with empty list");

        let mut common_prefixes = [0; _];
        // check:
        // - no two strings are not completely the same.
        // - they are completely ASCII (for easy indexing).
        let mut i = 0;
        while i < list.len() {
            // if string comparisons and case switch come to const time, this is no longer a
            // limitation.
            assert!(list[i].is_ascii(), "only ASCII values are supported");

            let mut j = i + 1;
            while j < list.len() {
                let a = list[i];
                let b = list[j];
                let eq_up_to = Self::eq_up_to_bytes(a, b);

                // if a map is implemented these are no longer a limitation
                // this is a limitation of crude searching.
                assert!(
                    a.len() != eq_up_to && b.len() != eq_up_to,
                    "one entry is the prefix for another so cannot be uniquely identified"
                );

                if common_prefixes[i] < eq_up_to {
                    common_prefixes[i] = eq_up_to;
                }
                if common_prefixes[j] < eq_up_to {
                    common_prefixes[j] = eq_up_to;
                }

                j += 1;
            }

            i += 1;
        }

        Self {
            values: list,
            common_prefixes,
        }
    }

    /// Match the given key if their prefixes match uniquely regardless of ASCII casing.
    pub const fn position(&self, key: &str) -> Option<usize> {
        let mut i = 0;
        while i < N {
            if key.len() > self.common_prefixes[i]
                && key.len() == Self::eq_up_to_bytes(self.values[i], key)
            {
                return Some(i);
            }

            i += 1;
        }
        None
    }

    /// How many bytes between the two strings is the same if their ASCII ignore case is the same.
    pub const fn eq_up_to_bytes(a: &str, b: &str) -> usize {
        let mut i = 0;

        // `min` is not const compatible
        let min_len = if a.len() < b.len() { a.len() } else { b.len() };
        let a = a.as_bytes();
        let b = b.as_bytes();

        while i < min_len {
            // this is the ignorecase part
            if a[i].to_ascii_lowercase() != b[i].to_ascii_lowercase() {
                return i;
            }
            i += 1;
        }
        min_len
    }
}

/// Parse from 1..=12 the valid month range.
fn parse_month_numeric(s: &str) -> Result<UMonth, &'static str> {
    if let Ok(v) = s.parse() {
        let month: UMonth = v;
        if (Month::MIN.get()..=Month::MAX.get()).contains(&month) {
            return Ok(month);
        }
    }

    // TODO make the error message rely on constants
    Err("month is from 1 to 12 when given as a number")
}

const JALALI_MATCHER: IgnoreCasePrefixMatch<12> = IgnoreCasePrefixMatch::new(JALALI_MONTHS);

const GREGORIAN_MATCHER: IgnoreCasePrefixMatch<12> = IgnoreCasePrefixMatch::new(GREGORIAN_MONTHS);

const WEEKDAYS_MATCHER: IgnoreCasePrefixMatch<7> = IgnoreCasePrefixMatch::new(WEEKDAYS);

fn parse_month_string(matcher: &IgnoreCasePrefixMatch<12>, s: &str) -> Option<UMonth> {
    parse_month_numeric(s)
        .ok()
        .or_else(|| matcher.position(s).map(|i| i as u8 + 1)) // month is 1 based but index is 0 based
}

/// Parse from 1..=12 the valid month range or name of Gregorian months in English.
pub fn parse_month(s: &str) -> Result<UMonth, &'static str> {
    parse_month_string(&JALALI_MATCHER, s)
        .ok_or("invalid month name (\"mehr\" or number where January is 1, up to 12)")
}

/// Parse from 1..=12 the valid month range or name of Gregorian months in English.
pub fn parse_jalali_month(s: &str) -> Result<UMonth, &'static str> {
    parse_month_string(&GREGORIAN_MATCHER, s)
        .ok_or("invalid month name (\"september\" or number where January is 1, up to 12)")
}

pub fn parse_weekday(s: &str) -> Result<Weekday, &'static str> {
    // first try numeric inputs from 0..=6
    if let Ok(v) = s.parse() {
        let weekday: UWeekday = v;
        if !(Weekday::MIN.get()..=Weekday::MAX.get()).contains(&weekday) {
            // TODO make the error message rely on constants
            return Err("weekday is from 0 (Sunday) to 6 (Saturday) when a number \
                 (regardless of the calendar)");
        }
        return Ok(weekday.into());
    }

    match WEEKDAYS_MATCHER.position(s) {
        Some(i) => Ok(Weekday::new(i as u8)), // okay since struct & WEEKDAYS are Sunday based
        None => Err("invalid weekday name (\"sunday\" or number where Sunday is 0, up to 6)"),
    }
}
