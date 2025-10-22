//! Structures a calendar for printing.
//!
//! For more about `width` functions see [`Self::width`].
//!
//! # Layout Reference
//!
//! Layout:
//! ```text
//! .----------------.
//! | CONTENT_HEADER | } 0..=1 Line
//! |----------------|
//! |     CONTENT    | } ROW Lines * ROW Count
//! `----------------*
//! ```
//!
//! `CONTENT_HEADER`: Any arbitrary text (year in year_header mode, centered, else omitted)
//!
//! Layout:
//! ```text
//! .------------------<CONTENT>-----------------.
//! |         NEVER         | COMMON_ROWS_PREFIX | } 0..=1 Line
//! |-----------------------+--------------------|
//! | COMMON_COLUMNS_PREFIX |        ROW 1       |
//! |-----------------------+--------------------|
//! | COMMON_COLUMNS_PREFIX |        ROW 2       |
//! |-----------------------+--------------------|
//! | COMMON_COLUMNS_PREFIX |         ...        |
//! |-----------------------+--------------------|
//! | COMMON_COLUMNS_PREFIX |        ROW r       |
//! `--------------------------------------------*
//!  `----------v----------*
//!         0..=1 CELL
//! ```
//!
//! A `COMMON_ROWS_PREFIX` layout:
//! ```text
//! .-----------------<COMMON_ROWS_PREFIX n>------------------.
//! | COMMON_COLUMNS_HEADER 1 | ... | COMMON_COLUMNS_HEADER n | } 0..=1 Line
//! `---------------------------------------------------------*
//!  `-----------v-----------*
//!         COLUMN Width
//! ```
//!
//! A `ROW` layout:
//! ```text
//! .-------------------------<ROW n>------------------------.
//! |         COLUMN 1        | ... |        COLUMN n        | } COLUMN Lines
//! `--------------------------------------------------------*
//!  `-----------v-----------*
//!         COLUMN Width
//! ```
//!
//! `COMMON_COLUMNS_PREFIX`: Is used only in vertical mode having a vertical list of weekdays each
//! one CELL unless disabled explicitly. Either this is enabled or `COMMON_COLUMNS_HEADER`.
//!
//! `COMMON_COLUMNS_HEADER`: Is the weekday indicator if explictly requested.
//!
//! A `COLUMN` layout:
//! ```text
//! .---<COLUMN n>---.
//! | COLUMN_HEADER  | } 1 Line
//! |----------------|
//! | COLUMN_CONTENT |
//! `----------------*
//! ```
//!
//! `COLUMN_HEADER`: Any arbitrary text (month + year name or just month if in year_header)
//!
//! `COLUMN_CONTENT`: Basically holds the month information.
//! ```text
//! .-------------<COLUMN_CONTENT n>--------------.
//! |                       | GRID_HEADER_CONTENT | } 0..=1 Line  .
//! |                       |---------------------|               |
//! | COLUMN_CONTENT_PREFIX |         GRID        | } 6..=7 Lines  > 6..=8 Lines
//! |                       |---------------------|               |
//! |                       | GRID_FOOTER_CONTENT | } 0..=1 Line  *
//! `---------------------------------------------*
//!  `----------v----------* `----------v--------*
//!         0..=1 CELL             6..=7 CELLs
//!   `---------------------v-------------------*
//!                   6..=8 CELLs
//! ```
//!
//! `GRID_HEADER`: Is either empty in vertical mode (6 CELLs configuration), or holds 7 CELLs with
//! the weekday abbreviation indicator (either this is enabled or `GRID_FOOTER`). Alternatively,
//! if explictly requested to have common weekdays will be omitted in 7CELLs.
//!
//! `GRID_FOOTER`: Is either empty (7 CELLs configuration) or in vertical mode holds 6 CELLs with
//! week numbers if requested (either this is enabled or `GRID_HEADER`).
//!
//! `COLUMN_CONTENT_PREFIX`: Only when not vertical (7 CELLs configuration) and with week numbers
//! requested holds the week number unless requested explictly to have week days per column in
//! vertical mode.
//!
//! `GRID`: Basically is the month calendar. Either 6x7 CELLs in vertical mode (6 CELLs
//! configuration) or 7x6 otherwise (7 CELLs configuration).
//!
//! `CELL`: Is either 2 characters in length or 3 if ordinals (Julian) is requested.

#![allow(dead_code)]

use core::array;

use jcal::{
    WEEKDAYS,
    date::{CommonDate, Date},
};
use jelal::{IYear, UOrdinal, Weekday};

use crate::string::{Aligner, ansi_width, highlight};

/// How many weeks is in each grid.
pub const WEEK_COUNT: usize = 6;

