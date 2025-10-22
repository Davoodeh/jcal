use std::{convert::Infallible, path::PathBuf, str::FromStr};

use clap::{
    Arg, ArgAction, ArgGroup, ArgMatches, Command, CommandFactory, FromArgMatches, command,
    error::ErrorKind, value_parser,
};
use jiff::{Zoned, civil::Time, tz::TimeZone};

use jcal::{clap_helper::*, parser::*, posix};

/// Provides lines each having a date to parse.
#[derive(Debug, Clone, PartialEq)]
pub enum Reader {
    File(PathBuf),
    Stdin,
}

#[derive(Debug, PartialEq)]
pub enum When {
    /// Delay the value as far as possible.
    Now,
    /// The content of a file formatted with a string.
    Reader(Reader),
    /// The edit time of a file as set in `reference` flag
    Reference(PathBuf),
    /// The given time.
    Given(Zoned),
    // /// Do not print the current date and time (for resolution for example)
    // None, // or perhaps Resolution? maybe even Option<When> + check where it came from?
}

#[derive(Debug, PartialEq)]
pub struct Args {
    pub format: String,
    pub timezone: TimeZone,
    pub when: When,
    pub debug: bool,
    pub jalali: bool,
}

impl Args {
    pub const DEBUG_LONG: &str = "debug";
    pub const UTC_LONG: &str = "utc";
    pub const DATE_LONG: &str = "date";
    pub const FILE_LONG: &str = "file";
    pub const REFERENCE_LONG: &str = "reference";
    pub const JALALI_LONG: &str = "jalali";
    pub const GREGORIAN_LONG: &str = "gregorian";
    // pub const RESOLUTION_LONG: & str = "resolution";
    pub const RFC_3339_LONG: &str = "rfc-3339";
    pub const RFC_3339_PAIRS: StaticMap<&'static str> = StaticMap(&[
        ("date", "%Y-%m-%d"),
        ("seconds", "%Y-%m-%d %H:%M:%S%:z"),
        ("ns", "%Y-%m-%d %H:%M:%S.%N%:z"),
    ]);
    pub const RFC_EMAIL_LONG: &str = "rfc-email";
    pub const ISO_8601_LONG: &str = "iso-8601";
    pub const ISO_8601_DEFAULT: &str = "date";
    pub const ISO_8601_PAIRS: StaticMap<&'static str> = StaticMap(&[
        (Self::ISO_8601_DEFAULT, "%Y-%m-%d"),
        ("hours", "%Y-%m-%dT%H%:z"),
        ("minutes", "%Y-%m-%dT%H:%M%:z"),
        ("seconds", "%Y-%m-%dT%H:%M:%S%:z"),
        ("ns", "%Y-%m-%dT%H:%M:%S,%N%:z"),
    ]);
    pub const POSITIONAL_ID: &str = "opt";

    pub const DATE_SETTERS_GROUP: &str = "whens";
    pub const DATE_SETTERS_ARGS: &[&str] = &[
        Self::REFERENCE_LONG,
        Self::FILE_LONG,
        Self::DATE_LONG,
        Self::GREGORIAN_LONG,
    ];

    pub const FORMAT_SETTERS_GROUP: &str = "formatters";
    pub const FORMAT_SETTERS_ARGS: &[&str] = &[
        Self::ISO_8601_LONG,
        Self::RFC_3339_LONG,
        Self::RFC_EMAIL_LONG,
    ];

    pub const RFC_EMAIL_FORMAT: &str = "%a, %d %b %Y %H:%M:%S %z";
    pub const DEFAULT_FORMAT: &str = "%a %b %e %H:%M:%S %Z %Y";

    pub fn groups() -> [ArgGroup; 2] {
        [
            ArgGroup::new(Self::DATE_SETTERS_GROUP)
                .multiple(false)
                .args(Self::DATE_SETTERS_ARGS),
            ArgGroup::new(Self::FORMAT_SETTERS_GROUP)
                .multiple(true)
                .args(Self::FORMAT_SETTERS_ARGS),
        ]
    }

    pub fn args() -> [Arg; 11] {
        [
            Arg::new(Self::JALALI_LONG)
                .long(Self::JALALI_LONG)
                .short('j')
                .help("print this date in Jalali")
                .action(ArgAction::SetTrue),
            Arg::new(Self::DEBUG_LONG)
                .long(Self::DEBUG_LONG)
                .help("enable minor extra logs in STDERR")
                .action(ArgAction::SetTrue),
            // general flags
            Arg::new(Self::UTC_LONG)
                .long(Self::UTC_LONG)
                .short('u')
                .visible_alias("uct")
                .visible_alias("universal")
                .help("as if timezone is Coordinated Universal Time (UTC)")
                .action(ArgAction::SetTrue),
            Arg::new(Self::GREGORIAN_LONG)
                .long(Self::GREGORIAN_LONG)
                .short('g')
                .value_name("%Y/%m/%d")
                .help("print the given Jalali date in Gregorian"),
            Arg::new(Self::DATE_LONG)
                .long(Self::DATE_LONG)
                .short('d')
                .overrides_with(Self::DATE_LONG)
                .help("as if `now` is the given (only the last of multiple values takes effect)"),
            // .value_parser should delegate this since the value may need a custom format
            Arg::new(Self::FILE_LONG)
                .long(Self::FILE_LONG)
                .short('f')
                .help("read a file or STDIN for dates (use '-' for STDIN)")
                .value_parser(|s: &str| -> Result<Reader, Infallible> {
                    Ok(if s == "-" {
                        Reader::Stdin
                    } else {
                        Reader::File(PathBuf::from_str(s)?)
                    })
                }),
            Arg::new(Self::REFERENCE_LONG)
                .long(Self::REFERENCE_LONG)
                .short('r')
                .help("as if `now` is the modification time of the given file")
                .value_parser(value_parser!(PathBuf)),
            // arg!(RESOLUTION_LONG)
            // "formatters"
            // edit match_format funciton for parsing
            Arg::new(Self::RFC_EMAIL_LONG)
                .long(Self::RFC_EMAIL_LONG)
                .alias("rfc-822")
                .alias("rfc-2822")
                .overrides_with_all(Self::FORMAT_SETTERS_ARGS)
                .help("output in the specification of RFC 5322")
                .action(ArgAction::SetTrue),
            Arg::new(Self::RFC_3339_LONG)
                .long(Self::RFC_3339_LONG)
                .value_name("SPEC")
                .overrides_with_all(Self::FORMAT_SETTERS_ARGS)
                .help("output in a specification of RFC 3339")
                .value_parser(Self::RFC_3339_PAIRS),
            Arg::new(Self::ISO_8601_LONG)
                .long(Self::ISO_8601_LONG)
                .short('I')
                .value_name("SPEC")
                .num_args(0..=1) // if not given don't push the default
                .default_missing_value(Self::ISO_8601_DEFAULT)
                .overrides_with_all(Self::FORMAT_SETTERS_ARGS)
                .help(format!(
                    "output in a specification of RFC 3339 [default SPEC: {}]",
                    Self::ISO_8601_DEFAULT,
                ))
                .value_parser(Self::ISO_8601_PAIRS),
            // positionals
            Arg::new(Self::POSITIONAL_ID)
                .value_name("INPUT")
                .help("`MMDDhhmm[[CC]YY][.ss]` (POSIX) or a `+FORMAT` (without marks)"),
        ]
    }
}

