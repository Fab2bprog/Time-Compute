//! A collection of parsed date and time items.
//!
//! They can be constructed incrementally while being checked for consistency.

use super::{ParseResult, IMPOSSIBLE, NOT_ENOUGH, OUT_OF_RANGE};
use crate::naive::{NaiveDate, NaiveDateTime, NaiveTime};
use crate::offset::{FixedOffset, MappedLocalTime, Offset, TimeZone};
use crate::{DateTime, Datelike, TimeDelta, Timelike, Weekday};

/// A type to hold parsed fields of date and time, that can check all fields
/// for consistency.
///
/// There are three classes of methods:
///
/// - `set_*` methods to set fields you have available. They do a basic range
///   check, and if the same field is set more than once it is checked for
///   consistency.
/// - `to_*` methods try to build a concrete date and time value out of the
///   set fields. They fully check that all fields are consistent and that
///   the date/datetime exists.
/// - methods to inspect the parsed fields.
///
/// `Parsed` is used internally by all parsing functions in this crate. It is
/// a public type so that it can be used to write custom parsers that reuse
/// the resolving algorithm, or to inspect the results of a string parse
/// without converting it to a concrete type.
#[allow(clippy::manual_non_exhaustive)]
#[derive(Clone, PartialEq, Eq, Debug, Default, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Parsed {
    #[doc(hidden)]
    pub year: Option<i32>,
    #[doc(hidden)]
    pub year_div_100: Option<i32>,
    #[doc(hidden)]
    pub year_mod_100: Option<i32>,
    #[doc(hidden)]
    pub isoyear: Option<i32>,
    #[doc(hidden)]
    pub isoyear_div_100: Option<i32>,
    #[doc(hidden)]
    pub isoyear_mod_100: Option<i32>,
    #[doc(hidden)]
    pub quarter: Option<u32>,
    #[doc(hidden)]
    pub month: Option<u32>,
    #[doc(hidden)]
    pub week_from_sun: Option<u32>,
    #[doc(hidden)]
    pub week_from_mon: Option<u32>,
    #[doc(hidden)]
    pub isoweek: Option<u32>,
    #[doc(hidden)]
    pub weekday: Option<Weekday>,
    #[doc(hidden)]
    pub ordinal: Option<u32>,
    #[doc(hidden)]
    pub day: Option<u32>,
    #[doc(hidden)]
    pub hour_div_12: Option<u32>,
    #[doc(hidden)]
    pub hour_mod_12: Option<u32>,
    #[doc(hidden)]
    pub minute: Option<u32>,
    #[doc(hidden)]
    pub second: Option<u32>,
    #[doc(hidden)]
    pub nanosecond: Option<u32>,
    #[doc(hidden)]
    pub timestamp: Option<i64>,
    #[doc(hidden)]
    pub offset: Option<i32>,
    #[doc(hidden)]
    _dummy: (),
}

/// Checks if `old` is either empty or has the same value as `new` (i.e.
/// "consistent"), and if it is empty, sets `old` to `new` as well.
#[inline]
fn set_if_consistent<T: PartialEq>(old: &mut Option<T>, new: T) -> ParseResult<()> {
    match old {
        Some(old) if *old != new => Err(IMPOSSIBLE),
        _ => {
            *old = Some(new);
            Ok(())
        }
    }
}

impl Parsed {
    /// Returns the initial value of parsed parts.
    #[must_use]
    pub fn new() -> Parsed {
        Parsed::default()
    }

    /// Sets the [`year`](Parsed::year) field. The value can be negative.
    #[inline]
    pub fn set_year(&mut self, value: i64) -> ParseResult<()> {
        set_if_consistent(&mut self.year, i32::try_from(value).map_err(|_| OUT_OF_RANGE)?)
    }

