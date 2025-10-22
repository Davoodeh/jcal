use std::num::ParseIntError;

use clap::{
    Arg, ArgAction, ArgMatches, Command, CommandFactory, FromArgMatches, command, error::ErrorKind,
    value_parser,
};
use jcal::{
    clap_helper::{ArgMatchesExt, CommandFactoryExt, StaticMap},
    date::{CommonDate, Date},
    parser::{parse_jalali_month, parse_month, parse_weekday},
};
use jelal::{MonthDay, Weekday};
use jiff::{Timestamp, ToSpan};

use crate::layout::{Highlight, Layout, WeekNumConfig};

#[derive(Debug, Clone, PartialEq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub const PARSER_DEFAULT: &'static str = "auto";

    pub const PARSER_MAP: StaticMap<&'static Self> = StaticMap(&[
        (Self::PARSER_DEFAULT, &Self::Auto),
        ("always", &Self::Always),
        ("never", &Self::Never),
    ]);
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum Reform {
    Y1752,
    Gregorian,
    Julian,
}

impl Reform {
    // only allow for proleptic greogiran
    pub const PARSER_MAP: StaticMap<&'static Self> = StaticMap(&[
        // ("1752", &Self::Y1752),
        ("gregorian", &Self::Gregorian),
        ("iso", &Self::Gregorian),
        // ("julian", &Self::Julian),
    ]);
}

#[derive(Debug, PartialEq)]
pub struct Args {
    // since calendar is only proleptic, nothing is saved
    // pub reform: Reform,
    /// non-zero, how many months is in the given span.
    pub months: usize,
    pub span: bool,
    pub color: ColorMode,
    /// How many months should be printed
    pub columns: usize,
    /// If true, up to this number of columns will be set but may be less if cannot fit in terminal
    pub auto_columns: bool,
    /// The width of the terminal/space in characters
    pub width_chars: usize,
    /// What is the given time or system's time if not given.
    ///
    /// This is the basis for calculating the "start date" of the layout.
    pub now: Date,
    pub layout: Layout,
    pub full_year_mode: bool,
}

impl Args {
    /// Set now field and sync it with the layout.
    fn sync_layout(&mut self) {
        self.layout.base_row.column.content.grid.date = self.start_month();
        self.layout.base_row.more_columns = self.months.saturating_sub(1);
        self.layout.next_row_after_column = self.suggested_columns();

        // Default to having now highlighted, this differs with cal
        match self.layout.highlight {
            Some(Highlight::Day(_)) | None => {
                self.layout.highlight = Some(Highlight::Day(self.now.clone()))
            }
            Some(Highlight::Week(_)) => {}
        }

        let column = &mut self.layout.base_row.column;
        if column.vertical {
            column.content.weeknums_before_grid = false;
            self.layout.common_weekday = Some(true);
        }
    }

    /// What is the earliest month to be printed.
    ///
    /// This removes the need for "spanning" mechanism to complicate [`CalendarLayout`].
    // Now is the given value to the configuration without accounting for the span or month count.
    fn start_month(&self) -> Date {
        if self.full_year_mode {
            let mut date = self.now.clone();
            date.set_saturating_month(1);
            return date;
        }

        // normalize just in case, it doesn't matter but since nothing is tested, better do
        let mut now = self.now.clone();
        now.set_saturating_day(1);

        if !self.span || (self.months == 1) {
            return now;
        }
        // basically if in span mode, put the given time at the center of the span which naturally
        // sends the start month half of the span behind
        let mut months_before = (self.months - 1) / 2; // remove the initial month
        let months_before_rem = (self.months - 1) % 2;
        months_before += months_before_rem; // if not even, put the odd one behind the current

        let months_before: jelal::IDayDiff =
            months_before.try_into().unwrap_or(jelal::IDayDiff::MAX);

        now.set_saturating_months_offset(-months_before);
        now
    }

