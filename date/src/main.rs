//! Holds a `date` program with Jalali options.
//!
//! This is a `coreutils`' `date` compatible `date` program with [`jelal`] springkled on top.
//! `uutils`' `date` is a mature alternative if you don't want [`jelal`].
//!
//! This is not related to any of the aforementioned software. The behavior of this instance may
//! slightly differ. See the `lib.rs` file for that.
//!
//! Differences with `date`:
//! - `jelal` support
//! - does not warn if multiple flags are set for one value and the last one is used only
//! - no support for showing `resolution` and everything is fixed to nano by the libraries used
//!   (this also means no resolution adjustment happens)
//! - no support for localization (`rfc*`, `iso*`)
//! - no support for `set`
//! - parsing datetime is done with mostly `parse_datetime` (POSIX support is extended) crate so its
//!   limitations apply

use std::io::BufRead;

use jcal::{clap_helper::Parse, parser::parse_datetime, strftime::jalali_strftime};

mod arg_parser;

use arg_parser::{Args, When};
use jiff::{Timestamp, Zoned, tz::TimeZone};

use crate::arg_parser::Reader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Args::parse();

    // The rest of the program is the actual logic.
    let zoned = match config.when {
        When::Reader(input_path) => {
            if file_apply(input_path, &config.format, config.timezone, config.jalali) {
                return Ok(());
            } else {
                return Err("failed to parse all lines".into());
            }
        }
        When::Given(v) => v,
        When::Now => Zoned::now().with_time_zone(config.timezone),
        When::Reference(path_buf) => {
            let time = std::fs::File::open(path_buf)?.metadata()?.modified()?;
            Timestamp::try_from(time)?.to_zoned(config.timezone)
        }
    };

    if config.debug {
        eprintln!("output format: `{}`", config.format);
        eprintln!("basis: {}", &zoned);
    }

    print_strftime(&config.format, &zoned, config.jalali);

    Ok(())
}

/// Print time in the given calendar.
fn print_strftime(format: &str, tm: &Zoned, jalali: bool) {
    println!(
        "{}",
        if jalali {
            jalali_strftime(format, tm).unwrap()
        } else {
            tm.strftime(format).to_string()
        }
    )
}

/// Parse each line in a stream as with --date and display each resulting time and date.
///
/// If the file or stream fails to open or yield lines panics. Prints warning for each failed to
/// parse value.
///
/// Returns false if any parsing failed.
// TODO test
fn file_apply(reader: Reader, format: &str, timezone: TimeZone, jalali: bool) -> bool {
    // TODO make an enum
    let read: &mut dyn std::io::Read = match reader {
        Reader::Stdin => &mut std::io::stdin(),
        Reader::File(path) => &mut std::fs::File::open(path).expect("cannot open the file"),
    };
    let mut buf_reader = std::io::BufReader::new(read);

    let mut ok = true;
    let mut buf = String::new();
    let now = Zoned::now().with_time_zone(timezone);
    // 0 is the end of the file
    while buf_reader.read_line(&mut buf).expect("cannot read line") != 0 {
        match parse_datetime(&buf, Some(now.clone())) {
            Ok(tm) => print_strftime(format, &tm, jalali),
            Err(e) => {
                eprintln!("invalid date {}", e);
                ok = false;
            }
        };
        buf.clear();
    }

    ok
}