    /// Sets the [`year_div_100`](Parsed::year_div_100) field.
    #[inline]
    pub fn set_year_div_100(&mut self, value: i64) -> ParseResult<()> {
        if !(0..=i32::MAX as i64).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.year_div_100, value as i32)
    }

    /// Sets the [`year_mod_100`](Parsed::year_mod_100) field.
    #[inline]
    pub fn set_year_mod_100(&mut self, value: i64) -> ParseResult<()> {
        if !(0..100).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.year_mod_100, value as i32)
    }

    /// Sets the [`isoyear`](Parsed::isoyear) field, part of an ISO 8601 week date.
    #[inline]
    pub fn set_isoyear(&mut self, value: i64) -> ParseResult<()> {
        set_if_consistent(&mut self.isoyear, i32::try_from(value).map_err(|_| OUT_OF_RANGE)?)
    }

    /// Sets the [`isoyear_div_100`](Parsed::isoyear_div_100) field.
    #[inline]
    pub fn set_isoyear_div_100(&mut self, value: i64) -> ParseResult<()> {
        if !(0..=i32::MAX as i64).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.isoyear_div_100, value as i32)
    }

    /// Sets the [`isoyear_mod_100`](Parsed::isoyear_mod_100) field.
    #[inline]
    pub fn set_isoyear_mod_100(&mut self, value: i64) -> ParseResult<()> {
        if !(0..100).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.isoyear_mod_100, value as i32)
    }

    /// Sets the [`quarter`](Parsed::quarter) field (1 through 4, quarter 1 starts in January).
    #[inline]
    pub fn set_quarter(&mut self, value: i64) -> ParseResult<()> {
        if !(1..=4).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.quarter, value as u32)
    }

    /// Sets the [`month`](Parsed::month) field (1 through 12).
    #[inline]
    pub fn set_month(&mut self, value: i64) -> ParseResult<()> {
        if !(1..=12).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.month, value as u32)
    }

    /// Sets the [`week_from_sun`](Parsed::week_from_sun) field (week 1 starts at the first Sunday of January).
    #[inline]
    pub fn set_week_from_sun(&mut self, value: i64) -> ParseResult<()> {
        if !(0..=53).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.week_from_sun, value as u32)
    }

    /// Sets the [`week_from_mon`](Parsed::week_from_mon) field (week 1 starts at the first Monday of January).
    #[inline]
    pub fn set_week_from_mon(&mut self, value: i64) -> ParseResult<()> {
        if !(0..=53).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.week_from_mon, value as u32)
    }

    /// Sets the [`isoweek`](Parsed::isoweek) field, part of an ISO 8601 week date (1 through 53).
    #[inline]
    pub fn set_isoweek(&mut self, value: i64) -> ParseResult<()> {
        if !(1..=53).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.isoweek, value as u32)
    }

    /// Sets the [`weekday`](Parsed::weekday) field.
    #[inline]
    pub fn set_weekday(&mut self, value: Weekday) -> ParseResult<()> {
        set_if_consistent(&mut self.weekday, value)
    }

    /// Sets the [`ordinal`](Parsed::ordinal) (day of the year) field (1 through 366).
    #[inline]
    pub fn set_ordinal(&mut self, value: i64) -> ParseResult<()> {
        if !(1..=366).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.ordinal, value as u32)
    }

    /// Sets the [`day`](Parsed::day) of the month field (1 through 31).
    #[inline]
    pub fn set_day(&mut self, value: i64) -> ParseResult<()> {
        if !(1..=31).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.day, value as u32)
    }

    /// Sets the [`hour_div_12`](Parsed::hour_div_12) am/pm field
    /// (`false` indicates AM, `true` indicates PM).
    #[inline]
    pub fn set_ampm(&mut self, value: bool) -> ParseResult<()> {
        set_if_consistent(&mut self.hour_div_12, value as u32)
    }

    /// Sets the [`hour_mod_12`](Parsed::hour_mod_12) field for a 12-hour clock (1 through 12,
    /// stored internally as 0 through 11).
    #[inline]
    pub fn set_hour12(&mut self, mut value: i64) -> ParseResult<()> {
        if !(1..=12).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        if value == 12 {
            value = 0
        }
        set_if_consistent(&mut self.hour_mod_12, value as u32)
    }

    /// Sets [`hour_div_12`](Parsed::hour_div_12) and [`hour_mod_12`](Parsed::hour_mod_12)
    /// for a 24-hour clock value (0 through 23).
    #[inline]
    pub fn set_hour(&mut self, value: i64) -> ParseResult<()> {
        let (hour_div_12, hour_mod_12) = match value {
            hour @ 0..=11 => (0, hour as u32),
            hour @ 12..=23 => (1, hour as u32 - 12),
            _ => return Err(OUT_OF_RANGE),
        };
        set_if_consistent(&mut self.hour_div_12, hour_div_12)?;
        set_if_consistent(&mut self.hour_mod_12, hour_mod_12)
    }

    /// Sets the [`minute`](Parsed::minute) field (0 through 59).
    #[inline]
    pub fn set_minute(&mut self, value: i64) -> ParseResult<()> {
        if !(0..=59).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.minute, value as u32)
    }

    /// Sets the [`second`](Parsed::second) field (0 through 60, 60 for a leap second).
    #[inline]
    pub fn set_second(&mut self, value: i64) -> ParseResult<()> {
        if !(0..=60).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.second, value as u32)
    }

    /// Sets the [`nanosecond`](Parsed::nanosecond) field (0 through 999,999,999).
    #[inline]
    pub fn set_nanosecond(&mut self, value: i64) -> ParseResult<()> {
        if !(0..=999_999_999).contains(&value) {
            return Err(OUT_OF_RANGE);
        }
        set_if_consistent(&mut self.nanosecond, value as u32)
    }

    /// Sets the [`timestamp`](Parsed::timestamp) field: the number of non-leap
    /// seconds since midnight UTC on January 1, 1970.
    #[inline]
    pub fn set_timestamp(&mut self, value: i64) -> ParseResult<()> {
        set_if_consistent(&mut self.timestamp, value)
    }

    /// Sets the [`offset`](Parsed::offset) field: seconds from local time to UTC.
    #[inline]
    pub fn set_offset(&mut self, value: i64) -> ParseResult<()> {
        set_if_consistent(&mut self.offset, i32::try_from(value).map_err(|_| OUT_OF_RANGE)?)
    }

    /// Returns a parsed naive date out of the given fields.
    ///
    /// This method is able to determine the date from the given subset of fields:
    /// - year, month, day;
    /// - year, day of the year (ordinal);
    /// - year, week number counted from Sunday or Monday, day of the week;
    /// - ISO week date.
    ///
    /// It checks all given date fields are consistent with each other.
    pub fn to_naive_date(&self) -> ParseResult<NaiveDate> {
        fn resolve_year(
            y: Option<i32>,
            q: Option<i32>,
            r: Option<i32>,
        ) -> ParseResult<Option<i32>> {
            match (y, q, r) {
                // if there is no further information, simply return the given full year.
                (y, None, None) => Ok(y),

                // if there is a full year *and* also quotient and/or modulo,
                // check if the present quotient and/or modulo is consistent with the full year.
                (Some(y), q, r @ Some(0..=99)) | (Some(y), q, r @ None) => {
                    if y < 0 {
                        return Err(IMPOSSIBLE);
                    }
                    let q_ = y / 100;
                    let r_ = y % 100;
                    if q.unwrap_or(q_) == q_ && r.unwrap_or(r_) == r_ {
                        Ok(Some(y))
                    } else {
                        Err(IMPOSSIBLE)
                    }
                }

                // the full year is missing but we have quotient and modulo.
                // reconstruct the full year, always positive.
                (None, Some(q), Some(r @ 0..=99)) => {
                    if q < 0 {
                        return Err(IMPOSSIBLE);
                    }
                    let y = q.checked_mul(100).and_then(|v| v.checked_add(r));
                    Ok(Some(y.ok_or(OUT_OF_RANGE)?))
                }

                // we only have the modulo. interpret it as a conventional two-digit year.
                (None, None, Some(r @ 0..=99)) => Ok(Some(r + if r < 70 { 2000 } else { 1900 })),

                // otherwise it is an out-of-bound or insufficient condition.
                (None, Some(_), None) => Err(NOT_ENOUGH),
                (_, _, Some(_)) => Err(OUT_OF_RANGE),
            }
        }

        let given_year = resolve_year(self.year, self.year_div_100, self.year_mod_100)?;
        let given_isoyear = resolve_year(self.isoyear, self.isoyear_div_100, self.isoyear_mod_100)?;

        // verify the normal year-month-day date.
        let verify_ymd = |date: NaiveDate| {
            let year = date.year();
            let (year_div_100, year_mod_100) = if year >= 0 {
                (Some(year / 100), Some(year % 100))
            } else {
                (None, None) // they should be empty to be consistent
            };
            let month = date.month();
            let day = date.day();
            self.year.unwrap_or(year) == year
                && self.year_div_100.or(year_div_100) == year_div_100
                && self.year_mod_100.or(year_mod_100) == year_mod_100
                && self.month.unwrap_or(month) == month
                && self.day.unwrap_or(day) == day
        };

        // verify the ISO week date.
        let verify_isoweekdate = |date: NaiveDate| {
            let week = date.iso_week();
            let isoyear = week.year();
            let isoweek = week.week();
            let weekday = date.weekday();
            let (isoyear_div_100, isoyear_mod_100) = if isoyear >= 0 {
                (Some(isoyear / 100), Some(isoyear % 100))
            } else {
                (None, None) // they should be empty to be consistent
            };
            self.isoyear.unwrap_or(isoyear) == isoyear
                && self.isoyear_div_100.or(isoyear_div_100) == isoyear_div_100
                && self.isoyear_mod_100.or(isoyear_mod_100) == isoyear_mod_100
                && self.isoweek.unwrap_or(isoweek) == isoweek
                && self.weekday.unwrap_or(weekday) == weekday
        };

        // verify the ordinal and other (non-ISO) week dates.
        let verify_ordinal = |date: NaiveDate| {
            let ordinal = date.ordinal();
            let week_from_sun = date.weeks_from(Weekday::Sun);
            let week_from_mon = date.weeks_from(Weekday::Mon);
            self.ordinal.unwrap_or(ordinal) == ordinal
                && self.week_from_sun.map_or(week_from_sun, |v| v as i32) == week_from_sun
                && self.week_from_mon.map_or(week_from_mon, |v| v as i32) == week_from_mon
        };

        // test several possibilities.
        // tries to construct a full `NaiveDate` as much as possible, then
        // verifies that it is consistent with the other given fields.
        let (verified, parsed_date) = match (given_year, given_isoyear, self) {
            (Some(year), _, &Parsed { month: Some(month), day: Some(day), .. }) => {
                // year, month, day
                let date = NaiveDate::from_ymd_opt(year, month, day).ok_or(OUT_OF_RANGE)?;
                (verify_isoweekdate(date) && verify_ordinal(date), date)
            }

            (Some(year), _, &Parsed { ordinal: Some(ordinal), .. }) => {
                // year, day of the year
                let date = NaiveDate::from_yo_opt(year, ordinal).ok_or(OUT_OF_RANGE)?;
                (verify_ymd(date) && verify_isoweekdate(date) && verify_ordinal(date), date)
            }

            (Some(year), _, &Parsed { week_from_sun: Some(week), weekday: Some(weekday), .. }) => {
                // year, week (starting at 1st Sunday), day of the week
                let date = resolve_week_date(year, week, weekday, Weekday::Sun)?;
                (verify_ymd(date) && verify_isoweekdate(date) && verify_ordinal(date), date)
            }

            (Some(year), _, &Parsed { week_from_mon: Some(week), weekday: Some(weekday), .. }) => {
                // year, week (starting at 1st Monday), day of the week
                let date = resolve_week_date(year, week, weekday, Weekday::Mon)?;
                (verify_ymd(date) && verify_isoweekdate(date) && verify_ordinal(date), date)
            }

            (_, Some(isoyear), &Parsed { isoweek: Some(isoweek), weekday: Some(weekday), .. }) => {
                // ISO year, week, day of the week
                let date = NaiveDate::from_isoywd_opt(isoyear, isoweek, weekday);
                let date = date.ok_or(OUT_OF_RANGE)?;
                (verify_ymd(date) && verify_ordinal(date), date)
            }

            (_, _, _) => return Err(NOT_ENOUGH),
        };

        if !verified {
            return Err(IMPOSSIBLE);
        } else if let Some(parsed) = self.quarter {
            if parsed != parsed_date.quarter() {
                return Err(IMPOSSIBLE);
            }
        }

        Ok(parsed_date)
    }

    /// Returns a parsed naive time out of the given fields.
    ///
    /// This method is able to determine the time from the given subset of fields:
    /// - hour, minute (second and nanosecond assumed to be 0);
    /// - hour, minute, second (nanosecond assumed to be 0);
    /// - hour, minute, second, nanosecond.
    ///
    /// It is able to handle leap seconds when the given second is 60.
    pub fn to_naive_time(&self) -> ParseResult<NaiveTime> {
        let hour_div_12 = match self.hour_div_12 {
            Some(v @ 0..=1) => v,
            Some(_) => return Err(OUT_OF_RANGE),
            None => return Err(NOT_ENOUGH),
        };
        let hour_mod_12 = match self.hour_mod_12 {
            Some(v @ 0..=11) => v,
            Some(_) => return Err(OUT_OF_RANGE),
            None => return Err(NOT_ENOUGH),
        };
        let hour = hour_div_12 * 12 + hour_mod_12;

        let minute = match self.minute {
            Some(v @ 0..=59) => v,
            Some(_) => return Err(OUT_OF_RANGE),
            None => return Err(NOT_ENOUGH),
        };

        // we allow omitting seconds or nanoseconds, but they should be in range.
        let (second, mut nano) = match self.second.unwrap_or(0) {
            v @ 0..=59 => (v, 0),
            60 => (59, 1_000_000_000),
            _ => return Err(OUT_OF_RANGE),
        };
        nano += match self.nanosecond {
            Some(v @ 0..=999_999_999) if self.second.is_some() => v,
            Some(0..=999_999_999) => return Err(NOT_ENOUGH), // second is missing
            Some(_) => return Err(OUT_OF_RANGE),
            None => 0,
        };

        NaiveTime::from_hms_nano_opt(hour, minute, second, nano).ok_or(OUT_OF_RANGE)
    }

    /// Returns a parsed naive date and time out of the given fields, except
    /// for the offset field.
    ///
    /// The offset is assumed to have the given value; it is not compared
    /// against the offset field set in `self`, so it is allowed to be
    /// inconsistent.
    ///
    /// This method is able to determine the combined date and time from date
    /// and time fields, or from a single timestamp field. It checks all
    /// fields are consistent with each other.
    pub fn to_naive_datetime_with_offset(&self, offset: i32) -> ParseResult<NaiveDateTime> {
        let date = self.to_naive_date();
        let time = self.to_naive_time();
        if let (Ok(date), Ok(time)) = (date, time) {
            let datetime = date.and_time(time);

            // verify the timestamp field if any
            let timestamp = datetime.and_utc().timestamp() - i64::from(offset);
            if let Some(given_timestamp) = self.timestamp {
                // if `datetime` represents a leap second, it might be off by one second.
                if given_timestamp != timestamp
                    && !(datetime.nanosecond() >= 1_000_000_000 && given_timestamp == timestamp + 1)
                {
                    return Err(IMPOSSIBLE);
                }
            }

            Ok(datetime)
        } else if let Some(timestamp) = self.timestamp {
            use super::ParseError as PE;
            use super::ParseErrorKind::{Impossible, OutOfRange};

            // if date and time is problematic already, there is no point proceeding.
            match (date, time) {
                (Err(PE(OutOfRange)), _) | (_, Err(PE(OutOfRange))) => return Err(OUT_OF_RANGE),
                (Err(PE(Impossible)), _) | (_, Err(PE(Impossible))) => return Err(IMPOSSIBLE),
                (_, _) => {} // one of them is insufficient
            }

            // reconstruct date and time fields from timestamp
            let ts = timestamp.checked_add(i64::from(offset)).ok_or(OUT_OF_RANGE)?;
            let mut datetime = DateTime::from_timestamp_secs(ts).ok_or(OUT_OF_RANGE)?.naive_utc();

            // fill year, ordinal, hour, minute and second fields from timestamp.
            let mut parsed = self.clone();
            if parsed.second == Some(60) {
                // `datetime.second()` cannot be 60, so this is the only case for a leap second.
                match datetime.second() {
                    59 => {} // it's okay, just do not try to overwrite the existing field.
                    0 => {
                        // `datetime` is known to be off by one second.
                        datetime -= TimeDelta::try_seconds(1).unwrap();
                    }
                    _ => return Err(IMPOSSIBLE), // otherwise it is impossible.
                }
            } else {
                parsed.set_second(i64::from(datetime.second()))?;
            }
            parsed.set_year(i64::from(datetime.year()))?;
            parsed.set_ordinal(i64::from(datetime.ordinal()))?; // more efficient than ymd
            parsed.set_hour(i64::from(datetime.hour()))?;
            parsed.set_minute(i64::from(datetime.minute()))?;

            // validate other fields (e.g. week) and return
            let date = parsed.to_naive_date()?;
            let time = parsed.to_naive_time()?;
            Ok(date.and_time(time))
        } else {
            // reproduce the previous error(s)
            date?;
            time?;
            unreachable!()
        }
    }

    /// Returns a parsed fixed time zone offset out of the given fields.
    pub fn to_fixed_offset(&self) -> ParseResult<FixedOffset> {
        FixedOffset::east_opt(self.offset.ok_or(NOT_ENOUGH)?).ok_or(OUT_OF_RANGE)
    }

    /// Returns a parsed timezone-aware date and time out of the given fields.
    ///
    /// This method is able to determine the combined date and time from
    /// date, time and offset fields, and/or from a single timestamp field.
    /// It checks all fields are consistent with each other.
    pub fn to_datetime(&self) -> ParseResult<DateTime<FixedOffset>> {
        // If there is no explicit offset, consider a timestamp value as
        // indication of a UTC value.
        let offset = match (self.offset, self.timestamp) {
            (Some(off), _) => off,
            (None, Some(_)) => 0, // UNIX timestamp may assume 0 offset
            (None, None) => return Err(NOT_ENOUGH),
        };
        let datetime = self.to_naive_datetime_with_offset(offset)?;
        let offset = FixedOffset::east_opt(offset).ok_or(OUT_OF_RANGE)?;

        match offset.from_local_datetime(&datetime) {
            MappedLocalTime::None => Err(IMPOSSIBLE),
            MappedLocalTime::Single(t) => Ok(t),
            MappedLocalTime::Ambiguous(..) => Err(NOT_ENOUGH),
        }
    }

    /// Returns a parsed timezone-aware date and time out of the given
    /// fields, with an additional [`TimeZone`] used to interpret and
    /// validate the local date.
    ///
    /// If the parsed fields include a UTC offset, it also has to be
    /// consistent with the offset in the provided `tz` time zone for that
    /// datetime.
    pub fn to_datetime_with_timezone<Tz: TimeZone>(&self, tz: &Tz) -> ParseResult<DateTime<Tz>> {
        // if we have `timestamp` specified, guess an offset from that.
        let mut guessed_offset = 0;
        if let Some(timestamp) = self.timestamp {
            let nanosecond = self.nanosecond.unwrap_or(0);
            let dt =
                DateTime::from_timestamp(timestamp, nanosecond).ok_or(OUT_OF_RANGE)?.naive_utc();
            guessed_offset = tz.offset_from_utc_datetime(&dt).fix().local_minus_utc();
        }

        // checks if the given `DateTime` has a consistent `Offset` with `self.offset`.
        let check_offset = |dt: &DateTime<Tz>| {
            if let Some(offset) = self.offset {
                dt.offset().fix().local_minus_utc() == offset
            } else {
                true
            }
        };

        // `guessed_offset` should be correct when `self.timestamp` is given.
        // it will be 0 otherwise, but this is fine as the algorithm ignores
        // offset for that case.
        let datetime = self.to_naive_datetime_with_offset(guessed_offset)?;
        match tz.from_local_datetime(&datetime) {
            MappedLocalTime::None => Err(IMPOSSIBLE),
            MappedLocalTime::Single(t) => {
                if check_offset(&t) {
                    Ok(t)
                } else {
                    Err(IMPOSSIBLE)
                }
            }
            MappedLocalTime::Ambiguous(min, max) => {
                // try to disambiguate two possible local dates by offset.
                match (check_offset(&min), check_offset(&max)) {
                    (false, false) => Err(IMPOSSIBLE),
                    (false, true) => Ok(max),
                    (true, false) => Ok(min),
                    (true, true) => Err(NOT_ENOUGH),
                }
            }
        }
    }

    /// Gets the `year` field if set.
    #[inline]
    pub fn year(&self) -> Option<i32> {
        self.year
    }

    /// Gets the `year_div_100` field if set.
    #[inline]
    pub fn year_div_100(&self) -> Option<i32> {
        self.year_div_100
    }

    /// Gets the `year_mod_100` field if set.
    #[inline]
    pub fn year_mod_100(&self) -> Option<i32> {
        self.year_mod_100
    }

    /// Gets the `isoyear` field if set.
    #[inline]
    pub fn isoyear(&self) -> Option<i32> {
        self.isoyear
    }

    /// Gets the `isoyear_div_100` field if set.
    #[inline]
    pub fn isoyear_div_100(&self) -> Option<i32> {
        self.isoyear_div_100
    }

    /// Gets the `isoyear_mod_100` field if set.
    #[inline]
    pub fn isoyear_mod_100(&self) -> Option<i32> {
        self.isoyear_mod_100
    }

    /// Gets the `quarter` field if set.
    #[inline]
    pub fn quarter(&self) -> Option<u32> {
        self.quarter
    }

    /// Gets the `month` field if set.
    #[inline]
    pub fn month(&self) -> Option<u32> {
        self.month
    }

    /// Gets the `week_from_sun` field if set.
    #[inline]
    pub fn week_from_sun(&self) -> Option<u32> {
        self.week_from_sun
    }

    /// Gets the `week_from_mon` field if set.
    #[inline]
    pub fn week_from_mon(&self) -> Option<u32> {
        self.week_from_mon
    }

    /// Gets the `isoweek` field if set.
    #[inline]
    pub fn isoweek(&self) -> Option<u32> {
        self.isoweek
    }

    /// Gets the `weekday` field if set.
    #[inline]
    pub fn weekday(&self) -> Option<Weekday> {
        self.weekday
    }

    /// Gets the `ordinal` (day of the year) field if set.
    #[inline]
    pub fn ordinal(&self) -> Option<u32> {
        self.ordinal
    }

    /// Gets the `day` of the month field if set.
    #[inline]
    pub fn day(&self) -> Option<u32> {
        self.day
    }

    /// Gets the `hour_div_12` field (am/pm) if set.
    #[inline]
    pub fn hour_div_12(&self) -> Option<u32> {
        self.hour_div_12
    }

    /// Gets the `hour_mod_12` field if set.
    pub fn hour_mod_12(&self) -> Option<u32> {
        self.hour_mod_12
    }

    /// Gets the `minute` field if set.
    #[inline]
    pub fn minute(&self) -> Option<u32> {
        self.minute
    }

    /// Gets the `second` field if set.
    #[inline]
    pub fn second(&self) -> Option<u32> {
        self.second
    }

    /// Gets the `nanosecond` field if set.
    #[inline]
    pub fn nanosecond(&self) -> Option<u32> {
        self.nanosecond
    }

    /// Gets the `timestamp` field if set.
    #[inline]
    pub fn timestamp(&self) -> Option<i64> {
        self.timestamp
    }

    /// Gets the `offset` field if set.
    #[inline]
    pub fn offset(&self) -> Option<i32> {
        self.offset
    }
}