    /// How many months does should this calendar print.
    ///
    /// This keeps the "fitting" concern away from [`CalendarLayout`].
    fn suggested_columns(&self) -> usize {
        if self.auto_columns {
            self.layout
                .columns_in_width(self.width_chars)
                .min(self.columns)
                .max(1) // keep the minimum 1
        } else {
            self.columns
        }
    }
}

impl Args {
    pub const MONTHS_1_LONG: &str = "one";
    pub const MONTHS_3_LONG: &str = "three";
    pub const MONTHS_12_LONG: &str = "twelve";
    pub const MONTHS_LONG: &str = "months";
    pub const SPAN_LONG: &str = "span";
    pub const SUNDAY_LONG: &str = "sunday";
    pub const MONDAY_LONG: &str = "monday";
    pub const WEEKDAY_LONG: &str = "weekday";
    pub const ORDINAL_LONG: &str = "julian";
    pub const REFORM_LONG: &str = "reform";
    pub const ISO_LONG: &str = "iso";
    pub const YEAR_LONG: &str = "year";
    pub const WEEK_LONG: &str = "week";
    pub const VERTICAL_LONG: &str = "vertical";
    pub const COLUMNS_LONG: &str = "columns";
    pub const COLOR_LONG: &str = "color";
    pub const JALALI_LONG: &str = "jalali";
    pub const POSITIONAL_1_ID: &str = "opt1";
    pub const POSITIONAL_2_ID: &str = "opt2";
    pub const POSITIONAL_3_ID: &str = "opt3";

    pub const MONTHS_SETTERS_ARGS: &[&str] = &[
        Self::MONTHS_1_LONG,
        Self::MONTHS_3_LONG,
        Self::MONTHS_12_LONG,
        Self::MONTHS_LONG,
    ];
    pub const REFORM_SETTERS_ARGS: &[&str] = &[Self::REFORM_LONG, Self::ISO_LONG];
    pub const WEEKDAY_SETTERS_ARGS: &[&str] =
        &[Self::SUNDAY_LONG, Self::MONDAY_LONG, Self::WEEKDAY_LONG];

