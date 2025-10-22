//! A generic module for parsing POSIX related strings.
//!
//! `parse_datetime` almost does this but the difference is that its API is totally private.
//!
//! The main purpose of this module is to parse the following formats and there is not much of a
//! reason for it since other functionalities here are better integrated in other crates so the
//! following are the only formats (and other similar variations) left:
//! - "CCYYMMDDhhmm"
//! - "MMDDhhmmCCYY"
//! - "YYMMDDhhmm"
//! - "MMDDhhmmYY"
//! - "MMDDhhmm"
//! - "MMDDhhmm.SS"
//!
//! See parser methods for more information.
//
// #[test]
// #[ignore = "methods not used in application"]
// fn test_parse_datetime_crate_fails() {
//     let parse = |s| parse_datetime::parse_datetime(s);
//     assert!(parse("140007041924").is_err(), "ccyymmddhhmm");
//     assert!(parse("061507042624").is_err(), "mmddhhmmccyy");
//     assert!(parse("061507042099").is_err(), "mmddhhmmccyy");
//     assert!(parse("0615070424").is_err(), "mmddhhmmyy");
//     assert!(parse("6807041924").is_err(), "yymmddhhmm");
//     assert!(parse("6907041924").is_err(), "yymmddhhmm");
//     assert!(parse("07041924").is_err(), "mmddhhmm");
//     assert!(parse("07041924.30").is_err(), "mmddhhmm.ss");
// }

use std::{
    fmt::{self, Display},
    ops::RangeInclusive,
};

/// The default result of this module.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors given by the POSIX format reader.
#[derive(Debug)]
pub enum Error {
    /// Invalid syntax
    Syntax,
    /// Syntax is known but not allowed for this instance of parser
    Forbidden,
    /// Syntax is valid but values are not in POSIX range.
    OutOfRange,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax => "value is not a valid POSIX string".fmt(f),
            Error::Forbidden => "value is a valid POSIX-like but not allowed".fmt(f),
            Error::OutOfRange => "one of the POSIX string fields is out of its range".fmt(f),
        }
    }
}

impl std::error::Error for Error {}

/// A generic broken time holder (by no means guarantees a valid date).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateTime {
    /// See [`Self::YEAR_RANGE`]
    pub year: Option<u16>,
    /// See [`Self::MONTH_RANGE`]
    pub month: u8,
    /// See [`Self::DAY_RANGE`]
    pub day: u8,
    /// See [`Self::HOUR_RANGE`]
    pub hour: u8,
    /// See [`Self::MINUTE_RANGE`]
    pub minute: u8,
    /// See [`Self::SECOND_RANGE`] and [`Self::SECOND_LEGACY_RANGE`].
    pub second: Option<u8>,
}

impl Default for DateTime {
    fn default() -> Self {
        Self {
            year: None,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: None,
        }
    }
}

impl DateTime {
    /// Valid range for [`Self::year`].
    pub const YEAR_RANGE: RangeInclusive<u16> = 0000..=9999;

    /// Valid range for [`Self::month`].
    pub const MONTH_RANGE: RangeInclusive<u8> = 01..=12;

    /// Valid range for [`Self::day`].
    pub const DAY_RANGE: RangeInclusive<u8> = 01..=31;

    /// Valid range for [`Self::hour`].
    pub const HOUR_RANGE: RangeInclusive<u8> = 00..=23;

    /// Valid range for [`Self::minute`].
    pub const MINUTE_RANGE: RangeInclusive<u8> = 00..=59;

    /// Valid range for [`Self::second`] (the end results will be in this, inputs may differ).
    ///
    /// Inputs are valid if they match [`Self::SECOND_SATURATING_MAX`] but saturated to this.
    pub const SECOND_RANGE: RangeInclusive<u8> = Self::SECOND_MIN..=Self::SECOND_MAX;

    /// The lower bound of the second range.
    pub const SECOND_MIN: u8 = 00;

    /// The upper bound of the second range.
    pub const SECOND_MAX: u8 = 60;

