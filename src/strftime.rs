//! Holds `strftime`-like functions and related helpers.

use jelal::UMonth;
use jiff::{Zoned, fmt::strtime::BrokenDownTime};

use crate::{JALALI_MONTHS, JALALI_MONTHS_ABB, date::CommonDate};

/// Holds an exploded list of directives and literals.
#[derive(Debug, Clone, PartialEq)]
pub struct Formatter<'a> {
    directives: Vec<(usize, &'a str)>,
    original: &'a str,
}

impl<'a> Formatter<'a> {
    pub fn new(format: &'a str) -> Self {
        let mut chars = format.char_indices().peekable();
        let mut directives = Vec::new();
        let mut selection_start = None;

        while let Some(current) = chars.next() {
            // % then [^%] until [^EO][:ascii:] (a valid directive)
            // structured in a way that not `continue`ing early will add the `current` char to new_fmt

            // this part handles r"%[^%]"
            if current.1 == '%' {
                let next = chars.peek();
                if next.is_some_and(|(_, c)| *c != '%') {
                    selection_start = Some(current.0);
                    continue; // do not add until reaching the end of the selection
                }

                // If here, either:
                // - "%%": which means the selection should ignore the previous directive because it
                //    is definitely not a valid one. Since a directive cannot have an argument
                //    (%[ARGS][END]) that is '%' or "%%" so the input should be removed from
                //    selection to be delegated to the caller to handle.
                // - "%$" (end of string): which also means there should not be any selection since
                //   '%' by itself is not a valid directive so once again the treatment is applied.

                // next = Some('%') then discard the next
                if next.is_some() {
                    chars.next();
                }

                selection_start = None;
                continue;
            }

            // this handles the rest
            //
            // if in a selection (%.* and going onward), check if a successful termination criteria
            // is met. Otherwise just leave it to fail in the future iteration or not.
            if let Some(s) = selection_start.take() {
                // Even though jiff does not support E and O, it will fail regardless but for the
                // sake of POSIX, we handle it. Basically, we ignore if E or O which are the only
                // alphabetic exceptions that do not terminate a directive.
                if current.1.is_ascii_alphabetic() && current.1 != 'E' && current.1 != 'O' {
                    // ..= works since "is_ascii*" then automatilly len is 1 then ..= will not be
                    // on boundaries
                    directives.push((s, &format[s..=current.0]));
                } else {
                    selection_start = Some(s); // continue selection
                }
                continue;
            }
        }

        Self {
            directives,
            original: format,
        }
    }

    /// Reconstruct the values given a "reconstructor" function.
    ///
    /// A reconstructor function takes a value that necessarily starts with "%" and ends with a
    /// directive (not checked whether a valid/known directive or not) and outputs another string to
    /// replace it ("%s" -> "123"). All the details and checks are delegated to the reconstructor.
    pub fn lenient_reconstruct_with<F: Fn(&str) -> Option<String>>(&self, f: F) -> String {
        let mut new = String::with_capacity(self.original.len()); // this usually holds true
        let mut previous_end = 0;
        for (start_index, directive) in self.directives.iter() {
            // fill the gap between this directive and previous
            let left = &self.original[previous_end..*start_index];
            new.push_str(left);
            previous_end += left.len();

            let result = f(directive);

            // if the given function did not return a value, do nothing and keep the value intact
            let s = result.as_ref().map(|i| i.as_str()).unwrap_or(directive);
            new.push_str(s);
            previous_end += directive.len();
        }

        // append the rest
        new.push_str(&self.original[previous_end..]);

        new
    }

    // /// Like [`Self::lenient_reconstruct_with`] but with functions that may fail.
    // pub fn reconstruct_with<F, E>(f: F) -> Result<String, E> {}
}

/// Given a Jalali month (1..=12), create a function that formats `%s`-like directives to its name.
///
/// This is a "reconstructor" function for [`Formatter`] that handles Jalali limitations of `jiff`.
///
/// As documented in `jelal`, `%b`, `%B` and `%h` need to be replaced with proper month names.
/// This only handles those with little to no argument support (suffice is the level of support
/// provided by `jiff`).
///
/// Month is 1..=12. A valid input to this must start with a `%` and end with an ASCII.
pub fn jalali_month_format_resolve(jalali_month: UMonth) -> impl Fn(&str) -> Option<String> {
    move |s: &str| {
        if !s.starts_with('%') {
            return None;
        }

        let arr = if s.ends_with('B') {
            JALALI_MONTHS
        } else if s.ends_with('b') || s.ends_with('h') {
            JALALI_MONTHS_ABB
        } else {
            return None;
        };

        // if here, arr is valid, arg is valid and either uppercase or toggle
        let string = arr[jalali_month as usize - 1];
        Some(if s[1..(s.len() - 1)].contains('^') {
            string.to_uppercase()
        } else {
            // for any other modifier jiff doesn't do anything so we don't either
            string.to_string()
        })
    }
}