    pub fn args() -> [Arg; 20] {
        [
            Arg::new(Self::MONTHS_1_LONG)
                .long(Self::MONTHS_1_LONG)
                .short('1')
                .overrides_with_all(Self::MONTHS_SETTERS_ARGS)
                .help("print one month (default, equal to `--months 1`)")
                .action(ArgAction::SetTrue),
            Arg::new(Self::MONTHS_3_LONG)
                .long(Self::MONTHS_3_LONG)
                .short('3')
                .overrides_with_all(Self::MONTHS_SETTERS_ARGS)
                .help("print 3 month spanning (equal to `--months 3 --span`)")
                .action(ArgAction::SetTrue),
            Arg::new(Self::MONTHS_12_LONG)
                .long(Self::MONTHS_12_LONG)
                .short('Y')
                .overrides_with_all(Self::MONTHS_SETTERS_ARGS)
                .help("print 11 months after this one (equal to `--months 12`)")
                .action(ArgAction::SetTrue),
            Arg::new(Self::MONTHS_LONG)
                .long(Self::MONTHS_LONG)
                .short('n')
                .overrides_with_all(Self::MONTHS_SETTERS_ARGS)
                .help("print the number of months (starting with this one if not spanning)")
                .value_parser(value_parser!(usize)),
            Arg::new(Self::SPAN_LONG)
                .long(Self::SPAN_LONG)
                .short('S')
                .help("put the current month in the middle of multiple months")
                .action(ArgAction::SetTrue),
            Arg::new(Self::SUNDAY_LONG)
                .long(Self::SUNDAY_LONG)
                .short('s')
                .overrides_with_all(Self::WEEKDAY_SETTERS_ARGS)
                .help("set Sunday as the first weekday (default, equal to `--weekday Sunday`)")
                .action(ArgAction::SetTrue),
            Arg::new(Self::MONDAY_LONG)
                .long(Self::MONDAY_LONG)
                .short('m')
                .overrides_with_all(Self::WEEKDAY_SETTERS_ARGS)
                .help("set Monday as the first weekday (equal to `--weekday Monday`)")
                .action(ArgAction::SetTrue),
            Arg::new(Self::WEEKDAY_LONG)
                .long(Self::WEEKDAY_LONG)
                .overrides_with_all(Self::WEEKDAY_SETTERS_ARGS)
                .value_parser(parse_weekday)
                .help("set the given as the first weekday (`sunday = 0`)"),
            Arg::new(Self::ORDINAL_LONG)
                .long(Self::ORDINAL_LONG)
                .short('j')
                .overrides_with(Self::ORDINAL_LONG)
                .help("use ordinals instead of day of month")
                .action(ArgAction::SetTrue),
            Arg::new(Self::REFORM_LONG)
                .long(Self::REFORM_LONG)
                .overrides_with_all(Self::REFORM_SETTERS_ARGS)
                .value_parser(Reform::PARSER_MAP)
                .ignore_case(true)
                .help("reform Gregorian calendar (for now, no option but proleptic is supported)"),
            Arg::new(Self::ISO_LONG)
                .long(Self::ISO_LONG)
                .overrides_with_all(Self::REFORM_SETTERS_ARGS)
                .help("reform Gregorian calendar in ISO (equal to `--reform iso`)")
                .action(ArgAction::SetTrue),
            Arg::new(Self::YEAR_LONG)
                .long(Self::YEAR_LONG)
                .short('y')
                .help("print the full year calendar")
                .overrides_with(Self::YEAR_LONG)
                .conflicts_with_all(Self::MONTHS_SETTERS_ARGS)
                .action(ArgAction::SetTrue),
            Arg::new(Self::WEEK_LONG)
                .long(Self::WEEK_LONG)
                .short('w')
                .num_args(0..=1) // if not given don't push the default
                .overrides_with(Self::WEEK_LONG)
                .default_missing_value("")
                .value_parser(|s: &str| -> Result<Option<usize>, String> {
                    if s.is_empty() {
                        return Ok(None);
                    }
                    let v: usize = s.parse().map_err(|e: ParseIntError| e.to_string())?;
                    if (1..=54).contains(&v) {
                        Ok(Some(v - 1))
                    } else {
                        Err("a week number must be between 1..=54".to_string())
                    }
                })
                .help("print the week numbers in US or ISO format"),
            Arg::new(Self::VERTICAL_LONG)
                .long(Self::VERTICAL_LONG)
                .short('v')
                .overrides_with(Self::VERTICAL_LONG)
                .help("print a week as a vertical line instead")
                .action(ArgAction::SetTrue),
            Arg::new(Self::COLUMNS_LONG)
                .long(Self::COLUMNS_LONG)
                .short('c')
                .overrides_with(Self::COLUMNS_LONG)
                .num_args(0..=1) // if not given don't push the default
                .value_parser(|s: &str| -> Result<Option<usize>, String> {
                    if s == "auto" {
                        return Ok(None);
                    }
                    let v: usize = s.parse().map_err(|e: ParseIntError| e.to_string())?;
                    Ok(Some(v.max(1)))
                })
                .help("how many months to fit in one row (`auto` for the length of output)"),
            Arg::new(Self::COLOR_LONG)
                .long(Self::COLOR_LONG)
                .overrides_with(Self::COLOR_LONG)
                .num_args(0..=1) // if not given don't push the default
                .default_missing_value(ColorMode::PARSER_DEFAULT)
                .value_parser(ColorMode::PARSER_MAP)
                .ignore_case(true)
                .help("set coloring behavior"),
            Arg::new(Self::JALALI_LONG)
                .long(Self::JALALI_LONG)
                .short('J')
                .help("print the calendar in Jalali and default the starting weekday to Saturday")
                .action(ArgAction::SetTrue),
            Arg::new(Self::POSITIONAL_1_ID)
                .value_name("[[[DAY] MONTH] YEAR]|MONTH|@TIMESTAMP")
                .help("optionally give a `@timestamp`, month name or date in `dmy` order"),
            Arg::new(Self::POSITIONAL_2_ID).hide(true),
            Arg::new(Self::POSITIONAL_3_ID).hide(true),
        ]
    }
}