/// Creates a `NaiveDate` from a year, week, weekday, and the definition of
/// which day of the week a week starts.
fn resolve_week_date(
    year: i32,
    week: u32,
    weekday: Weekday,
    week_start_day: Weekday,
) -> ParseResult<NaiveDate> {
    if week > 53 {
        return Err(OUT_OF_RANGE);
    }

    let first_day_of_year = NaiveDate::from_yo_opt(year, 1).ok_or(OUT_OF_RANGE)?;
    // Ordinal of the day at which week 1 starts.
    let first_week_start = 1 + week_start_day.days_since(first_day_of_year.weekday()) as i32;
    // Number of the `weekday`, which is 0 for the first day of the week.
    let weekday = weekday.days_since(week_start_day) as i32;
    let ordinal = first_week_start + (week as i32 - 1) * 7 + weekday;
    if ordinal <= 0 {
        return Err(IMPOSSIBLE);
    }
    first_day_of_year.with_ordinal(ordinal as u32).ok_or(IMPOSSIBLE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Datelike, Timelike, Utc};

    #[test]
    fn set_if_consistent_allows_same_value_twice_but_rejects_conflict() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_year(2023).unwrap(); // same value again: fine
        assert_eq!(p.set_year(2024), Err(IMPOSSIBLE));
    }

    #[test]
    fn getters_return_none_when_unset_and_some_after_set() {
        let mut p = Parsed::new();
        assert_eq!(p.year(), None);
        p.set_year(10).unwrap();
        assert_eq!(p.year(), Some(10));
    }

    #[test]
    fn set_month_rejects_out_of_range() {
        let mut p = Parsed::new();
        assert_eq!(p.set_month(0), Err(OUT_OF_RANGE));
        assert_eq!(p.set_month(13), Err(OUT_OF_RANGE));
        p.set_month(6).unwrap();
        assert_eq!(p.month(), Some(6));
    }

    #[test]
    fn set_hour12_maps_twelve_to_zero() {
        let mut p = Parsed::new();
        p.set_hour12(12).unwrap();
        assert_eq!(p.hour_mod_12(), Some(0));

        let mut p2 = Parsed::new();
        p2.set_hour12(5).unwrap();
        assert_eq!(p2.hour_mod_12(), Some(5));
        assert_eq!(p2.set_hour12(0), Err(OUT_OF_RANGE));
        assert_eq!(p2.set_hour12(13), Err(OUT_OF_RANGE));
    }

    #[test]
    fn set_hour_derives_div_and_mod_12() {
        let mut p = Parsed::new();
        p.set_hour(0).unwrap();
        assert_eq!((p.hour_div_12(), p.hour_mod_12()), (Some(0), Some(0)));

        let mut p2 = Parsed::new();
        p2.set_hour(13).unwrap();
        assert_eq!((p2.hour_div_12(), p2.hour_mod_12()), (Some(1), Some(1)));
        assert_eq!(p2.set_hour(24), Err(OUT_OF_RANGE));
    }

    #[test]
    fn set_second_allows_60_for_leap_second() {
        let mut p = Parsed::new();
        p.set_second(60).unwrap();
        assert_eq!(p.second(), Some(60));
        assert_eq!(p.set_second(61), Err(OUT_OF_RANGE));
    }

    #[test]
    fn to_naive_date_from_year_month_day() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_month(6).unwrap();
        p.set_day(15).unwrap();
        assert_eq!(p.to_naive_date().unwrap(), NaiveDate::from_ymd_opt(2023, 6, 15).unwrap());
    }

    #[test]
    fn to_naive_date_from_year_and_ordinal() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        // Day 166 of 2023: Jan(31)+Feb(28)+Mar(31)+Apr(30)+May(31) = 151, +15 = June 15.
        p.set_ordinal(166).unwrap();
        assert_eq!(p.to_naive_date().unwrap(), NaiveDate::from_ymd_opt(2023, 6, 15).unwrap());
    }

    #[test]
    fn to_naive_date_from_iso_year_week_weekday() {
        // Hand-verified in `calendar.rs`'s own tests: 2023-01-01 is a
        // Sunday, and falls in ISO week 2022-W52.
        let mut p = Parsed::new();
        p.set_isoyear(2022).unwrap();
        p.set_isoweek(52).unwrap();
        p.set_weekday(Weekday::Sun).unwrap();
        assert_eq!(p.to_naive_date().unwrap(), NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
    }

    #[test]
    fn to_naive_date_from_week_from_sun_round_trips() {
        // Self-verifying: derive the fields from a known date, then check
        // that `to_naive_date` reconstructs the same date, regardless of
        // the exact `weeks_from` numbering convention.
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_week_from_sun(date.weeks_from(Weekday::Sun) as i64).unwrap();
        p.set_weekday(date.weekday()).unwrap();
        assert_eq!(p.to_naive_date().unwrap(), date);
    }

    #[test]
    fn to_naive_date_from_week_from_mon_round_trips() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_week_from_mon(date.weeks_from(Weekday::Mon) as i64).unwrap();
        p.set_weekday(date.weekday()).unwrap();
        assert_eq!(p.to_naive_date().unwrap(), date);
    }

    #[test]
    fn to_naive_date_insufficient_fields_is_not_enough() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        assert_eq!(p.to_naive_date(), Err(NOT_ENOUGH));
    }

    #[test]
    fn to_naive_date_rejects_inconsistent_quarter() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_month(6).unwrap(); // June is Q2
        p.set_day(15).unwrap();
        p.set_quarter(1).unwrap(); // claims Q1: inconsistent
        assert_eq!(p.to_naive_date(), Err(IMPOSSIBLE));
    }

    #[test]
    fn to_naive_date_rejects_inconsistent_year_div_mod_100() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_year_div_100(19).unwrap(); // should be 20 for year 2023
        p.set_month(6).unwrap();
        p.set_day(15).unwrap();
        assert_eq!(p.to_naive_date(), Err(IMPOSSIBLE));
    }

    #[test]
    fn to_naive_time_from_hour_minute() {
        let mut p = Parsed::new();
        p.set_hour(9).unwrap();
        p.set_minute(30).unwrap();
        assert_eq!(p.to_naive_time().unwrap(), NaiveTime::from_hms_opt(9, 30, 0).unwrap());
    }

    #[test]
    fn to_naive_time_missing_hour_is_not_enough() {
        let mut p = Parsed::new();
        p.set_minute(30).unwrap();
        assert_eq!(p.to_naive_time(), Err(NOT_ENOUGH));
    }

    #[test]
    fn to_naive_time_leap_second_maps_to_59_with_extra_nanos() {
        let mut p = Parsed::new();
        p.set_hour(23).unwrap();
        p.set_minute(59).unwrap();
        p.set_second(60).unwrap();
        let t = p.to_naive_time().unwrap();
        assert_eq!(t.second(), 59); // `second()` never reports 60.
        assert_eq!(t.nanosecond(), 1_000_000_000);
    }

    #[test]
    fn to_naive_time_nanosecond_without_second_is_not_enough() {
        let mut p = Parsed::new();
        p.set_hour(9).unwrap();
        p.set_minute(30).unwrap();
        p.set_nanosecond(500_000_000).unwrap();
        assert_eq!(p.to_naive_time(), Err(NOT_ENOUGH));
    }

    #[test]
    fn to_naive_datetime_with_offset_from_date_and_time_fields() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_month(6).unwrap();
        p.set_day(15).unwrap();
        p.set_hour(9).unwrap();
        p.set_minute(30).unwrap();
        p.set_second(0).unwrap();
        let dt = p.to_naive_datetime_with_offset(0).unwrap();
        assert_eq!(
            dt,
            NaiveDate::from_ymd_opt(2023, 6, 15).unwrap().and_hms_opt(9, 30, 0).unwrap()
        );
    }

    #[test]
    fn to_naive_datetime_with_offset_reconstructs_from_timestamp_alone() {
        // 1_700_000_000 is the well-known 2023-11-14 22:13:20 UTC instant
        // (hand-verified in `datetime.rs`'s own tests).
        let mut p = Parsed::new();
        p.set_timestamp(1_700_000_000).unwrap();
        let dt = p.to_naive_datetime_with_offset(0).unwrap();
        assert_eq!((dt.year(), dt.month(), dt.day()), (2023, 11, 14));
        assert_eq!((dt.hour(), dt.minute(), dt.second()), (22, 13, 20));
    }

    #[test]
    fn to_naive_datetime_with_offset_checks_timestamp_consistency() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_month(6).unwrap();
        p.set_day(15).unwrap();
        p.set_hour(9).unwrap();
        p.set_minute(30).unwrap();
        p.set_second(0).unwrap();
        p.set_timestamp(0).unwrap(); // inconsistent with the date/time above
        assert_eq!(p.to_naive_datetime_with_offset(0), Err(IMPOSSIBLE));
    }

    #[test]
    fn to_fixed_offset_uses_the_offset_field() {
        let mut p = Parsed::new();
        p.set_offset(3600).unwrap();
        assert_eq!(p.to_fixed_offset().unwrap(), FixedOffset::east_opt(3600).unwrap());
        assert_eq!(Parsed::new().to_fixed_offset(), Err(NOT_ENOUGH));
    }

    #[test]
    fn to_datetime_requires_offset_or_timestamp() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_month(6).unwrap();
        p.set_day(15).unwrap();
        p.set_hour(9).unwrap();
        p.set_minute(30).unwrap();
        p.set_second(0).unwrap();
        assert_eq!(p.to_datetime(), Err(NOT_ENOUGH));
        p.set_offset(0).unwrap();
        let dt = p.to_datetime().unwrap();
        assert_eq!(dt.offset().local_minus_utc(), 0);
    }

    #[test]
    fn to_datetime_defaults_offset_to_utc_when_only_timestamp_given() {
        let mut p = Parsed::new();
        p.set_timestamp(0).unwrap();
        let dt = p.to_datetime().unwrap();
        assert_eq!(dt.offset().local_minus_utc(), 0);
        assert_eq!(dt.timestamp(), 0);
    }

    #[test]
    fn to_datetime_with_timezone_resolves_against_utc() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_month(6).unwrap();
        p.set_day(15).unwrap();
        p.set_hour(9).unwrap();
        p.set_minute(30).unwrap();
        p.set_second(0).unwrap();
        let dt = p.to_datetime_with_timezone(&Utc).unwrap();
        assert_eq!((dt.year(), dt.hour()), (2023, 9));
    }

    #[test]
    fn to_datetime_with_timezone_rejects_inconsistent_offset() {
        let mut p = Parsed::new();
        p.set_year(2023).unwrap();
        p.set_month(6).unwrap();
        p.set_day(15).unwrap();
        p.set_hour(9).unwrap();
        p.set_minute(30).unwrap();
        p.set_second(0).unwrap();
        p.set_offset(3600).unwrap(); // `Utc`'s offset is always 0
        assert_eq!(p.to_datetime_with_timezone(&Utc), Err(IMPOSSIBLE));
    }
}
