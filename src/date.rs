//! Holds a generic calendar utilities with predefined and unified calendar relations.

use jelal::{IDayDiff, IYear, MonthDay, Ordinal, UDayDiff, UMonth, UMonthDay, UOrdinal, Weekday};
use jiff::civil;

/// A tuple of 3 values of year, month and day without any checks.
///
/// Any type that implements a `From<this>` and `Into<this>` with no panics or exception (taking the
/// default strategy of saturating and slightly modifying values to fit in this range) will act as a
/// valid calendar for this crate.
pub use jelal::IYmd;

use crate::{GREGORIAN_MONTHS, JALALI_MONTHS};

const JIFF_MIN_YEAR: IYear = -9999;
const JIFF_MAX_YEAR: IYear = 9999;

/// Provides primitive insights for date structs.
///
/// This is the most basic solution for unifying calendars with no explicit enum listing them.
/// Keeping this as a `dyn compatible` trait, helps code like this execute so the user doesn't need
/// to know what is what beforehand and can just implement their own calendar.
///
/// Note that this is not an elegant API and its design encourages careless cloning. To prevent
/// that, consider implementing an enum. For applications with short lifetime like `date` and `cal`,
/// this was a fairly simple to write and use strategy.
pub trait CommonDate {
    /// Return what year it is (limits to boundaries).
    fn year(&self) -> IYear;

    fn set_saturating_year(&mut self, year: IYear);

    /// Return what month it is (1..=12).
    fn month(&self) -> UMonth;

    fn set_saturating_month(&mut self, month: UMonth);

    /// Returns what day of the month is it (1..=31).
    fn day(&self) -> UMonthDay;

    fn set_saturating_day(&mut self, day: UMonthDay);

    /// What day is the first of this month (1..=366).
    // This is used only in ordinal mode so maybe in a cleaner code this is optional or something.
    fn ordinal(&self) -> UOrdinal;

    fn set_saturating_ordinal(&mut self, ordinal: UOrdinal);

    /// What weekday it is.
    fn weekday(&self) -> Weekday;

    /// What week number it is (0..=53).
    fn weeknum(&self, base: Weekday) -> u8
    where
        Self: Clone,
    {
        let mut new_year = self.clone();
        new_year.set_saturating_ordinal(1);
        new_year
            .weekday()
            .count_weeks(self.ordinal() as UDayDiff, &base) as u8
    }

    /// Given a number from 0..=53, set the date to the start of that week.
    ///
    /// Given a number larger than the range may cause saturation to the max ordinal.
    fn set_saturating_weeknum(&mut self, weeks: usize, base: Weekday) {
        let weeks = weeks.clamp(0, 53);
        let offset = base.till_next(&self.weekday()) as UOrdinal;
        self.set_saturating_ordinal(weeks as UOrdinal * 7 + offset) // reset back to that week
    }

    /// What is the maximum day of month (limitations as in [`Self::day`]).
    fn month_end_day(&self) -> UMonthDay;

    /// What is the maximum day of year (limitations as in [`Self::ordinal`]).
    fn year_end_ordinal(&self) -> UOrdinal;

    /// Add or remove a month to this month cross year boundaries and never panic.
    fn set_saturating_months_offset(&mut self, months: IDayDiff) {
        // date handles this smoothly and there is no need for other structs.
        let new =
            jelal::Date::from((self.year(), self.month(), MonthDay::MIN_DAY)).add_months(months);

        // since ordinals are not 1-1 across different calendars, just used all the fields in order
        self.set_saturating_year(CommonDate::year(&new));
        self.set_saturating_month(CommonDate::month(&new));
        self.set_saturating_day(CommonDate::day(&new));
    }

    /// Experimental ISO week number.
    // TODO if iso is defined on other calendars and stuff, move it to commondate
    fn iso_weeknum(&self) -> u8
    where
        Self: Clone,
    {
        let mut v = self.clone();
        v.set_saturating_ordinal(1);
        v.weekday().count_iso_weeks(self.ordinal() as UDayDiff) as u8
    }
}

impl CommonDate for jelal::Date {
    fn year(&self) -> IYear {
        self.year().get()
    }

    fn set_saturating_year(&mut self, year: IYear) {
        *self = (year, self.ordinal()).into();
    }

    fn month(&self) -> UMonth {
        MonthDay::from(self.clone()).month().get()
    }

    fn set_saturating_month(&mut self, month: UMonth) {
        *self = (self.year(), month, self.day()).into();
    }

    fn day(&self) -> UMonthDay {
        MonthDay::from(self.clone()).day()
    }

    fn set_saturating_day(&mut self, day: UMonthDay) {
        *self = (self.year(), self.month(), day).into();
    }

    fn ordinal(&self) -> UOrdinal {
        self.ordinal().get()
    }

    fn set_saturating_ordinal(&mut self, ordinal: UOrdinal) {
        *self = (self.year(), ordinal).into();
    }

    fn weekday(&self) -> Weekday {
        self.weekday()
    }