impl CommandFactory for Args {
    fn command() -> Command {
        command!(/* with version, about and author */)
            // TODO add a -c/--calendar that passes to jiff-icu
            .args(Self::args())
    }

    fn command_for_update() -> Command {
        Self::command()
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            months: 1.try_into().unwrap(),
            span: false,
            color: ColorMode::Auto,
            columns: 3,
            auto_columns: true,
            now: Date::Gregorian(jiff::Zoned::now().date()),
            width_chars: terminal_size::terminal_size()
                .map(|(w, _)| w.0)
                .unwrap_or(80) as usize,
            // Doesn't matter what it is as of now.
            layout: Default::default(),
            full_year_mode: false,
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
        // flags
        if matches.get_flag(Self::SPAN_LONG) {
            self.span = true;
        }
        if matches.get_flag(Self::ORDINAL_LONG) {
            self.layout.base_row.column.content.grid.ordinal_mode = true;
        }
        if matches.get_flag(Self::VERTICAL_LONG) {
            self.layout.base_row.column.vertical = true;
        }

        if matches.get_flag(Self::JALALI_LONG) {
            self.now = match self.now.clone() {
                Date::Gregorian(date) => Date::Jalali(date.into()),
                v @ Date::Jalali(_) => v,
            };
            self.layout.base_row.column.content.grid.base_weekday = Weekday::SAT;
        }

        // MONTHS_SETTERS_ARGS
        if matches.get_flag(Self::MONTHS_1_LONG) {
            self.months = 1;
        } else if matches.get_flag(Self::MONTHS_3_LONG) {
            self.months = 3;
            self.span = true;
        } else if matches.get_flag(Self::MONTHS_12_LONG) {
            self.months = 12;
        } else if let Some(&months) = matches.get_one::<usize>(Self::MONTHS_LONG) {
            self.months = months.max(1);
        }

        // REFORM_SETTERS_ARGS
        // if matches.get_flag(Self::ISO_LONG) {
        //     self.reform = Reform::Iso;
        // } else if let Some(&reform) = matches.get_one::<&'static Reform>(Self::REFORM_LONG) {
        //     self.reform = reform.clone();
        // }

        if let Some(columns) = matches.get_one::<Option<usize>>(Self::COLUMNS_LONG) {
            (self.columns, self.auto_columns) = match columns {
                Some(v) => (*v, false),
                None => (usize::MAX, true),
            };
        }

        if let Some(&color) = matches.get_one::<&ColorMode>(Self::COLOR_LONG) {
            self.color = color.clone();
        }

        // POSITIONAL
        if let Some(pos1) = matches.get_one::<String>(Self::POSITIONAL_1_ID) {
            if pos1.starts_with("@") {
                if matches.is_explicit(Self::POSITIONAL_2_ID)
                    || matches.is_explicit(Self::POSITIONAL_3_ID)
                {
                    return Err(Self::error(
                        ErrorKind::ArgumentConflict,
                        "given a @TIMESTAMP, no other parameters for setting the date can be used",
                    ));
                }

                // parse
                match pos1[1..]
                    .parse()
                    .map_err(|e: ParseIntError| e.to_string())
                    .and_then(|i: i64| {
                        let tz = jiff::tz::TimeZone::system();
                        let v = Timestamp::new(i, 0).map_err(|e| e.to_string())?;
                        let offset = tz.to_offset(v).seconds();
                        match v.checked_add(ToSpan::seconds(offset)) {
                            Ok(v) => Ok(v.to_zoned(tz)),
                            Err(e) => Err(e.to_string()),
                        }
                    }) {
                    Ok(v) => {
                        self.now = match self.now {
                            Date::Jalali(_) => Date::Jalali(v.into()),
                            Date::Gregorian(_) => Date::Gregorian(v.date()),
                        };
                        // will get synced later
                        self.layout.highlight = Some(Highlight::Day(Default::default()));
                    }
                    Err(e) => {
                        return Err(Self::error(
                            ErrorKind::InvalidValue,
                            format!("timestamp is invalid ({})", e),
                        ));
                    }
                }
            } else if let Ok(pos1) = i16::from_str_radix(pos1, 10) {
                (|| {
                    let Some(pos2) = matches.get_one::<String>(Self::POSITIONAL_2_ID) else {
                        // pos1 could be the day so we set it here not earlier not to modify
                        // it twice and/or saturate/wrap to make invalid values
                        self.now.set_saturating_year(pos1 as i32);

                        // since year is set, also set the year flag
                        self.layout.base_row.column.year_in_header = true;

                        return Ok(()); // [YEAR]
                    };

                    let month = match self.now {
                        Date::Jalali(_) => parse_jalali_month(pos2),
                        Date::Gregorian(_) => parse_month(pos2),
                    }
                    .map_err(|e| Self::error(ErrorKind::InvalidValue, e))?;
                    self.now.set_saturating_month(month);

                    let Some(pos3) = matches.get_one::<String>(Self::POSITIONAL_3_ID) else {
                        return Ok(()); // [[MONTH] YEAR]
                    };

                    let Ok(year) = pos3.parse() else {
                        return Err(Self::error(ErrorKind::InvalidValue, "year is invalid"));
                    };

                    let day = pos1.clamp(1, MonthDay::MAX_DAY as i16) as u8; // not to wrap
                    self.now.set_saturating_year(year);
                    self.now.set_saturating_day(day);
                    Ok(()) // [[[DAY] MONTH] YEAR]
                })()?;
            } else {
                if matches.is_explicit(Self::POSITIONAL_2_ID)
                    || matches.is_explicit(Self::POSITIONAL_3_ID)
                {
                    return Err(Self::error(
                        ErrorKind::ArgumentConflict,
                        "given a month only, no other parameters for setting the date can be used",
                    ));
                }

                let month = match &self.now {
                    Date::Jalali(_) => parse_jalali_month(pos1),
                    Date::Gregorian(_) => parse_month(pos1),
                }
                .map_err(|_| {
                    Self::error(
                        ErrorKind::InvalidValue,
                        "either give a @TIMESTAMP, a MONTH or [[DAY] MONTH] YEAR",
                    )
                })?;
                self.now.set_saturating_month(month);
            }
        }

        // is this java?
        let base_weekday = &mut self.layout.base_row.column.content.grid.base_weekday;
        // WEEKDAY_SETTERS_ARGS (must come after Jalali since that default to Sat)
        if matches.get_flag(Self::SUNDAY_LONG) {
            *base_weekday = Weekday::SUN;
        } else if matches.get_flag(Self::MONDAY_LONG) {
            *base_weekday = Weekday::MON;
        } else if let Some(weekday) = matches.get_one::<Weekday>(Self::WEEKDAY_LONG) {
            *base_weekday = weekday.clone();
        }
        // after WEEKDAY_SETTERS_ARGS and after now since this has precedence over other NOW options
        if let Some(when_week) = matches.get_one::<Option<usize>>(Self::WEEK_LONG) {
            if let Some(week) = when_week {
                self.now.set_saturating_weeknum(*week, base_weekday.clone());
                self.layout.highlight = Some(Highlight::Week(*week + 1));
            }
            // Without reform there is no way now to set ISO as the weeknumconfig
            self.layout
                .base_row
                .column
                .content
                .weeknums
                .get_or_insert(WeekNumConfig::Based);
        }

        if matches.get_flag(Self::YEAR_LONG) {
            self.layout.base_row.column.year_in_header = false;
            self.months = 12;
            self.full_year_mode = true;
        }

        self.sync_layout();

        Ok(())
    }
}