    /// Given this number, it will be accepted but also saturated to max of [`Self::SECOND_RANGE`].
    ///
    /// Values larger than this will be outright rejected as out of range.
    pub const SECOND_SATURATING_MAX: u8 = 61;

    /// [`Self::parse`] but if "MMDDhhmm" misses digits, prioritize time and then date to set zero.
    ///
    /// This supports ".SS" regardless of the given format.
    pub fn parse_loose(
        chars: &str,
        mut prioritize_trailing: bool,
        now_month: u8,
        now_day: u8,
    ) -> Result<Self> {
        // easy access to indices
        if !chars.is_ascii() {
            return Err(Error::Syntax);
        }

        let (chars, ss) = chars.split_once('.').unwrap_or((chars, "00"));

        let (hh, mm);
        let chars = match chars.len() {
            i @ 0..=4 => {
                (hh, mm) = match i {
                    0 => ("", ""),
                    1 | 2 => (chars, ""),
                    3 | 4 => (&chars[0..(i - 2)], &chars[(i - 2)..]),
                    _ => unreachable!(),
                };
                format_args!("{:0>2}{:0>2}{:0>2}{:0>2}", now_month, now_day, hh, mm)
            }
            5 | 7 => {
                prioritize_trailing = false;
                format_args!("0{:0>7}0000", chars)
            }
            6 => {
                prioritize_trailing = false;
                format_args!("{}0000", chars)
            }
            // since jiff and others don't parse large values, there is no point parsing past 7
            _ => format_args!("{}", chars),
        };

        let chars = &format!("{}.{:0>2}", chars, ss);
        Self::parse(chars, prioritize_trailing)
    }

    /// Parse a POSIX Time format.
    ///
    /// A POSIX Time format is either of the following:
    /// - "[[CC]YY]MMDDhhmm[.SS]" (called "normal" in this struct documents as it is in `touch`).
    /// - "MMDDhhmm[CCYY]" or "trailing".
    /// - "MMDDhhmm[YY]" (with "YY" in `69..=99`) which is obsolete (see page 724 of
    ///   <https://pubs.opengroup.org/onlinepubs/009639599/toc.pdf>).
    ///
    /// This only supports positive values.
    ///
    /// With combination of flags, this parser also allows for "invalid" (but sensible) variations:
    /// - Allow second ("[.SS]") for all variants above.
    ///
    /// "CC" is 20 for 00..=68 and 19 for 69..=99.
    pub fn parse(chars: &str, prioritize_trailing: bool) -> Result<Self> {
        let chars = chars.chars().collect::<Vec<_>>();
        let (chars, ss) = {
            let mut dot_split = chars.as_slice().splitn(2, |&c| c == '.');
            (dot_split.next().unwrap(), dot_split.next())
        };

        let mut candidate = Self::parse_no_second(chars, prioritize_trailing)
            .or_else(|_| Self::parse_no_second(chars, !prioritize_trailing))?;

        if let Some(ss) = ss {
            candidate.set_ss(ss)?;
        }

        Ok(candidate)
    }

    /// Just like [`Self::parse`] but do not process seconds.
    pub fn parse_no_second(chars: &[char], prioritize_trailing: bool) -> Result<Self> {
        // Take 8 characters from start or end of a value and return the remainer and the taken.
        let (ccyy, mmddhhmm) = if prioritize_trailing {
            chars.split_first_chunk::<8>().map(|(a, b)| (b, a))
        } else {
            chars.split_last_chunk::<8>()
        }
        .ok_or(Error::Syntax)?;

        let mut candidate = Self::try_from_mmddhhmm(mmddhhmm)?;
        match ccyy.len() {
            0 => {}
            2 | 4 => {
                let (may_cc, yy) = ccyy.split_last_chunk::<2>().unwrap();
                candidate.set_cc_yy(may_cc.first_chunk::<2>(), yy)?;
            }
            _ => return Err(Error::Syntax),
        };

        Ok(candidate)
    }

    /// Return seconds but saturate at 59.
    pub fn second_min_59(&self) -> Option<u8> {
        self.second.map(|i| i.min(59))
    }