    fn month_end_day(&self) -> UMonthDay {
        jelal::Date::from((self.year(), self.month(), MonthDay::MAX_DAY)).day()
    }

    fn year_end_ordinal(&self) -> UOrdinal {
        jelal::Date::from((self.year(), Ordinal::MAX))
            .ordinal()
            .get()
    }
}

impl CommonDate for civil::Date {
    fn year(&self) -> IYear {
        self.clone().year() as IYear
    }

    fn set_saturating_year(&mut self, year: IYear) {
        let previous_day = self.day() as u8;
        *self = self
            .with()
            .year(year.clamp(JIFF_MIN_YEAR, JIFF_MAX_YEAR) as i16)
            .month(self.month() as i8)
            .day(1)
            .build()
            .unwrap();
        // using with_day, prevents overflow and corrects invalid dates
        self.set_saturating_day(previous_day);
    }

    fn month(&self) -> UMonth {
        self.clone().month() as UMonth
    }

    fn set_saturating_month(&mut self, month: UMonth) {
        let previous_day = self.day() as u8;
        *self = self
            .with()
            .month(month.clamp(1, 12) as i8)
            .day(1)
            .build()
            .unwrap();
        // using with_day, prevents overflow and corrects invalid dates
        self.set_saturating_day(previous_day);
    }

    fn day(&self) -> UMonthDay {
        self.clone().day() as UMonthDay
    }

    fn set_saturating_day(&mut self, day: UMonthDay) {
        *self = self
            .with()
            .day(day.clamp(1, self.month_end_day()) as i8)
            .build()
            .unwrap();
    }

    fn ordinal(&self) -> UOrdinal {
        self.clone().day_of_year() as UOrdinal
    }

    fn set_saturating_ordinal(&mut self, ordinal: UOrdinal) {
        *self = self
            .with()
            .day_of_year(ordinal.clamp(1, self.year_end_ordinal()) as i16)
            .build()
            .unwrap();
    }

    fn weekday(&self) -> Weekday {
        self.clone().weekday().into()
    }

    fn month_end_day(&self) -> UMonthDay {
        self.clone().last_of_month().day() as UMonthDay
    }

    fn year_end_ordinal(&self) -> UOrdinal {
        self.clone().last_of_year().day_of_year() as UOrdinal
    }
}

/// Holds the calendars that this package concerns.
#[derive(Clone, Debug)]
pub enum Date {
    Jalali(jelal::Date),
    Gregorian(civil::Date),
}

impl Date {
    pub fn common(&self) -> &dyn CommonDate {
        match self {
            Date::Jalali(date) => date,
            Date::Gregorian(date) => date,
        }
    }

    pub fn common_mut(&mut self) -> &mut dyn CommonDate {
        match self {
            Date::Jalali(date) => date,
            Date::Gregorian(date) => date,
        }
    }

    pub fn month_names(&self) -> &'static [&'static str; 12] {
        match self {
            Date::Jalali(_) => &JALALI_MONTHS,
            Date::Gregorian(_) => &GREGORIAN_MONTHS,
        }
    }

    pub fn month_name(&self) -> &'static str {
        self.month_names()[self.month() as usize - 1]
    }
}

impl CommonDate for Date {
    fn year(&self) -> IYear {
        self.common().year()
    }

    fn set_saturating_year(&mut self, year: IYear) {
        self.common_mut().set_saturating_year(year);
    }

    fn month(&self) -> UMonth {
        self.common().month()
    }

    fn set_saturating_month(&mut self, month: UMonth) {
        self.common_mut().set_saturating_month(month);
    }

    fn day(&self) -> UMonthDay {
        self.common().day()
    }

    fn set_saturating_day(&mut self, day: UMonthDay) {
        self.common_mut().set_saturating_day(day);
    }

    fn ordinal(&self) -> UOrdinal {
        self.common().ordinal()
    }

    fn set_saturating_ordinal(&mut self, ordinal: UOrdinal) {
        self.common_mut().set_saturating_ordinal(ordinal);
    }

    fn weekday(&self) -> Weekday {
        self.common().weekday()
    }

    fn month_end_day(&self) -> UMonthDay {
        self.common().month_end_day()
    }

    fn year_end_ordinal(&self) -> UOrdinal {
        self.common().year_end_ordinal()
    }
}

impl PartialEq for Date {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Jalali(j1), Self::Jalali(j2)) => j1 == j2,
            (Self::Gregorian(g1), Self::Gregorian(g2)) => g1 == g2,
            (Self::Gregorian(g), Self::Jalali(j)) | (Self::Jalali(j), Self::Gregorian(g)) => {
                *j == jelal::Date::from(g.clone())
            }
        }
    }
}

impl From<jelal::Date> for Date {
    fn from(value: jelal::Date) -> Self {
        Date::Jalali(value)
    }
}

impl From<civil::Date> for Date {
    fn from(value: civil::Date) -> Self {
        Date::Gregorian(value)
    }
}

impl Default for Date {
    fn default() -> Self {
        Self::Gregorian(civil::Date::constant(1, 1, 1))
    }
}
