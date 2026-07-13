//! Traits for accessing the components of a date (`Datelike`) or a time of
//! day (`Timelike`).

use crate::naive::date::IsoWeek;
use crate::weekday::Weekday;

/// Read access to the components of a date.
///
/// Implemented by every type that represents a date (`NaiveDate`,
/// `NaiveDateTime`, `DateTime<Tz>`). API aligned with `chrono::Datelike`.
pub trait Datelike: Sized {
    /// Year (can be negative or zero: proleptic calendar).
    fn year(&self) -> i32;

    /// The absolute year number starting from 1, paired with `false` if the
    /// year predates the epoch (BCE) or `true` otherwise (CE).
    fn year_ce(&self) -> (bool, u32) {
        let year = self.year();
        if year < 1 {
            (false, (1 - year) as u32)
        } else {
            (true, year as u32)
        }
    }

    /// Quarter of the year, from 1 to 4.
    fn quarter(&self) -> u32 {
        (self.month() - 1) / 3 + 1
    }

    /// Month, from 1 (January) to 12 (December).
    fn month(&self) -> u32;

    /// Month, from 0 (January) to 11 (December).
    fn month0(&self) -> u32;

    /// Day of the month, from 1 to the last day of the month.
    fn day(&self) -> u32;

    /// Day of the month, from 0 to the last day of the month minus one.
    fn day0(&self) -> u32;

    /// Day of the year, from 1 to 365 (or 366 in leap years).
    fn ordinal(&self) -> u32;

    /// Day of the year, from 0 to 364 (or 365 in leap years).
    fn ordinal0(&self) -> u32;

    /// Day of the week.
    fn weekday(&self) -> Weekday;

    /// ISO 8601 week (year + week number).
    fn iso_week(&self) -> IsoWeek;

    /// Returns a copy with the year replaced, or `None` if the resulting
    /// date does not exist (e.g. February 29th on a non-leap year).
    fn with_year(&self, year: i32) -> Option<Self>;

    /// Returns a copy with the month replaced (1..=12), or `None` if
    /// invalid.
    fn with_month(&self, month: u32) -> Option<Self>;

    /// Returns a copy with the month replaced (0..=11), or `None` if
    /// invalid.
    fn with_month0(&self, month0: u32) -> Option<Self>;

    /// Returns a copy with the day of the month replaced, or `None` if
    /// invalid.
    fn with_day(&self, day: u32) -> Option<Self>;

    /// Returns a copy with the day of the month replaced (0-indexed), or
    /// `None` if invalid.
    fn with_day0(&self, day0: u32) -> Option<Self>;

    /// Returns a copy with the day of the year replaced, or `None` if
    /// invalid.
    fn with_ordinal(&self, ordinal: u32) -> Option<Self>;

    /// Returns a copy with the day of the year replaced (0-indexed), or
    /// `None` if invalid.
    fn with_ordinal0(&self, ordinal0: u32) -> Option<Self>;

    /// Counts the days in the proleptic Gregorian calendar, with January
    /// 1st, year 1 (CE) as day 1.
    fn num_days_from_ce(&self) -> i32 {
        (crate::calendar::days_from_civil(self.year(), self.month(), self.day())
            - crate::calendar::EPOCH_OFFSET_FROM_CE
            + 1) as i32
    }

    /// Length in days of the month this date falls in.
    fn num_days_in_month(&self) -> u8 {
        crate::calendar::days_in_month(self.year(), self.month()) as u8
    }
}

/// Read access to the components of a time of day.
///
/// Implemented by every type that represents a time of day (`NaiveTime`,
/// `NaiveDateTime`, `DateTime<Tz>`). API aligned with `chrono::Timelike`.
pub trait Timelike: Sized {
    /// Hour, from 0 to 23.
    fn hour(&self) -> u32;

    /// Hour, from 1 to 12, for 12-hour clock display, paired with whether
    /// it is before (`false`) or after/at (`true`) noon.
    fn hour12(&self) -> (bool, u32) {
        let hour = self.hour();
        let hour12 = match hour % 12 {
            0 => 12,
            h => h,
        };
        (hour >= 12, hour12)
    }

    /// Minute, from 0 to 59.
    fn minute(&self) -> u32;