    /// Create a new instance.
    pub fn new(month: u8, day: u8, hour: u8, minute: u8) -> Result<Self> {
        if [
            Self::MONTH_RANGE.contains(&month),
            Self::DAY_RANGE.contains(&day),
            Self::HOUR_RANGE.contains(&hour),
            Self::MINUTE_RANGE.contains(&minute),
        ]
        .into_iter()
        .all(|i| i)
        {
            Ok(Self {
                month,
                day,
                hour,
                minute,
                ..Default::default()
            })
        } else {
            Err(Error::OutOfRange)
        }
    }

    /// Set the year if in range.
    pub fn set_year(&mut self, year: u16) -> Result<&mut Self> {
        if Self::YEAR_RANGE.contains(&year) {
            self.year = Some(year);
            Ok(self)
        } else {
            Err(Error::OutOfRange)
        }
    }

    /// Set the second if in the current or legacy range.
    pub fn set_second(&mut self, second: u8) -> Result<&mut Self> {
        // The first check does nothing since the number is 0 <= anyways but it must be here for
        // the sake of consistency
        if (Self::SECOND_MIN..=Self::SECOND_SATURATING_MAX).contains(&second) {
            self.second = Some(Self::SECOND_MAX.min(second)); // saturate
            Ok(self)
        } else {
            Err(Error::OutOfRange)
        }
    }

    /// Create from the mandatory datetime section.
    pub fn try_from_mmddhhmm(mmddhhmm: &[char; 8]) -> Result<Self> {
        // as_chunks map collect in simpler ways
        Self::new(
            Self::two_as_num(&mmddhhmm[0..2])?,
            Self::two_as_num(&mmddhhmm[2..4])?,
            Self::two_as_num(&mmddhhmm[4..6])?,
            Self::two_as_num(&mmddhhmm[6..8])?,
        )
    }

    /// Given a string of "[CC]YY" digits, set it to the fields if valid.
    ///
    /// As goes with POSIX, when no "CC" is given but "YY" is present:
    /// - "CC" is 20 for "YY" strictly under 69.
    /// - "CC" is 19 for "YY" above and including 69.
    pub fn set_cc_yy(&mut self, cc: Option<&[char; 2]>, yy: &[char; 2]) -> Result<&mut Self> {
        let yy: u16 = Self::two_as_num(yy)? as _;

        let cc: u16 = match cc {
            Some(v) => Self::two_as_num(v)? as _,
            None if yy < 69 => 20,
            None /* yy >= 69 */ => 19,
        };

        // this won't fail if here but still a setter looks nicer
        self.set_year(cc * 100 + yy)
    }

    /// Set the seconds from the given string if valid.
    pub fn set_ss(&mut self, ss: &[char]) -> Result<&mut Self> {
        self.set_second(Self::two_as_num(ss)?)
    }

    /// If two digits, convert as if written in succession.
    fn two_as_num(pair: &[char]) -> Result<u8> {
        if pair.len() != 2 {
            return Err(Error::Syntax);
        }

        let get = |i: usize| pair[i].to_digit(10).ok_or(Error::Syntax);

        Ok((get(0)? * 10 + get(1)?) as _)
    }
}