/// [`jalali_strftime_to`] a newly created string.
pub fn jalali_strftime(format: &str, now: &Zoned) -> Result<String, jiff::Error> {
    let mut buf = String::new();
    jalali_strftime_to(format, now, &mut buf)?;
    Ok(buf)
}

/// Convert this date to Jalali and put it in the given formatter.
// TODO move to `jelal`
pub fn jalali_strftime_to<W: jiff::fmt::Write>(
    format: &str,
    now: &Zoned,
    mut wtr: W,
) -> Result<(), jiff::Error> {
    let jdate = jelal::Date::from(now.date());

    // jdate.set_to_broken with a BrokenDownTime that is created from a Zoned initializes all fields
    // so any formatter works except `%h`, `%b` and `%B` which are the Gregorian month names
    let bdt = jdate.set_to_broken(BrokenDownTime::from(now))?;

    // This identifies the formatters and replaces them with the given function
    // [`jalali_month_format_resolve`] replaces the aforementioned directives
    let format =
        Formatter::new(format).lenient_reconstruct_with(jalali_month_format_resolve(jdate.month()));

    bdt.format(format, &mut wtr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_identification() {
        assert_eq!(Formatter::new("Hello There").directives, vec![]);
        assert_eq!(Formatter::new("Hello%sThere").directives, vec![(5, "%s")]);
        assert_eq!(Formatter::new("Hello%^sThere").directives, vec![(5, "%^s")]);
        assert_eq!(Formatter::new("%^V").directives, vec![(0, "%^V")]);
        assert_eq!(Formatter::new("%0_V").directives, vec![(0, "%0_V")]);
        assert_eq!(Formatter::new("%%0_V").directives, vec![]);
        assert_eq!(Formatter::new("%__%0_V").directives, vec![(3, "%0_V")]);
        assert_eq!(Formatter::new("%__0_").directives, vec![]);
        assert_eq!(
            Formatter::new("%%%G-W%V(%U)-\u{625}-%u(%j)-%0A%%").directives,
            vec![
                (2, "%G"),
                (6, "%V"),
                (9, "%U"),
                (16, "%u"),
                (19, "%j"),
                (23, "%0A")
            ]
        );
    }

    #[test]
    fn test_strftime_invalid_greg_date_valid_jalali() {
        // 1404/2/31 (2/31 is invalid in Gregorian so if formatter checks the input on that basis,
        // will crash)
        let tm = Zoned::strptime("%Y/%m/%d %z", "2025/05/21 +0000").unwrap();

        assert_eq!(
            "::.1404/02/31.::",
            jalali_strftime("::.%Y/%m/%d.::", &tm).unwrap()
        );

        assert_eq!("%", jalali_strftime("%%", &tm).unwrap());
        assert_eq!("Wednesday", jalali_strftime("%A", &tm).unwrap());
        assert_eq!("Wed", jalali_strftime("%a", &tm).unwrap());
        assert_eq!("Ordibehesht", jalali_strftime("%B", &tm).unwrap());
        assert_eq!("Ord", jalali_strftime("%b", &tm).unwrap());
        assert_eq!("Ord", jalali_strftime("%h", &tm).unwrap());
        assert_eq!("14", jalali_strftime("%C", &tm).unwrap());
        assert_eq!(
            "1404 M02 31, Wed 00:00:00",
            jalali_strftime("%c", &tm).unwrap()
        );
        assert_eq!("02/31/04", jalali_strftime("%D", &tm).unwrap()); // after #432
        assert_eq!("31", jalali_strftime("%d", &tm).unwrap());
        assert_eq!("31", jalali_strftime("%e", &tm).unwrap());
        assert_eq!("1404-02-31", jalali_strftime("%F", &tm).unwrap());
        assert_eq!("0", jalali_strftime("%f", &tm).unwrap());
        assert_eq!("", jalali_strftime("%.f", &tm).unwrap());
        assert_eq!("1404", jalali_strftime("%G", &tm).unwrap()); // ISO 8601 in Jalali
        assert_eq!("04", jalali_strftime("%g", &tm).unwrap()); // ISO 8601 in Jalali
        assert_eq!("00", jalali_strftime("%H", &tm).unwrap());
        assert_eq!("12", jalali_strftime("%I", &tm).unwrap());
        assert_eq!("062", jalali_strftime("%j", &tm).unwrap());
        assert_eq!(" 0", jalali_strftime("%k", &tm).unwrap());
        assert_eq!("12", jalali_strftime("%l", &tm).unwrap());
        assert_eq!("00", jalali_strftime("%M", &tm).unwrap());
        assert_eq!("02", jalali_strftime("%m", &tm).unwrap());
        assert_eq!("000000000", jalali_strftime("%N", &tm).unwrap());
        assert_eq!("\n", jalali_strftime("%n", &tm).unwrap());
        assert_eq!("am", jalali_strftime("%P", &tm).unwrap());
        assert_eq!("AM", jalali_strftime("%p", &tm).unwrap());
        assert_eq!("UTC", jalali_strftime("%Q", &tm).unwrap());
        assert_eq!("UTC", jalali_strftime("%:Q", &tm).unwrap());
        assert_eq!("1", jalali_strftime("%q", &tm).unwrap());
        assert_eq!("00:00", jalali_strftime("%R", &tm).unwrap());
        assert_eq!("12:00:00 AM", jalali_strftime("%r", &tm).unwrap());
        assert_eq!("00", jalali_strftime("%S", &tm).unwrap()); // after #432
        assert_eq!("1747785600", jalali_strftime("%s", &tm).unwrap());
        assert_eq!("00:00:00", jalali_strftime("%T", &tm).unwrap());
        assert_eq!("\t", jalali_strftime("%t", &tm).unwrap());
        assert_eq!("09", jalali_strftime("%U", &tm).unwrap());
        assert_eq!("3", jalali_strftime("%u", &tm).unwrap());
        assert_eq!("09", jalali_strftime("%V", &tm).unwrap()); // ISO 8601 in Jalali
        assert_eq!("09", jalali_strftime("%W", &tm).unwrap());
        assert_eq!("3", jalali_strftime("%w", &tm).unwrap());
        assert_eq!("00:00:00", jalali_strftime("%X", &tm).unwrap());
        assert_eq!("1404 M02 31", jalali_strftime("%x", &tm).unwrap());
        assert_eq!("1404", jalali_strftime("%Y", &tm).unwrap());
        assert_eq!("04", jalali_strftime("%y", &tm).unwrap()); // after #432
        assert_eq!("UTC", jalali_strftime("%Z", &tm).unwrap());
        assert_eq!("+0000", jalali_strftime("%z", &tm).unwrap());
        assert_eq!("+00:00", jalali_strftime("%:z", &tm).unwrap());
        assert_eq!("+00:00:00", jalali_strftime("%::z", &tm).unwrap());
        assert_eq!("+00", jalali_strftime("%:::z", &tm).unwrap());
    }

    #[test]
    fn test_strftime_invalid_greg_date_valid_jalali_args() {
        // 1404/2/31 (2/31 is invalid in Gregorian so if formatter checks the input on that basis,
        // will crash)
        let tm = Zoned::strptime("%Y/%m/%d %z", "2025/05/21 +0000").unwrap();

        // Persian valid arg behvaior as in jiff (see below)
        assert_eq!("ORDIBEHESHT", jalali_strftime("%^B", &tm).unwrap());
        assert_eq!("Ordibehesht", jalali_strftime("%010B", &tm).unwrap());
        assert_eq!("Ordibehesht", jalali_strftime("%#B", &tm).unwrap());
        assert_eq!("Ordibehesht", jalali_strftime("%_10B", &tm).unwrap());
        // jiff valid arg behavior
        assert_eq!("WEDNESDAY", jalali_strftime("%^A", &tm).unwrap());
        assert_eq!("Wednesday", jalali_strftime("%010A", &tm).unwrap());
        assert_eq!("Wednesday", jalali_strftime("%#A", &tm).unwrap());
        assert_eq!("Wednesday", jalali_strftime("%_10A", &tm).unwrap());

        // jiff does not provide more complex behavior like `%#^#010A` so it's not added to this
        // resolver either
    }
}