/// How many days is in each week.
pub const WEEK_DAYS: usize = 7;

pub const DEFAULT_DELIMITER: &str = " ";

/// Join a string with the given delimiter.
fn join<S: AsRef<str>>(mut v: impl Iterator<Item = S>, delimiter: &str) -> String {
    let Some(first) = v.next() else {
        return Default::default();
    };
    v.fold(first.as_ref().to_string(), |acc, i| {
        acc + delimiter + i.as_ref()
    })
}

/// Week numbers in compatible cells with this grid. (const len of 6)
pub fn weeknums(config: &WeekNumConfig, date: &Date, base_weekday: Weekday) -> [usize; WEEK_COUNT] {
    let date = {
        // ensure the day is the first day of the month for weeknum calculation
        let mut new = date.clone();
        new.set_saturating_day(1);
        new
    };

    array::from_fn(|i| {
        match config {
            // 0 is the 53 of the previous year
            WeekNumConfig::Iso => date.iso_weeknum() as usize + i,
            WeekNumConfig::Based => date.weeknum(base_weekday) as usize + i,
        }
    })
}

/// Count 6 weeks from the 1st of the given month, format and optionally highlight.
///
/// Since this only count a year's weeks at max, it's output should never exceed 2 in width.
pub fn format_weeknums(
    date: &Date,
    base_weekday: Weekday,
    config: &WeekNumConfig,
    highlight_week: Option<usize>,
) -> [String; WEEK_COUNT] {
    weeknums(config, date, base_weekday).map(|mut weeknum| {
        if weeknum == 0 {
            // set the max weeknum
            let mut date = date.clone();
            date.set_saturating_year(date.year().saturating_sub(1));
            date.set_saturating_ordinal(UOrdinal::MAX);
            weeknum = date.weeknum(base_weekday) as usize;
        }
        let v = Aligner::SPACE.right(&weeknum.to_string(), 2);
        if Some(weeknum) == highlight_week {
            highlight(&v)
        } else {
            v
        }
    })
}

/// Collect a column weekdays from the base to the end.
pub fn weekdays(base_weekday: Weekday) -> [&'static str; WEEK_DAYS] {
    array::from_fn(|offset| WEEKDAYS[base_weekday.forward(offset).get() as usize])
}

/// How week counting should work.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeekNumConfig {
    /// ISO 8601 system of counting (Monday based, the first Thursday in the new year is Week 1).
    Iso,
    /// Given the base, the week that holds this day is Week 1.
    Based,
}

/// What to highlight.
#[derive(Debug, Clone, PartialEq)]
pub enum Highlight {
    Week(usize),
    Day(Date),
}

impl Highlight {
    pub fn day(&self) -> Option<&Date> {
        match self {
            Self::Day(v) => Some(v),
            Self::Week(_) => None,
        }
    }

    pub fn week(&self) -> Option<usize> {
        match self {
            Self::Week(v) => Some(*v),
            Self::Day(_) => None,
        }
    }
}

/// Create a grid of 7x6 of weeks of a month and weekdays.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// Take the year and month to print.
    pub date: Date,
    /// If true, prints day of year instead of day of month.
    pub ordinal_mode: bool,
    /// The start of the week.
    pub base_weekday: Weekday,
}

impl Grid {
    /// Put a value in a cell size of this grid.
    pub fn format_in_day_cell(&self, s: &str) -> String {
        Aligner::SPACE.right(&s, self.day_cell_width())
    }
    /// How many characters make a single cell for writing a day of month.
    pub fn day_cell_width(&self) -> usize {
        if self.ordinal_mode { 3 } else { 2 }
    }

    /// Format a 7x6 grid of weeks with corresponding weekdays as string, optionally a day brighter.
    pub fn format(&self, highlight_day: Option<&Date>) -> [[String; WEEK_DAYS]; WEEK_COUNT] {
        let date = &self.date;

        let is_highlight = |day: UOrdinal| {
            highlight_day
                .map(|hday| {
                    // not the most performant but the most pretty
                    let mut date = date.clone();
                    if self.ordinal_mode {
                        date.set_saturating_ordinal(day);
                    } else {
                        date.set_saturating_day(day as u8);
                    }
                    *hday == date
                })
                .unwrap_or(false)
        };

        let raw = self.new_grid();
        array::from_fn(|i| {
            array::from_fn(|j| {
                let value = raw[i][j];
                if value == 0 {
                    self.format_in_day_cell("")
                } else {
                    let s = self.format_in_day_cell(&value.to_string());
                    if is_highlight(value) {
                        highlight(&s)
                    } else {
                        s
                    }
                }
            })
        })
    }