/// Given an initial variable like `VAR="X"REST` return `X` trimmed and `REST`.
///
/// This will still pass if `VAR` has whitespace before it.
///
/// The comments here assume that the `open_delimiter` is '"' and `close_delimiter` is also `"` but
/// they can vary. Same goes for `infix` which is assumed to be '='.
///
/// If `VAR="` (ignoring the whitespaces) does not prefix the string, Some(None) will be returned
/// and the rest just trimmed. In any other case, the function tries to extract `X` and should it
/// fail, returns None.
///
/// Returns:
/// - Some(Some(X), REST): the value of `VAR` (trimmed) and `REST`.
/// - Some(None, REST): REST and no `VAR` value was found.
/// - None: there was syntax error in `VAR` declaration.
///
/// The given string supports escaped slash ("\\") and escaped close delimiter ("\"") but not much
/// else.
fn parse_var_prefix<'proc, 'src>(
    var: &'proc str,
    infix: &'proc str,
    open_delimiter: &'proc char,
    close_delimiter: &'proc char,
    src: &'src str,
) -> Option<(Option<&'src str>, &'src str)> {
    // if found `VAR="` remove it, else just return that the var is not found (don't touch)
    let Some(src) = src
        .trim_start()
        .strip_prefix(var)
        .and_then(|s| s.strip_prefix(infix))
        .and_then(|s| s.strip_prefix(*open_delimiter))
    else {
        return Some((None, src));
    };

    // determine where it all ends (non-inclusive)
    let end_i = {
        let mut chars = src.char_indices();
        let mut end_i = 0;
        let mut closed = false;
        while let Some((c_i, c)) = chars.next() {
            end_i = c_i;
            if c == '\\' {
                // skip the next and don't terminate if its just an escape (using next again)
                if let Some((next_i, next_c)) = chars.next() {
                    end_i = next_i;
                    if next_c != '\\' && next_c != *close_delimiter {
                        return None; // err: format!("unsupported escape sequence '\\{next_c}'");
                    }
                }
            } else if c == *close_delimiter {
                closed = true;
                break;
            }
        }
        if !closed {
            return None; // err: "delimiter not closed"
        }
        end_i
    };

    let value = &src[..end_i].trim(); // also trim inside quotes

    // since its sure that the quote_closed is here, end_i + 1 is valid
    let rest = &src[(end_i + close_delimiter.len_utf8())..];

    Some((Some(value), rest))
}

/// Given a string, try to take out the trimmed initial `TZ="X"` and return "X" and also the rest.
///
/// This does not perform any checks on the string whatsoever.
pub fn take_timezone(s: &str) -> Option<(Option<&str>, &str)> {
    parse_var_prefix("TZ", "=", &'"', &'"', s)
}

/// Parse a `TZ="TIMEZONE"` prefix. If cannot parse, will return None.
// this was initially a dedicated implementation using both TimeZone::{get,posix} but it's delegated
// to parse_datetime since their implementation covers more altho it also throws in some fixed
// offset which jiff frowns upon
pub fn parse_timezone(s: &str) -> (Option<jiff::tz::TimeZone>, &str) {
    let original_s = s;
    let s = s.trim_start();

    // parse taking out TZ="" to validate and trim whitespaces then giving it back
    // https://github.com/uutils/parse_datetime/issues/240
    if let Some((Some(s), rest)) = take_timezone(s) {
        if let Ok(v) = parse_datetime::parse_datetime(&format!("TZ=\"{}\"", s)) {
            return (Some(v.time_zone().clone()), rest);
        }
    }

    (None, original_s)
}

impl From<Error> for jiff::Error {
    fn from(value: Error) -> Self {
        jiff::Error::from_args(format_args!("{}", value))
    }
}

impl DateTime {
    /// Convert to Zoned with the given year basis and keep seconds 0 if not given (as in `date`).
    pub fn to_datetime(self, year_basis: i16) -> Result<jiff::civil::DateTime, jiff::Error> {
        jiff::civil::DateTime::new(
            self.year.map(|i| i as i16).unwrap_or(year_basis),
            self.month as i8,
            self.day as i8,
            self.hour as i8,
            self.minute as i8,
            self.second_min_59().unwrap_or(0) as i8,
            0,
        )
    }
}

impl TryFrom<jiff::civil::DateTime> for DateTime {
    type Error = Error;