    /// Second, from 0 to 59. Never returns 60, even for a leap second: use
    /// formatting to display leap seconds in human-readable form.
    fn second(&self) -> u32;

    /// Nanoseconds since the last whole non-leap second. The range
    /// `1_000_000_000..2_000_000_000` represents a leap second.
    fn nanosecond(&self) -> u32;

    /// Returns a copy with the hour replaced, or `None` if `hour >= 24`.
    fn with_hour(&self, hour: u32) -> Option<Self>;

    /// Returns a copy with the minute replaced, or `None` if `min >= 60`.
    fn with_minute(&self, min: u32) -> Option<Self>;

    /// Returns a copy with the second replaced, or `None` if `sec >= 60`.
    /// As with [`second`](Self::second), leap seconds cannot be set this
    /// way; use [`with_nanosecond`](Self::with_nanosecond) instead.
    fn with_second(&self, sec: u32) -> Option<Self>;

    /// Returns a copy with the nanosecond field replaced, or `None` if
    /// `nano >= 2_000_000_000`. Values of 1_000_000_000 and above set a
    /// leap second.
    fn with_nanosecond(&self, nano: u32) -> Option<Self>;

    /// Number of non-leap seconds since midnight.
    fn num_seconds_from_midnight(&self) -> u32 {
        self.hour() * 3600 + self.minute() * 60 + self.second()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NaiveDate, NaiveTime};

    // These are default trait methods with no per-type override, so
    // `NaiveDate`/`NaiveTime` are used here only as concrete stand-ins to
    // exercise the default-method arithmetic itself.

    #[test]
    fn year_ce_reports_bce_and_ce_correctly() {
        let ce = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        assert_eq!(ce.year_ce(), (true, 2023));
        let bce = NaiveDate::from_ymd_opt(0, 1, 1).unwrap(); // year 0 = "1 BCE"
        assert_eq!(bce.year_ce(), (false, 1));
        let bce2 = NaiveDate::from_ymd_opt(-1, 1, 1).unwrap(); // year -1 = "2 BCE"
        assert_eq!(bce2.year_ce(), (false, 2));
    }

    #[test]
    fn quarter_maps_months_to_quarters() {
        let months_and_quarters =
            [(1, 1), (3, 1), (4, 2), (6, 2), (7, 3), (9, 3), (10, 4), (12, 4)];
        for (month, expected_q) in months_and_quarters {
            let date = NaiveDate::from_ymd_opt(2023, month, 1).unwrap();
            assert_eq!(date.quarter(), expected_q);
        }
    }

    #[test]
    fn num_days_from_ce_round_trips_with_from_num_days_from_ce_opt() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let n = date.num_days_from_ce();
        assert_eq!(NaiveDate::from_num_days_from_ce_opt(n).unwrap(), date);
    }

    #[test]
    fn num_days_from_ce_of_ce_epoch_is_one() {
        let date = NaiveDate::from_ymd_opt(1, 1, 1).unwrap();
        assert_eq!(date.num_days_from_ce(), 1);
    }

    #[test]
    fn num_days_in_month_accounts_for_leap_years() {
        assert_eq!(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap().num_days_in_month(), 29);
        assert_eq!(NaiveDate::from_ymd_opt(2023, 2, 1).unwrap().num_days_in_month(), 28);
        assert_eq!(NaiveDate::from_ymd_opt(2023, 4, 1).unwrap().num_days_in_month(), 30);
        assert_eq!(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().num_days_in_month(), 31);
    }

    #[test]
    fn hour12_maps_24_hour_to_12_hour_clock() {
        assert_eq!(NaiveTime::from_hms_opt(0, 0, 0).unwrap().hour12(), (false, 12));
        assert_eq!(NaiveTime::from_hms_opt(9, 0, 0).unwrap().hour12(), (false, 9));
        assert_eq!(NaiveTime::from_hms_opt(12, 0, 0).unwrap().hour12(), (true, 12));
        assert_eq!(NaiveTime::from_hms_opt(23, 0, 0).unwrap().hour12(), (true, 11));
    }

    #[test]
    fn num_seconds_from_midnight_computes_elapsed_seconds() {
        let t = NaiveTime::from_hms_opt(1, 2, 3).unwrap();
        assert_eq!(t.num_seconds_from_midnight(), 1 * 3600 + 2 * 60 + 3);
    }
}