    /// Create a grid in 7 days times 6 weeks formation.
    // maximum of 6 weeks of 7 days.
    //
    // last_day must be strictly smaller than 38 and offset + last_day should not overflow.
    //
    // Technically there can only be 7 configuration of months (which day of week is the first day
    // of the month) times the number of possible month last days (28..=31 for Gregorian and
    // 29..=31 for Jalali).
    pub fn new_grid(&self) -> [[UOrdinal; WEEK_DAYS]; WEEK_COUNT] {
        let mut cells = [[0; _]; _];

        let start_month = {
            let mut v = self.date.clone();
            v.set_saturating_day(1);
            v
        };
        let month_end = self.date.month_end_day();

        let offset = if self.ordinal_mode {
            start_month.ordinal() - 1
        } else {
            0
        };

        // How many empty days are in the grid before the first day of the month in the given base.
        //
        // This is guaranteed to be at maximum 6 days (`week_len - 1`). [`usize`] is returned since
        // this is to be used in indexing.
        //
        // For example if the calendar is Sunday based and the first day of the month is Saturday
        // the grid will look like this:
        // ```text
        // Su Mo Tu We Th Fr Sa
        // 00 00 00 00 00 00 01
        // ```
        //
        // That is 6.
        let first_i: usize = self.base_weekday.till_next(&start_month.weekday()) as usize;

        let mut row: usize = 0;
        let mut i = first_i;
        for v in 1..=month_end {
            cells[row][i] = v as UOrdinal + offset;

            // max - 1
            if i == 6 {
                i = 0;
                row += 1;
            } else {
                i += 1;
            }
        }

        cells
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            date: Date::default(),
            ordinal_mode: false,
            base_weekday: Weekday::SUN,
        }
    }
}

/// Holds a grid in string format.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnContent {
    /// If given prints the week number.
    pub weeknums: Option<WeekNumConfig>,
    /// If true, first (from right to left or top to bottom) week numbers appears.
    pub weeknums_before_grid: bool,
    /// If true, prints the week days.
    pub weekdays: bool,
    /// If true, first (from right to left or top to bottom) week days appears.
    pub weekdays_before_grid: bool,
    pub grid: Grid,
}

impl ColumnContent {
    const WEEKNUM_EMPTY: &str = "  ";

    /// Return the weekdays helper even if weeknums is off.
    ///
    /// This has extra empty fields to adjust its width hence not statically 7 days.
    pub fn format_weekdays_force(&self) -> Vec<String> {
        let mut v = weekdays(self.grid.base_weekday)
            .map(|s| self.grid.format_in_day_cell(s))
            .to_vec();
        if self.weeknums.is_some() {
            // create an empty cell to shift for the added row
            if self.weeknums_before_grid {
                v.insert(0, Self::WEEKNUM_EMPTY.to_owned());
            } else {
                v.push(Self::WEEKNUM_EMPTY.to_owned());
            }
        }
        v
    }

    /// How many rows and columns will this formatted value have.
    pub fn row_cols(&self) -> (usize, usize) {
        let rows = WEEK_COUNT + if self.weekdays { 1 } else { 0 };
        let cols = WEEK_DAYS + if self.weeknums.is_some() { 1 } else { 0 };
        (rows, cols)
    }

    /// If printed back to back, what will be the width of each row.
    pub fn row_str_width(&self) -> usize {
        WEEK_DAYS * self.grid.day_cell_width()
            + if self.weeknums.is_some() {
                ansi_width(Self::WEEKNUM_EMPTY)
            } else {
                0
            }
    }

    /// This guarantees that every inner vec has the same length.
    pub fn format(&self, highlight_section: Option<&Highlight>) -> Vec<Vec<String>> {
        let mut grid = self
            .grid
            .format(highlight_section.as_ref().and_then(|i| i.day()))
            .into_iter()
            .map(|i| i.to_vec())
            .collect::<Vec<_>>();

        // regardless of the content, this always inserts a row then adds a column.
        // flags just change the content of the rows and columns.

        let cols = self.weeknums.as_ref().map(|c| {
            format_weeknums(
                &self.grid.date,
                self.grid.base_weekday,
                c,
                highlight_section.and_then(|i| i.week()),
            )
        });

        if let Some(cols) = cols {
            for (i, v) in cols.into_iter().enumerate() {
                let col = if grid[i].iter().all(|c| c.trim_start().is_empty()) {
                    Self::WEEKNUM_EMPTY.to_owned()
                } else {
                    v
                };
                if self.weeknums_before_grid {
                    grid[i].insert(0, col);
                } else {
                    grid[i].push(col);
                }
            }
        }

        if self.weekdays {
            let row = self.format_weekdays_force();
            if self.weekdays_before_grid {
                grid.insert(0, row)
            } else {
                grid.push(row)
            }
        }

        grid
    }
}