impl CommandFactory for Args {
    fn command() -> Command {
        command!(/* with version, about and author */)
            .after_help(
                "The formatter syntax is as standard as it gets.\n\
                 Consult https://docs.rs/jiff/latest/jiff/fmt/strtime/index.html and other\n\
                 `date --help` on other implementation.",
            )
            // TODO add a -c/--calendar that passes to jiff-icu
            .args(Self::args())
            .groups(Self::groups())
    }

    fn command_for_update() -> Command {
        Self::command()
    }
}

impl Default for Args {
    /// `date` compatible defaults.
    fn default() -> Self {
        Self {
            format: Self::DEFAULT_FORMAT.to_owned(),
            timezone: TimeZone::system(),
            when: When::Now,
            debug: false,
            jalali: false,
        }
    }
}

impl FromArgMatches for Args {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let mut v = Self::default();
        v.update_from_arg_matches(matches)?;
        Ok(v)
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        if matches.get_flag(Self::UTC_LONG) {
            self.timezone = TimeZone::UTC;
        };

        let now = Zoned::now().with_time_zone(self.timezone.clone());

        self.debug = self.debug || matches.get_flag(Self::DEBUG_LONG);
        self.jalali = self.jalali || matches.get_flag(Self::JALALI_LONG);

        if let Some(v) = matches.get_one::<&'static str>(Self::RFC_3339_LONG) {
            self.format = v.to_string();
        } else if let Some(v) = matches.get_one::<&'static str>(Self::ISO_8601_LONG) {
            self.format = v.to_string();
        } else if matches.get_flag(Self::RFC_EMAIL_LONG) {
            self.format = Self::RFC_EMAIL_FORMAT.to_string();
        }

        // try date, then gregorian, then file, then reference
        if let Some(v) = matches.get_one::<String>(Self::DATE_LONG) {
            self.when = match parse_datetime(v, Some(now.clone())) {
                Ok(v) => When::Given(v),
                Err(e) => return Err(Self::error(ErrorKind::InvalidValue, e)),
            };
        } else if let Some(v) = matches.get_one::<String>(Self::GREGORIAN_LONG) {
            self.when = match parse_ymd_jalali(v).and_then(|i| i.try_into()) {
                Ok(v) => When::Given(now.with().date(v).time(Time::midnight()).build().unwrap()),
                Err(e) => return Err(Self::error(ErrorKind::InvalidValue, e)),
            };
        } else if let Some(v) = matches.get_one::<Reader>(Self::FILE_LONG) {
            self.when = When::Reader(v.clone());
        } else if let Some(v) = matches.get_one::<PathBuf>(Self::REFERENCE_LONG) {
            self.when = When::Reference(v.clone());
        }

        // custom validation for INPUT (POSIX / +FORMAT)
        if let Some(input) = matches.get_one::<String>(Self::POSITIONAL_ID) {
            if input.starts_with('+') {
                if matches.is_explicit(Self::FORMAT_SETTERS_GROUP) {
                    return Err(Self::error(
                        ErrorKind::ArgumentConflict,
                        "unexpected +FORMAT when other options set the format",
                    ));
                }

                self.format = input[1..].to_owned();
            } else {
                if matches.is_explicit(Self::DATE_SETTERS_GROUP) {
                    return Err(Self::error(
                        ErrorKind::ArgumentConflict,
                        "a flag already set the date so positional date is not allowed",
                    ));
                }

                self.when = When::Given(
                    posix::DateTime::parse(input, true)
                        .map_err(|e| e.to_string())
                        .and_then(|tm| {
                            tm.to_datetime(now.year())
                                .and_then(|i| i.to_zoned(self.timezone.clone()))
                                .map_err(|e| e.to_string())
                        })
                        .map_err(|e| Self::error(ErrorKind::InvalidValue, e))?,
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn call(no_0_args: &[&str]) -> Args {
        let matches = Args::command()
            .no_binary_name(true)
            .get_matches_from(no_0_args);
        Args::from_arg_matches(&matches).unwrap()
    }

    #[test]
    fn test_cli_default() {
        assert_eq!(
            call(&[]),
            Args {
                format: Args::DEFAULT_FORMAT.to_owned(),
                timezone: TimeZone::system(),
                when: When::Now,
                debug: false,
                jalali: false
            }
        );
    }

    #[test]
    fn test_cli_debug() {
        assert_eq!(
            call(&["--debug"]),
            Args {
                format: Args::DEFAULT_FORMAT.to_owned(),
                timezone: TimeZone::system(),
                when: When::Now,
                debug: true,
                jalali: false
            }
        );
    }

    #[test]
    fn test_cli_format_rfc_3339_date() {
        assert_eq!(
            call(&["--rfc-3339", "date"]),
            Args {
                format: Args::RFC_3339_PAIRS.get("date").unwrap().to_string(),
                timezone: TimeZone::system(),
                when: When::Now,
                debug: false,
                jalali: false
            }
        );
    }

    #[test]
    fn test_cli_format_iso_8601_date() {
        assert_eq!(
            call(&["--iso-8601", "date"]),
            Args {
                format: Args::ISO_8601_PAIRS.get("date").unwrap().to_string(),
                timezone: TimeZone::system(),
                when: When::Now,
                debug: false,
                jalali: false
            }
        );
    }

    #[test]
    fn test_cli_format_rfc_email() {
        assert_eq!(
            call(&["--rfc-email"]),
            Args {
                format: Args::RFC_EMAIL_FORMAT.to_owned(),
                timezone: TimeZone::system(),
                when: When::Now,
                debug: false,
                jalali: false
            }
        );
    }

    #[test]
    fn test_cli_format_last_take_precedence() {
        assert_eq!(
            call(&["-I", "--rfc-3339", "seconds", "--rfc-3339", "ns"]),
            Args {
                format: Args::RFC_3339_PAIRS.get("ns").unwrap().to_string(),
                timezone: TimeZone::system(),
                when: When::Now,
                debug: false,
                jalali: false
            }
        );

        assert_eq!(
            call(&["--rfc-email", "-I"]),
            Args {
                format: Args::ISO_8601_PAIRS
                    .get(Args::ISO_8601_DEFAULT)
                    .unwrap()
                    .to_string(),
                timezone: TimeZone::system(),
                when: When::Now,
                debug: false,
                jalali: false
            }
        );
    }

    #[test]
    fn test_cli_jalali_to_gregorian() {
        assert_eq!(
            call(&["-g", "1404/07/12"]),
            Args {
                format: Args::DEFAULT_FORMAT.to_owned(),
                timezone: TimeZone::system(),
                when: When::Given(
                    date(2025, 10, 04)
                        .at(0, 0, 0, 0)
                        .to_zoned(TimeZone::system())
                        .unwrap()
                ),
                debug: false,
                jalali: false
            }
        );
    }

    #[test]
    fn test_cli_gregorian_to_jalali() {
        assert_eq!(
            call(&["-j", "1004000025"]), // 2025/10/04
            Args {
                format: Args::DEFAULT_FORMAT.to_owned(),
                timezone: TimeZone::system(),
                when: When::Given(
                    date(2025, 10, 04)
                        .at(0, 0, 0, 0)
                        .to_zoned(TimeZone::system())
                        .unwrap()
                ),
                debug: false,
                jalali: true,
            }
        );
    }
}
