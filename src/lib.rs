//! Common utilities for `date` and `cal`.
pub mod clap_helper;
pub mod date;
pub mod parser;
pub mod posix;
pub mod strftime;

/// Sunday based weekdays in English.
pub const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// [`WEEKDAYS`] abbreviations to 3 letters.
pub const WEEKDAYS_ABB: [&str; 7] = abbr_strarr(WEEKDAYS);

/// Gregorian months in English.
pub const GREGORIAN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// [`GREGORIAN_MONTHS`] abbreviations to 3 letters.
pub const GREGORIAN_MONTHS_ABB: [&str; 12] = abbr_strarr(GREGORIAN_MONTHS);

/// Jalali months in English.
// Note to future self: these are popular, known and accepted, officially and non-officially.
// do NOT change!
pub const JALALI_MONTHS: [&str; 12] = [
    "Farvardin",
    "Ordibehesht",
    "Khordad",
    "Tir",
    "Mordad",
    "Shahrivar",
    "Mehr",
    "Aban",
    "Azar",
    "Dey",
    "Bahman",
    "Esfand",
];

/// [`JALALI_MONTHS`] abbreviations to 3 letters.
pub const JALALI_MONTHS_ABB: [&str; 12] = abbr_strarr(JALALI_MONTHS);

/// Abbreviate to 3 letters.
const fn abbr_strarr<const N: usize>(original: [&str; N]) -> [&str; N] {
    const CHARS: usize = 3;

    let mut v = [""; N];
    let mut i = 0;
    while i < original.len() {
        assert!(
            original[i].is_ascii() && original[i].len() >= CHARS,
            "automatic abbrevations only work with ASCII strings with enough length",
        );

        // a way around Index not being in const
        v[i] = unsafe {
            str::from_utf8_unchecked(
                original[i]
                    .as_bytes()
                    .first_chunk::<CHARS>()
                    .unwrap()
                    .as_slice(),
            )
        };
        i += 1;
    }
    v
}