impl Default for ColumnContent {
    fn default() -> Self {
        Self {
            weeknums: None,
            weeknums_before_grid: true, // no difference
            weekdays: true,
            weekdays_before_grid: true,
            grid: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub content: ColumnContent,
    /// What separates each cell.
    pub delimiter: String,
    /// If true, year will be explicitly written in the column header.
    pub year_in_header: bool,
    /// If false, each week is a row, else each week is a column (transposed).
    pub vertical: bool,
}

impl Column {
    /// How this column formats a year.
    ///
    /// Exposed for consistency if another formatter wants to print.
    pub fn year_format(year: IYear) -> String {
        // basically format!("{:0<4}") but written this way for the sake of consistency
        let s = year.to_string();
        if ansi_width(&s) < 4 {
            Aligner::ZERO.right(&year.to_string(), 4)
        } else {
            s
        }
    }

    fn format_header(&self) -> String {
        // TODO FIXME add tests to make sure this does not produce trimmed values if the produced
        //            string is smaller than the given width.
        let date = &self.content.grid.date;
        let month_name = date.month_name();
        let width = self.width();
        if self.year_in_header {
            Aligner::SPACE.center(
                &(month_name.to_owned() + " " + &Self::year_format(date.year())),
                width,
            )
        } else {
            Aligner::SPACE.center(month_name, width)
        }
    }

    /// Join the given cells with proper delimiter.
    pub fn join_cells<S: AsRef<str>>(&self, v: impl Iterator<Item = S>) -> String {
        join(v, &self.delimiter)
    }

    /// What will be the width of this column.
    pub fn width(&self) -> usize {
        let dw = ansi_width(&self.delimiter);
        if self.vertical {
            let c = self.content.row_cols().0;
            // since resize is done using the cell size, we just count that
            c * self.content.grid.day_cell_width() + (c - 1) * dw
        } else {
            let c = self.content.row_cols().1;
            self.content.row_str_width() + (c - 1) * dw
        }
    }

    /// Return a vec row for each line.
    pub fn format(&self, highlight_section: Option<&Highlight>) -> Vec<String> {
        // merge all the content into rows.
        let content = self.content.format(highlight_section);
        let (rows, cols) = if self.vertical {
            let v = self.content.row_cols();
            (v.1, v.0)
        } else {
            self.content.row_cols()
        };
        let mut lines = Vec::with_capacity(rows + 1);
        lines.push(self.format_header());
        for i in 0..rows {
            let line = self.join_cells((0..cols).map(|j| {
                if self.vertical {
                    // adjust weekdays for column size since they may not be.
                    self.content.grid.format_in_day_cell(&content[j][i])
                } else {
                    content[i][j].clone()
                }
            }));
            lines.push(line);
        }

        lines
    }
}

impl Default for Column {
    fn default() -> Self {
        Self {
            content: Default::default(),
            delimiter: DEFAULT_DELIMITER.to_owned(),
            year_in_header: false,
            vertical: false,
        }
    }
}

/// Holds multiple columns from a starting date to the end.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Months to print after the start month.
    pub more_columns: usize,
    /// How each column must be separated from the next.
    pub delimiter: String,
    /// Governs the start date and column formatting.
    pub column: Column,
}

impl Row {
    /// Join the given cells with proper delimiter.
    pub fn join_columns<S: AsRef<str>>(&self, v: impl Iterator<Item = S>) -> String {
        join(v, &self.delimiter)
    }

    /// What will be the width of this column.
    pub fn width(&self) -> usize {
        let dw = ansi_width(&self.delimiter);
        let cw = self.column.width();
        (cw * (self.more_columns + 1)) + (dw * self.more_columns)
    }

    /// Given a width, determine how many columns fit is the maximum that fits.
    pub fn columns_in_width(&self, maximum_width: usize) -> usize {
        let column_width = self.column.width();
        // first one doesn't use a delimiter so just subtract
        let Some(maximum_width) = maximum_width.checked_sub(column_width) else {
            return 0;
        };

        // after the first one, rest of columns have a delimiter
        let column_width = column_width + ansi_width(&self.delimiter);
        1 + (maximum_width / column_width)
    }