    fn try_from(value: jiff::civil::DateTime) -> std::result::Result<Self, Self::Error> {
        let mut candidate = Self {
            year: Default::default(),
            month: value.month() as u8,
            day: value.day() as u8,
            hour: value.hour() as u8,
            minute: value.minute() as u8,
            second: Some(value.second() as u8),
        };
        candidate.set_year(value.year() as u16)?; // large values will fail before `as` effects
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use jiff::{Zoned, civil::DateTime as Jdt};
    use std::str::FromStr;

    fn parse_posix(s: &str, trailing: bool, year: Option<u16>, second: u8) -> DateTime {
        let mut dt = DateTime::parse(s, trailing).expect("invalid POSIX in tests");
        if let Some(year) = year {
            dt.set_year(year).expect("cannot set year");
        }
        dt.set_second(second).expect("cannot set second");
        dt
    }

    fn parse_posix_loose(
        s: &str,
        trailing: bool,
        year: Option<u16>,
        month: u8,
        day: u8,
    ) -> DateTime {
        let mut dt =
            DateTime::parse_loose(s, trailing, month, day).expect("invalid POSIX in tests");
        if let Some(year) = year {
            dt.set_year(year).expect("cannot set year");
        }
        dt
    }

    fn parse_jiff(s: &str) -> DateTime {
        Jdt::from_str(s)
            .expect("invalid string")
            .try_into()
            .unwrap()
    }

    #[test]
    fn test_mmddhhmm_priority_ignore() {
        assert_eq!(
            DateTime::parse("06150704", false).unwrap(),
            DateTime::parse("06150704", true).unwrap(),
        );
    }

    #[test]
    fn test_mmddhhmmyy() {
        assert_eq!(
            parse_jiff("2024-06-15T07:04"),
            parse_posix("0615070424", false, Some(2024), 0),
        );
        assert_eq!(
            parse_jiff("1999-06-15T07:04"),
            parse_posix("0615070424", false, Some(1999), 0),
        );
    }

    #[test]
    fn test_priority_effect() {
        assert_eq!(
            parse_jiff("0615-07-04T19:24"),
            parse_posix("061507041924", false, None, 0),
        );
        assert_eq!(
            parse_jiff("1924-06-15T07:04"),
            parse_posix("061507041924", true, None, 0),
        );
    }

    #[test]
    fn test_mmddhhmmccyy() {
        assert_eq!(
            parse_jiff("2624-06-15T07:04"),
            parse_posix("061507042624", true, None, 0),
        );
        assert_eq!(
            parse_jiff("2099-06-15T07:04"),
            parse_posix("061507042099", true, None, 0),
        );
    }

    #[test]
    fn test_mmddhhmm() {
        let year = Zoned::now().year();
        assert_eq!(
            parse_jiff(&format!("{}-07-04T19:24", year)),
            parse_posix("07041924", true, Some(year as u16), 0),
        );
    }

    #[test]
    fn test_mmddhhmmss() {
        let year = Zoned::now().year();
        assert_eq!(
            parse_jiff(&format!("{}-07-04T19:24:30", year)),
            parse_posix("07041924.30", false, Some(year as u16), 30),
        );
    }

    #[test]
    fn test_yymmddhhmm() {
        assert_eq!(
            parse_jiff("2068-07-04T19:24"),
            parse_posix("6807041924", false, None, 0),
        );
        assert_eq!(
            parse_jiff("1969-07-04T19:24"),
            parse_posix("6907041924", false, None, 0),
        );
    }

    #[test]
    fn test_ccyymmddhhmm() {
        assert_eq!(
            parse_jiff("1400-07-04T19:24"),
            parse_posix("140007041924", false, None, 0),
        );
    }

    #[test]
    fn test_loose_empty() {
        assert_eq!(
            parse_jiff("1400-07-04T00:00"),
            parse_posix_loose("", false, Some(1400), 07, 04),
        );
    }

    #[test]
    fn test_loose_one_char() {
        assert_eq!(
            parse_jiff("1400-07-04T03:00"),
            parse_posix_loose("3", false, Some(1400), 07, 04),
        );
    }

    #[test]
    fn test_loose_two_char() {
        assert_eq!(
            parse_jiff("1400-07-04T23:00"),
            parse_posix_loose("23", false, Some(1400), 07, 04),
        );
    }

    #[test]
    fn test_loose_three_char() {
        assert_eq!(
            parse_jiff("1400-07-04T01:23"),
            parse_posix_loose("123", false, Some(1400), 07, 04),
        );
    }

    #[test]
    fn test_loose_four_char() {
        assert_eq!(
            parse_jiff("1400-07-04T21:23"),
            parse_posix_loose("2123", false, Some(1400), 07, 04),
        );
    }

    #[test]
    fn test_loose_five_char() {
        assert_eq!(
            parse_jiff("0008-11-22T00:00"),
            parse_posix_loose("81122", false, None, 07, 04),
        );
    }

    #[test]
    fn test_loose_six_char() {
        assert_eq!(
            parse_jiff("2012-11-13T00:00"),
            parse_posix_loose("121113", false, None, 07, 04),
        );
        assert_eq!(
            parse_jiff("1969-11-13T00:00"),
            parse_posix_loose("691113", false, None, 07, 04),
        );
    }

    #[test]
    fn test_loose_seven_char() {
        assert_eq!(
            parse_jiff("0412-11-13T00:00"),
            parse_posix_loose("4121113", false, None, 07, 04),
        );
        assert_eq!(
            parse_jiff("0469-11-13T00:00"),
            parse_posix_loose("4691113", false, None, 07, 04),
        );
    }

    #[test]
    fn test_parse_tz() {
        // the current parser is compared with `parse_datetime`'s since that's the most complete
        // the testing here then is about "is the initial parser working correctly" or not
        let op = |s: &str| match parse_datetime::parse_datetime(s) {
            Ok(v) => v.time_zone().clone(),
            Err(e) => panic!("{s:?} throws: {e}"),
        };
        // test that this does not confuse named offsets with TZ
        assert_eq!(
            parse_timezone("TZ=\"UTC+1\""),
            (Some(op("TZ=\"UTC+1\"")), "")
        );
        assert_eq!(
            parse_timezone("TZ=\"UTC-1\""),
            (Some(op("TZ=\"UTC-1\"")), "")
        );
        assert_eq!(
            parse_timezone("TZ=\"UTC-1\" "),
            (Some(op("TZ=\"UTC-1\" ")), " ")
        );
        assert_eq!(
            parse_timezone("TZ=\"UTC-1\"\t"),
            (Some(op("TZ=\"UTC-1\"")), "\t")
        );

        // when quotes have inner whitespace, `parse_datetime` produces invalid results.
        // as of now, it's not obvious if this is an "expected" behavior or not.
        //
        // the following cases have this minor change between the string given to `parse_timezone`
        // and the default implementation. Note the whitespaces inside the quotation.
        //
        // See also https://github.com/uutils/parse_datetime/pull/232#issuecomment-3421283917
        assert_eq!(
            parse_timezone("TZ=\"UTC-1 \""),
            (Some(op("TZ=\"UTC-1\"")), "")
        );
        assert_eq!(
            parse_timezone("TZ=\"\tUTC-1\"\t"),
            (Some(op("TZ=\"UTC-1\"")), "\t")
        );
        assert_eq!(
            parse_timezone("\tTZ=\"UTC-1\"\t"),
            (Some(op("TZ=\"UTC-1\"")), "\t")
        );

        assert_eq!(parse_timezone("UTC+1"), (None, "UTC+1"));
        assert_eq!(parse_timezone("UTC-1"), (None, "UTC-1"));
        assert_eq!(parse_timezone("UTC-1 "), (None, "UTC-1 "));
        assert_eq!(parse_timezone("UTC-1\t"), (None, "UTC-1\t"));
        assert_eq!(parse_timezone("\tUTC-1\t"), (None, "\tUTC-1\t"));

        assert_eq!(parse_timezone("\tNO TIME ZONE"), (None, "\tNO TIME ZONE"));
        // assert_eq!(op(""), jiff::tz::TimeZone::system());

        assert_eq!(
            parse_timezone("\tTZ=\"\"\tELSE"),
            (Some(jiff::tz::TimeZone::UTC), "\tELSE")
        );
        assert_eq!(
            parse_timezone("\tTZ=\"\t\"\tELSE"),
            (Some(jiff::tz::TimeZone::UTC), "\tELSE")
        );
        // assert_eq!(op("TZ=\"\""), jiff::tz::TimeZone::UTC);
    }
}