    /// Return a vec row for each line. This moves the column forward.
    pub fn format_mut(&mut self, highlight_section: Option<&Highlight>) -> Vec<String> {
        let mut lines = self.column.format(highlight_section);
        self.column
            .content
            .grid
            .date
            .set_saturating_months_offset(1);

        while self.more_columns != 0 {
            self.more_columns -= 1;

            let mut new = self.column.format(highlight_section).into_iter();
            self.column
                .content
                .grid
                .date
                .set_saturating_months_offset(1);

            for line in lines.iter_mut() {
                line.push_str(&self.delimiter);
                line.push_str(&new.next().unwrap());
            }
        }

        lines
    }
}

impl Default for Row {
    fn default() -> Self {
        Self {
            more_columns: 0,
            delimiter: DEFAULT_DELIMITER.repeat(3),
            column: Default::default(),
        }
    }
}

/// Manages a whole calendar to print and format.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Holds the starting row.
    pub base_row: Row,
    /// After this many months go to the next row (0 and 1 behave the same).
    pub next_row_after_column: usize,
    /// Use common week counters for all a row or column. If none, verticality determines it.
    ///
    /// See [`Column::vertical`].
    pub common_weekday: Option<bool>,
    /// What day to highlight.
    pub highlight: Option<Highlight>,
}

/// Width of the layout elements.
impl Layout {
    pub fn common_weekdays_is_enabled(&self) -> bool {
        self.common_weekday.unwrap_or(self.base_row.column.vertical)
    }

    /// `COMMON_COLUMNS_PREFIX` width.
    pub fn common_weekdays_cell_width(&self) -> usize {
        self.base_row.column.content.grid.day_cell_width()
    }

    pub fn common_weekdays_delimiter(&self) -> &str {
        self.base_row.column.delimiter.as_str()
    }

    /// How much the rows should come forward for possible prefixes.
    pub fn rows_left_offset(&self) -> usize {
        if self.base_row.column.vertical && self.common_weekdays_is_enabled() {
            self.common_weekdays_cell_width() + ansi_width(self.common_weekdays_delimiter())
        } else {
            0
        }
    }

    /// Uniform format for years.
    pub fn year_format(&self, year: IYear) -> String {
        Column::year_format(year)
    }

    // TODO
    // /// Returns each line as a string.
    // pub fn format(mut self) -> impl Iterator<Item = String> {}

    /// Print this value directly to std.
    pub fn print(mut self) {
        let mut prefixes = None;
        if self.common_weekdays_is_enabled() {
            self.base_row.column.content.weekdays = false;
            let weekdays = std::iter::once("".to_owned())
                .chain(
                    self.base_row
                        .column
                        .content
                        .format_weekdays_force()
                        .into_iter(),
                )
                .map(|i| {
                    self.base_row.column.content.grid.format_in_day_cell(&i)
                        + &self.base_row.column.delimiter
                })
                .collect::<Vec<_>>();
            if self.base_row.column.vertical {
                // since a header is in place, skip this
                prefixes = Some(weekdays.into_iter().cycle());
            } else {
                println!("{}", self.base_row.column.join_cells(weekdays.into_iter()));
            }
        }

        let months_requested = self.base_row.more_columns + 1;

        // if cross year boundaries, add the year number.
        {
            let mut date = self.base_row.column.content.grid.date.clone();
            let initial = date.year();
            date.set_saturating_months_offset(months_requested.min(i32::MAX as usize) as i32);
            if initial != date.year() {
                self.base_row.column.year_in_header = true;
            }
        }

        // if columns don't fit in a row, update
        let more_columns_new_value = |printed: usize| {
            (months_requested - printed)
                .min(self.next_row_after_column)
                .saturating_sub(1)
        };

        let mut printed_months = 0;
        self.base_row.more_columns = more_columns_new_value(printed_months);
        while printed_months < months_requested {
            printed_months += self.base_row.more_columns + 1;
            for line in self.base_row.format_mut(self.highlight.as_ref()) {
                if let Some(prefix) = &mut prefixes {
                    print!("{}", prefix.next().unwrap());
                }
                println!("{}", line);
            }
            // recharge row for more rows
            self.base_row.more_columns = more_columns_new_value(printed_months);
        }
    }

    // /// Width of the columns in this row.
    // pub fn columns_width(&self) -> usize {
    //     let columns = self.row_columns();
    //     columns * self.column_width() + ((columns - 1) * self.column_delimiter_width())
    // }

    // /// Width of the first to the last in this row.
    // pub fn row_width(&self) -> usize {
    //     self.common_columns_prefix_delimited_width() + self.column_width()
    // }
}

/// Other counting methods
impl Layout {
    // /// How many rows are in this layout (print this many and layout is exhausted.).
    // pub fn rows_count(&self) -> usize {
    //     ((self.forward_months + 1) / self.max_columns) + 1
    // }

    // /// How many columns to fit in this row.
    // ///
    // /// Guarantees to return at least one.
    // pub fn row_columns(&self) -> usize {
    //     (self.forward_months + 1) % self.max_columns
    // }

    /// Given the width, a row with how many columns does it fit.
    pub fn columns_in_width(&self, width: usize) -> usize {
        let Some(width) = width.checked_sub(self.rows_left_offset()) else {
            return 0;
        };
        self.base_row.columns_in_width(width)
    }

    // /// Calculate the "width" of a string.
    // ///
    // /// "width" is the number of characters for a string (this is ambiguous for now, but generally
    // /// should correspond to a single mono character on a terminal).
    // pub fn width(&self, s: &str) -> usize {
    //     s.chars().count()
    // }
}

/// Format related implementatoin.
impl Layout {
    // /// Format this row and continue to the next.
    // ///
    // /// Consecutive calls creates multiple rows until the end. If end is reached, returns false.
    // pub fn print(&self) {
    //     let mut l = self.clone();

    //     println!("{}", l.format_content_header());

    //     // print one row at a time.
    //     for row_i in 0..l.rows_count() {
    //         let grids = (0..=self.row_columns()).map(|_| {
    //             let grid = l.new_grid(l.start);
    //             l.forward_months -= 1; // won't trigger because row_columns
    //             l.start.add_month_saturating(1)
    //         });
    //     }
    //     let (rows, cols) = self.grid_cells();
    //     for i in 0..rows {
    //         for j in 0..cols {
    //             self.index_grid_cell(grid, row, col)
    //         }
    //     }
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn content_header_format(&self) -> Option<String> {
    //     if self.year_header {
    //         Some(
    //             LineFormatter::new(self.row_width(), &self.cell_delimiter())
    //                 .center(&self.year_format(self.start.year())),
    //         )
    //     } else {
    //         None
    //     }
    // }

    // /// Format start month as a column header.
    // ///
    // /// Assuming delimiter is at least 1 in length, this should always fit in one line.
    // ///
    // /// See struct documentation ([`Self`]).
    // pub fn column_header_format(&self) -> String {
    //     // if full year, no need to clutter with duplicate year values
    //     let delimiter = self.cell_delimiter();
    //     let formatter = LineFormatter::new(self.column_width(), &delimiter);
    //     let header = self.month_name();
    //     if self.year_header {
    //         formatter.center(header)
    //     } else {
    //         formatter.center(&format!(
    //             "{} {}",
    //             header,
    //             self.year_format(self.start.year())
    //         ))
    //     }
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn format_column_content(&self) -> Vec<String> {
    //     let mut lines = Vec::new();
    //     let grid_left_side_width = self.grid_left_side_width();
    //     let grid_left_side_filler = if grid_left_side_width != 0 {
    //         " ".repeat(grid_left_side_width) + &self.cell_delimiter()
    //     } else {
    //         Default::default()
    //     };

    //     lines.push(grid_left_side_filler + &self.format_grid_header());
    //     self.format_grid_left_side()
    //         .into_iter()
    //         .zip(self.format_grid())
    //         .for_each(|i| lines.push(i.0 + &i.1));
    //     lines.push(grid_left_side_filler + &self.format_grid_footer());
    //     lines
    // }

    // /// As documented and if empty, it's missing. See struct documentation ([`Self`]).
    // pub fn format_grid_left_side(&self) -> Vec<String> {
    //     if self.vertical {
    //         if self.common_weekday == Some(false) {
    //             Default::default()
    //         } else {
    //             self.format_weekday_names()
    //         }
    //     } else {
    //         if let Some(config) = &self.week_config {
    //             let start = match config {
    //                 WeekNumConfig::Iso => self.start.iso_weeknum(),
    //                 WeekNumConfig::Based(base) => self.start.weeknum(self.base_weekday),
    //             };
    //             // 6 weeks in rows
    //             (start..(start + 6)).map(|i| i.to_string()).collect()
    //         } else {
    //             Default::default()
    //         }
    //     }
    // }

    // /// Format the name of the weekdays in sequence fitting a cell.
    // pub fn format_weekday_names(&self) -> Vec<String> {
    //     let cell_width = self.cell_width();
    //     (0..7)
    //         .map(|offset| WEEKDAYS[self.base_weekday.forward(offset).get() as usize])
    //         .map(|weekday| LineFormatter::new(cell_width, " ").right(&weekday))
    //         .collect()
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn delimiter_width(&self) -> usize {
    //     self.width(&self.delimiter)
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn cell_width(&self) -> usize {
    //     if self.ordinal { 3 } else { 2 }
    // }

    // /// Separates cells from each other.
    // ///
    // /// See struct documentation ([`Self`]).
    // pub fn cell_delimiter(&self) -> String {
    //     // TODO replace with fields
    //     self.delimiter.repeat(1)
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn cell_delimiter_width(&self) -> usize {
    //     self.width(&self.cell_delimiter())
    // }

    // /// Separates columns from each other.
    // ///
    // /// See struct documentation ([`Self`]).
    // pub fn column_delimiter(&self) -> String {
    //     // TODO replace with fields
    //     self.delimiter.repeat(3)
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn column_delimiter_width(&self) -> usize {
    //     self.width(&self.column_delimiter())
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn grid_width(&self) -> usize {
    //     let (width_cells, _) = self.grid_cells();
    //     width_cells * self.cell_width()
    //         + (width_cells.saturating_sub(1)) * self.cell_delimiter_width()
    // }

    // /// See struct documentation ([`Self`]).
    // fn format_grid_header(&self) -> Option<String> {
    //     let cells = (0..7)
    //         .map(|offset| self.base_weekday.forward(offset).get() as usize)
    //         .map(|i| WEEKDAYS[i].chars().take(self.cell_width()).collect());
    //     // if user has more freedom to choose cell width and stuff like week names, it's better to
    //     // format this here
    //     // also for the grid footer
    //     // .map(|weekday| LineFormatter::new(cell_width, " ").right(&weekday));

    //     if let Some(filler) = self.format_grid_left_filler() {
    //         Some(self.join_cells(std::iter::once(filler).chain(cells)))
    //     } else {
    //         Some(self.join_cells(cells))
    //     }
    // }

    // /// See struct documentation ([`Self`]).
    // fn format_grid_footer(&self) -> String {
    //     let cells = (0..7)
    //         .map(|offset| self.base_weekday.forward(offset).get() as usize)
    //         .map(|i| WEEKDAYS[i].chars().take(self.cell_width()).collect());
    //     // if user has more freedom to choose cell width and stuff like week names, it's better to
    //     // format this here
    //     // .map(|weekday| LineFormatter::new(cell_width, " ").right(&weekday));

    //     self.join_cells(cells)
    // }

    // /// Given a list of iterators, join them with the cell delimiter.
    // pub fn join_cells(&self, iter: impl Iterator<Item = String>) -> String {
    //     let delim = self.cell_delimiter();
    //     iter.fold(String::new(), |acc, i| {
    //         if acc.is_empty() { i } else { acc + &delim + &i }
    //     })
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn grid_left_side_width(&self) -> usize {
    //     let enabled = if self.vertical {
    //         self.common_weekday != Some(false)
    //     } else {
    //         self.week_config.is_some()
    //     };
    //     if enabled { self.cell_width() } else { 0 }
    // }

    // /// See struct documentation ([`Self`]).
    // pub fn format_grid_left_filler(&self) -> Option<String> {
    //     let width = self.grid_left_side_width();
    //     if width != 0 {
    //         Some(" ".repeat(width))
    //     } else {
    //         None
    //     }
    // }
}

impl Default for Layout {
    /// Create one with defaults based on the common cals.
    fn default() -> Self {
        Self {
            base_row: Default::default(),
            next_row_after_column: 1,
            common_weekday: None,
            highlight: None,
        }
    }
}

/// Maximum 6 weeks of 7 days.
///
/// Needs to be u16 to fit possible ordinals.
type RawGrid = [[UOrdinal; 7]; 6];

#[cfg(test)]
mod tests {
    use jiff::civil;

    use super::*;

    #[test]
    fn test_cells_nov_2025_sun() {
        let nov25_sun = [
            [00, 00, 00, 00, 00, 00, 01],
            [02, 03, 04, 05, 06, 07, 08],
            [09, 10, 11, 12, 13, 14, 15],
            [16, 17, 18, 19, 20, 21, 22],
            [23, 24, 25, 26, 27, 28, 29],
            [30, 00, 00, 00, 00, 00, 00],
        ];

        assert_eq!(
            nov25_sun,
            Grid {
                date: Date::Gregorian(civil::Date::constant(2025, 11, 1)),
                ordinal_mode: false,
                base_weekday: Weekday::SUN
            }
            .new_grid()
        );
    }

    #[test]
    fn test_cells_nov_2025_sat() {
        let nov25_sat = [
            [01, 02, 03, 04, 05, 06, 07],
            [08, 09, 10, 11, 12, 13, 14],
            [15, 16, 17, 18, 19, 20, 21],
            [22, 23, 24, 25, 26, 27, 28],
            [29, 30, 00, 00, 00, 00, 00],
            [00, 00, 00, 00, 00, 00, 00],
        ];

        assert_eq!(
            nov25_sat,
            Grid {
                date: Date::Gregorian(civil::Date::constant(2025, 11, 1)),
                ordinal_mode: false,
                base_weekday: Weekday::SAT
            }
            .new_grid()
        );
    }

    #[test]
    fn test_cells_nov_2025_sun_format() {
        let nov25_sun = vec![
            vec!["  ", "  ", "  ", "  ", "  ", "  ", " 1"],
            vec![" 2", " 3", " 4", " 5", " 6", " 7", " 8"],
            vec![" 9", "10", "11", "12", "13", "14", "15"],
            vec!["16", "17", "18", "19", "20", "21", "22"],
            vec!["23", "24", "25", "26", "27", "28", "29"],
            vec!["30", "  ", "  ", "  ", "  ", "  ", "  "],
        ];

        assert_eq!(
            nov25_sun,
            Grid {
                date: Date::Gregorian(civil::Date::constant(2025, 11, 1)),
                ordinal_mode: false,
                base_weekday: Weekday::SUN
            }
            .format(None)
        );
    }

    #[test]
    fn test_cells_nov_2025_sun_format_ordinal() {
        let nov25_sun = vec![
            vec!["  ", "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
            vec!["43", "   ", "   ", "   ", "   ", "   ", "   ", "305"],
            vec!["44", "306", "307", "308", "309", "310", "311", "312"],
            vec!["45", "313", "314", "315", "316", "317", "318", "319"],
            vec!["46", "320", "321", "322", "323", "324", "325", "326"],
            vec!["47", "327", "328", "329", "330", "331", "332", "333"],
            vec!["48", "334", "   ", "   ", "   ", "   ", "   ", "   "],
        ];

        assert_eq!(
            nov25_sun,
            ColumnContent {
                weeknums: Some(WeekNumConfig::Based),
                weeknums_before_grid: true,
                weekdays: true,
                weekdays_before_grid: true,
                grid: Grid {
                    date: Date::Gregorian(civil::Date::constant(2025, 11, 1)),
                    ordinal_mode: true,
                    base_weekday: Weekday::SUN
                }
            }
            .format(None)
        );

        assert_eq!(
            nov25_sun,
            ColumnContent {
                weeknums: Some(WeekNumConfig::Based),
                weeknums_before_grid: true,
                weekdays: true,
                weekdays_before_grid: true,
                grid: Grid {
                    date: Date::Gregorian(civil::Date::constant(2025, 11, 1)),
                    ordinal_mode: true,
                    base_weekday: Weekday::SUN,
                }
            }
            .format(None)
        );
    }

    #[test]
    fn test_column_nov_2025_sun_ordinal() {
        let nov25_sun = vec![
            "           November           ".to_owned(),
            "  |Sun|Mon|Tue|Wed|Thu|Fri|Sat".to_owned(),
            "43|   |   |   |   |   |   |305".to_owned(),
            "44|306|307|308|309|310|311|312".to_owned(),
            "45|313|314|315|316|317|318|319".to_owned(),
            "46|320|321|322|323|324|325|326".to_owned(),
            "47|327|328|329|330|331|332|333".to_owned(),
            "48|334|   |   |   |   |   |   ".to_owned(),
        ];

        assert_eq!(
            nov25_sun,
            Column {
                content: ColumnContent {
                    weeknums: Some(WeekNumConfig::Based),
                    weeknums_before_grid: true,
                    weekdays: true,
                    weekdays_before_grid: true,
                    grid: Grid {
                        date: Date::Gregorian(civil::Date::constant(2025, 11, 1)),
                        ordinal_mode: true,
                        base_weekday: Weekday::SUN
                    }
                },
                delimiter: "|".to_owned(),
                year_in_header: false,
                vertical: false,
            }
            .format(None)
        );
    }

    #[test]
    fn test_column_nov_2025_sun_vertical_ordinal() {
        let nov25_sun = vec![
            "       November 2025       ".to_owned(),
            "   | 43| 44| 45| 46| 47| 48".to_owned(),
            "Sun|   |306|313|320|327|334".to_owned(),
            "Mon|   |307|314|321|328|   ".to_owned(),
            "Tue|   |308|315|322|329|   ".to_owned(),
            "Wed|   |309|316|323|330|   ".to_owned(),
            "Thu|   |310|317|324|331|   ".to_owned(),
            "Fri|   |311|318|325|332|   ".to_owned(),
            "Sat|305|312|319|326|333|   ".to_owned(),
        ];

        assert_eq!(
            nov25_sun,
            Column {
                content: ColumnContent {
                    weeknums: Some(WeekNumConfig::Based),
                    weeknums_before_grid: true,
                    weekdays: true,
                    weekdays_before_grid: true,
                    grid: Grid {
                        date: Date::Gregorian(civil::Date::constant(2025, 11, 1)),
                        ordinal_mode: true,
                        base_weekday: Weekday::SUN
                    }
                },
                delimiter: "|".to_owned(),
                year_in_header: true,
                vertical: true,
            }
            .format(None)
        );
    }
}
