//! `NaiveDateTime`: a date and a time of day combined, without an
//! associated time zone.

use crate::datetime::DateTime;
use crate::duration::{Days, Months, TimeDelta};
use crate::format::{
    parse, parse_and_remainder, DelayedFormat, Fixed, Item, Numeric, Pad, ParseError, ParseResult,
    Parsed, StrftimeItems,
};
use crate::naive::date::{IsoWeek, NaiveDate};
use crate::naive::time::NaiveTime;
use crate::offset::{FixedOffset, MappedLocalTime, TimeZone, Utc};
use crate::traits::{Datelike, Timelike};
use crate::weekday::Weekday;
use core::borrow::Borrow;
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// A date and time of day combined (proleptic Gregorian calendar), without
/// an associated time zone.
///
/// API aligned with `chrono::NaiveDateTime`. Build one from a [`NaiveDate`]
/// with [`NaiveDate::and_hms_opt`] and the other `NaiveDate::and_*`
/// constructors, or directly with [`NaiveDateTime::new`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct NaiveDateTime {
    date: NaiveDate,
    time: NaiveTime,
}

#[cfg(feature = "defmt")]
impl defmt::Format for NaiveDateTime {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}T{}", self.date, self.time);
    }
}

/// Deprecated alias for [`NaiveDateTime::MIN`].
#[deprecated(note = "use `NaiveDateTime::MIN` instead")]
pub const MIN_DATETIME: NaiveDateTime = NaiveDateTime::MIN;

/// Deprecated alias for [`NaiveDateTime::MAX`].
#[deprecated(note = "use `NaiveDateTime::MAX` instead")]
pub const MAX_DATETIME: NaiveDateTime = NaiveDateTime::MAX;

impl NaiveDateTime {
    /// The smallest representable `NaiveDateTime`.
    pub const MIN: Self = Self { date: NaiveDate::MIN, time: NaiveTime::MIN };

    /// The largest representable `NaiveDateTime`.
    pub const MAX: Self = Self { date: NaiveDate::MAX, time: NaiveTime::MAX };

    /// The datetime of the Unix epoch, 1970-01-01 00:00:00, *without* a
    /// time zone. The true Unix epoch (with its UTC time zone) is
    /// [`DateTime::UNIX_EPOCH`].
    #[deprecated(note = "use `DateTime::UNIX_EPOCH` instead")]
    pub const UNIX_EPOCH: Self = DateTime::UNIX_EPOCH.naive_utc();

    /// Combines a date and a time into a `NaiveDateTime`. Equivalent to
    /// [`date.and_time(time)`](NaiveDate::and_time).
    pub const fn new(date: NaiveDate, time: NaiveTime) -> NaiveDateTime {
        NaiveDateTime { date, time }
    }

    /// The date component.
    pub const fn date(&self) -> NaiveDate {
        self.date
    }

    /// The time-of-day component.
    pub const fn time(&self) -> NaiveTime {
        self.time
    }

    /// Builds a `NaiveDateTime` from a UNIX timestamp (non-leap seconds
    /// since 1970-01-01 00:00:00 UTC) and a nanosecond remainder, like
    /// [`and_utc`](Self::and_utc)`(`[`DateTime::from_timestamp`]`(secs, nsecs)).naive_utc()`.
    ///
    /// # Panics
    /// Panics on out-of-range input.
    #[deprecated(note = "use `DateTime::from_timestamp` instead")]
    pub const fn from_timestamp(secs: i64, nsecs: u32) -> NaiveDateTime {
        match DateTime::from_timestamp(secs, nsecs) {
            Some(dt) => dt.naive_utc(),
            None => panic!("invalid or out-of-range datetime"),
        }
    }

    /// Returns the number of non-leap seconds since the midnight of
    /// January 1, 1970. Does *not* account for the time zone: this is not
    /// a true UNIX timestamp unless `self` is already known to be in UTC.
    #[deprecated(note = "use `.and_utc().timestamp()` instead")]
    pub const fn timestamp(&self) -> i64 {
        self.and_utc().timestamp()
    }

    /// Builds a `NaiveDateTime` from a UNIX timestamp in milliseconds.
    /// Returns `None` if out of range.
    #[deprecated(note = "use `DateTime::from_timestamp_millis` instead")]
    pub const fn from_timestamp_millis(millis: i64) -> Option<NaiveDateTime> {
        match DateTime::from_timestamp_millis(millis) {
            Some(dt) => Some(dt.naive_utc()),
            None => None,
        }
    }

    /// Non-leap milliseconds since the UNIX epoch. Does *not* account for
    /// the time zone.
    #[deprecated(note = "use `.and_utc().timestamp_millis()` instead")]
    pub const fn timestamp_millis(&self) -> i64 {
        self.and_utc().timestamp_millis()
    }

    /// Builds a `NaiveDateTime` from a UNIX timestamp in microseconds.
    /// Returns `None` if out of range.
    #[deprecated(note = "use `DateTime::from_timestamp_micros` instead")]
    pub const fn from_timestamp_micros(micros: i64) -> Option<NaiveDateTime> {
        match DateTime::from_timestamp_micros(micros) {
            Some(dt) => Some(dt.naive_utc()),
            None => None,
        }
    }

    /// Non-leap microseconds since the UNIX epoch. Does *not* account for
    /// the time zone.
    #[deprecated(note = "use `.and_utc().timestamp_micros()` instead")]
    pub const fn timestamp_micros(&self) -> i64 {
        self.and_utc().timestamp_micros()
    }

    /// Builds a `NaiveDateTime` from a UNIX timestamp in nanoseconds.
    #[deprecated(note = "use `DateTime::from_timestamp_nanos` instead")]
    pub const fn from_timestamp_nanos(nanos: i64) -> NaiveDateTime {
        DateTime::from_timestamp_nanos(nanos).naive_utc()
    }

    /// Non-leap nanoseconds since the UNIX epoch. Does *not* account for
    /// the time zone.
    ///
    /// # Panics
    /// Panics if out of range; see
    /// [`timestamp_nanos_opt`](Self::timestamp_nanos_opt) for a
    /// non-panicking version.
    #[deprecated(note = "use `.and_utc().timestamp_nanos_opt()` instead")]
    #[allow(deprecated)]
    pub const fn timestamp_nanos(&self) -> i64 {
        self.and_utc().timestamp_nanos()
    }

    /// Non-leap nanoseconds since the UNIX epoch, or `None` if out of
    /// range. Does *not* account for the time zone.
    #[deprecated(note = "use `.and_utc().timestamp_nanos_opt()` instead")]
    pub const fn timestamp_nanos_opt(&self) -> Option<i64> {
        self.and_utc().timestamp_nanos_opt()
    }

    /// Builds a `NaiveDateTime` from a UNIX timestamp (seconds + nanosecond
    /// remainder), like [`from_timestamp`](Self::from_timestamp) but
    /// non-panicking.
    #[deprecated(note = "use `DateTime::from_timestamp` instead")]
    pub const fn from_timestamp_opt(secs: i64, nsecs: u32) -> Option<NaiveDateTime> {
        match DateTime::from_timestamp(secs, nsecs) {
            Some(dt) => Some(dt.naive_utc()),
            None => None,
        }
    }

    /// Milliseconds since the last whole non-leap second (0..=999, or up
    /// to 1,999 for a leap second).
    #[deprecated(note = "use `.and_utc().timestamp_subsec_millis()` instead")]
    pub const fn timestamp_subsec_millis(&self) -> u32 {
        self.and_utc().timestamp_subsec_millis()
    }

    /// Microseconds since the last whole non-leap second.
    #[deprecated(note = "use `.and_utc().timestamp_subsec_micros()` instead")]
    pub const fn timestamp_subsec_micros(&self) -> u32 {
        self.and_utc().timestamp_subsec_micros()
    }

    /// Nanoseconds since the last whole non-leap second.
    #[deprecated(note = "use `.and_utc().timestamp_subsec_nanos()` instead")]
    pub const fn timestamp_subsec_nanos(&self) -> u32 {
        self.and_utc().timestamp_subsec_nanos()
    }

    /// Adds a signed [`TimeDelta`], wrapping into the next/previous day(s)
    /// as needed. Returns `None` if the resulting date would be out of
    /// range.
    ///
    /// As part of the [leap second handling](NaiveTime#leap-second-handling),
    /// the addition assumes that **there is no leap second ever**, except
    /// when `self` itself represents a leap second, in which case the
    /// assumption becomes that **there is exactly a single leap second
    /// ever**.
    pub const fn checked_add_signed(self, rhs: TimeDelta) -> Option<NaiveDateTime> {
        let (time, remainder) = self.time.overflowing_add_signed(rhs);
        let remainder = match TimeDelta::try_seconds(remainder) {
            Some(d) => d,
            None => return None,
        };
        let date = match self.date.checked_add_signed(remainder) {
            Some(d) => d,
            None => return None,
        };
        Some(NaiveDateTime { date, time })
    }

    /// Subtracts a signed [`TimeDelta`]. See
    /// [`checked_add_signed`](Self::checked_add_signed).
    pub const fn checked_sub_signed(self, rhs: TimeDelta) -> Option<NaiveDateTime> {
        let (time, remainder) = self.time.overflowing_sub_signed(rhs);
        let remainder = match TimeDelta::try_seconds(remainder) {
            Some(d) => d,
            None => return None,
        };
        let date = match self.date.checked_sub_signed(remainder) {
            Some(d) => d,
            None => return None,
        };
        Some(NaiveDateTime { date, time })
    }

    /// Adds a number of [`Months`] to the date part, keeping the time of
    /// day unchanged. Uses the last day of the month if the day does not
    /// exist in the resulting month. Returns `None` if the resulting date
    /// would be out of range.
    pub const fn checked_add_months(self, rhs: Months) -> Option<NaiveDateTime> {
        match self.date.checked_add_months(rhs) {
            Some(date) => Some(NaiveDateTime { date, time: self.time }),
            None => None,
        }
    }

    /// Subtracts a number of [`Months`] from the date part, like
    /// [`checked_add_months`](Self::checked_add_months).
    pub const fn checked_sub_months(self, rhs: Months) -> Option<NaiveDateTime> {
        match self.date.checked_sub_months(rhs) {
            Some(date) => Some(NaiveDateTime { date, time: self.time }),
            None => None,
        }
    }

    /// Adds a number of [`Days`] to the date part. Returns `None` if the
    /// resulting date would be out of range.
    pub const fn checked_add_days(self, days: Days) -> Option<Self> {
        match self.date.checked_add_days(days) {
            Some(date) => Some(Self { date, time: self.time }),
            None => None,
        }
    }

    /// Subtracts a number of [`Days`] from the date part. Returns `None`
    /// if the resulting date would be out of range.
    pub const fn checked_sub_days(self, days: Days) -> Option<Self> {
        match self.date.checked_sub_days(days) {
            Some(date) => Some(Self { date, time: self.time }),
            None => None,
        }
    }

    /// Signed duration between two datetimes (`self - rhs`). Never
    /// overflows or underflows.
    ///
    /// As part of the [leap second handling](NaiveTime#leap-second-handling),
    /// the subtraction assumes that **there is no leap second ever**,
    /// except when either operand represents a leap second, in which case
    /// the assumption becomes that **there are exactly one (or two) leap
    /// second(s) ever**.
    pub const fn signed_duration_since(self, rhs: NaiveDateTime) -> TimeDelta {
        let date_part = self.date.signed_duration_since(rhs.date);
        let time_part = self.time.signed_duration_since(rhs.time);
        match date_part.checked_add(&time_part) {
            Some(d) => d,
            None => panic!("NaiveDateTime::signed_duration_since: result out of range"),
        }
    }

    /// Adds a given [`FixedOffset`] to the current datetime. Returns
    /// `None` if the result would be outside the valid range for
    /// `NaiveDateTime`. Preserves leap seconds, unlike
    /// [`checked_add_signed`](Self::checked_add_signed).
    pub const fn checked_add_offset(self, rhs: FixedOffset) -> Option<NaiveDateTime> {
        let (time, days) = self.time.overflowing_add_offset(rhs);
        let date = match days {
            -1 => match self.date.pred_opt() {
                Some(d) => d,
                None => return None,
            },
            1 => match self.date.succ_opt() {
                Some(d) => d,
                None => return None,
            },
            _ => self.date,
        };
        Some(NaiveDateTime { date, time })
    }

    /// Subtracts a given [`FixedOffset`] from the current datetime.
    /// See [`checked_add_offset`](Self::checked_add_offset).
    pub const fn checked_sub_offset(self, rhs: FixedOffset) -> Option<NaiveDateTime> {
        let (time, days) = self.time.overflowing_sub_offset(rhs);
        let date = match days {
            -1 => match self.date.pred_opt() {
                Some(d) => d,
                None => return None,
            },
            1 => match self.date.succ_opt() {
                Some(d) => d,
                None => return None,
            },
            _ => self.date,
        };
        Some(NaiveDateTime { date, time })
    }

    /// Adds a given [`FixedOffset`] to the current datetime. The result
    /// may fall (slightly) outside the normal representable range of
    /// `NaiveDateTime`, using the buffer space around
    /// [`NaiveDate::MIN`]/[`NaiveDate::MAX`]. Intended as an internal
    /// intermediate value; never exposed to users of this crate.
    pub(crate) const fn overflowing_add_offset(self, rhs: FixedOffset) -> NaiveDateTime {
        let (time, days) = self.time.overflowing_add_offset(rhs);
        let date = match days {
            -1 => match self.date.pred_opt() {
                Some(d) => d,
                None => NaiveDate::BEFORE_MIN,
            },
            1 => match self.date.succ_opt() {
                Some(d) => d,
                None => NaiveDate::AFTER_MAX,
            },
            _ => self.date,
        };
        NaiveDateTime { date, time }
    }

    /// Subtracts a given [`FixedOffset`] from the current datetime. See
    /// [`overflowing_add_offset`](Self::overflowing_add_offset).
    #[allow(unused)] // currently only used by `Local` on some platforms, exactly like in chrono
    pub(crate) const fn overflowing_sub_offset(self, rhs: FixedOffset) -> NaiveDateTime {
        let (time, days) = self.time.overflowing_sub_offset(rhs);
        let date = match days {
            -1 => match self.date.pred_opt() {
                Some(d) => d,
                None => NaiveDate::BEFORE_MIN,
            },
            1 => match self.date.succ_opt() {
                Some(d) => d,
                None => NaiveDate::AFTER_MAX,
            },
            _ => self.date,
        };
        NaiveDateTime { date, time }
    }

    /// Converts this `NaiveDateTime` into a timezone-aware `DateTime<Tz>`
    /// with the given time zone.
    pub fn and_local_timezone<Tz: TimeZone>(&self, tz: Tz) -> MappedLocalTime<DateTime<Tz>> {
        tz.from_local_datetime(self)
    }

    /// Converts this `NaiveDateTime` into a timezone-aware `DateTime<Utc>`.
    pub const fn and_utc(&self) -> DateTime<Utc> {
        DateTime::from_naive_utc_and_offset(*self, Utc)
    }

    /// Parses a `NaiveDateTime` from a string using a user-specified
    /// format. See the [`crate::format::strftime`] module for the
    /// supported escape sequences.
    pub fn parse_from_str(s: &str, fmt: &str) -> ParseResult<NaiveDateTime> {
        let mut parsed = Parsed::new();
        parse(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_naive_datetime_with_offset(0) // no offset adjustment
    }

    /// Parses a `NaiveDateTime` from a string using a user-specified
    /// format, returning the value and a slice with the remaining,
    /// unparsed portion of the string.
    pub fn parse_and_remainder<'a>(s: &'a str, fmt: &str) -> ParseResult<(NaiveDateTime, &'a str)> {
        let mut parsed = Parsed::new();
        let remainder = parse_and_remainder(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_naive_datetime_with_offset(0).map(|d| (d, remainder))
    }

    /// Formats the combined date and time with the specified formatting items.
    #[must_use]
    pub fn format_with_items<'a, I, B>(&self, items: I) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        DelayedFormat::new(Some(self.date), Some(self.time), items)
    }

    /// Formats the combined date and time with the specified format
    /// string. See the [`crate::format::strftime`] module for the
    /// supported escape sequences.
    #[must_use]
    pub fn format<'a>(&self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'a>> {
        self.format_with_items(StrftimeItems::new(fmt))
    }
}

impl From<NaiveDate> for NaiveDateTime {
    /// Converts a `NaiveDate` to a `NaiveDateTime` of the same date, at
    /// midnight.
    fn from(date: NaiveDate) -> Self {
        date.and_hms_opt(0, 0, 0).unwrap()
    }
}

impl Datelike for NaiveDateTime {
    fn year(&self) -> i32 {
        self.date.year()
    }

    fn month(&self) -> u32 {
        self.date.month()
    }

    fn month0(&self) -> u32 {
        self.date.month0()
    }

    fn day(&self) -> u32 {
        self.date.day()
    }

    fn day0(&self) -> u32 {
        self.date.day0()
    }

    fn ordinal(&self) -> u32 {
        self.date.ordinal()
    }

    fn ordinal0(&self) -> u32 {
        self.date.ordinal0()
    }

    fn weekday(&self) -> Weekday {
        self.date.weekday()
    }

    fn iso_week(&self) -> IsoWeek {
        self.date.iso_week()
    }

    fn with_year(&self, year: i32) -> Option<NaiveDateTime> {
        self.date
            .with_year(year)
            .map(|d| NaiveDateTime { date: d, ..*self })
    }

    fn with_month(&self, month: u32) -> Option<NaiveDateTime> {
        self.date
            .with_month(month)
            .map(|d| NaiveDateTime { date: d, ..*self })
    }

    fn with_month0(&self, month0: u32) -> Option<NaiveDateTime> {
        self.date
            .with_month0(month0)
            .map(|d| NaiveDateTime { date: d, ..*self })
    }

    fn with_day(&self, day: u32) -> Option<NaiveDateTime> {
        self.date
            .with_day(day)
            .map(|d| NaiveDateTime { date: d, ..*self })
    }

    fn with_day0(&self, day0: u32) -> Option<NaiveDateTime> {
        self.date
            .with_day0(day0)
            .map(|d| NaiveDateTime { date: d, ..*self })
    }

    fn with_ordinal(&self, ordinal: u32) -> Option<NaiveDateTime> {
        self.date
            .with_ordinal(ordinal)
            .map(|d| NaiveDateTime { date: d, ..*self })
    }

    fn with_ordinal0(&self, ordinal0: u32) -> Option<NaiveDateTime> {
        self.date
            .with_ordinal0(ordinal0)
            .map(|d| NaiveDateTime { date: d, ..*self })
    }
}

impl Timelike for NaiveDateTime {
    fn hour(&self) -> u32 {
        self.time.hour()
    }

    fn minute(&self) -> u32 {
        self.time.minute()
    }

    fn second(&self) -> u32 {
        self.time.second()
    }

    fn nanosecond(&self) -> u32 {
        self.time.nanosecond()
    }

    fn with_hour(&self, hour: u32) -> Option<NaiveDateTime> {
        self.time
            .with_hour(hour)
            .map(|t| NaiveDateTime { time: t, ..*self })
    }

    fn with_minute(&self, min: u32) -> Option<NaiveDateTime> {
        self.time
            .with_minute(min)
            .map(|t| NaiveDateTime { time: t, ..*self })
    }

    fn with_second(&self, sec: u32) -> Option<NaiveDateTime> {
        self.time
            .with_second(sec)
            .map(|t| NaiveDateTime { time: t, ..*self })
    }

    fn with_nanosecond(&self, nano: u32) -> Option<NaiveDateTime> {
        self.time
            .with_nanosecond(nano)
            .map(|t| NaiveDateTime { time: t, ..*self })
    }
}

/// Adds a `TimeDelta` to a `NaiveDateTime`.
///
/// # Panics
/// Panics if the resulting date would be out of range. Consider using
/// [`NaiveDateTime::checked_add_signed`] to get an `Option` instead.
impl Add<TimeDelta> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn add(self, rhs: TimeDelta) -> NaiveDateTime {
        self.checked_add_signed(rhs)
            .expect("`NaiveDateTime + TimeDelta` overflowed")
    }
}

/// Adds a `core::time::Duration` (unsigned) to a `NaiveDateTime`.
///
/// # Panics
/// Panics if `rhs` does not fit in a [`TimeDelta`], or if the resulting
/// date would be out of range.
impl Add<core::time::Duration> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn add(self, rhs: core::time::Duration) -> NaiveDateTime {
        let rhs = TimeDelta::from_std(rhs)
            .expect("overflow converting from core::time::Duration to TimeDelta");
        self.checked_add_signed(rhs)
            .expect("`NaiveDateTime + TimeDelta` overflowed")
    }
}

impl AddAssign<TimeDelta> for NaiveDateTime {
    fn add_assign(&mut self, rhs: TimeDelta) {
        *self = *self + rhs;
    }
}

impl AddAssign<core::time::Duration> for NaiveDateTime {
    fn add_assign(&mut self, rhs: core::time::Duration) {
        *self = *self + rhs;
    }
}

/// Adds a [`FixedOffset`] to the current datetime (offset only, the
/// timezone-awareness itself is not tracked by `NaiveDateTime`).
///
/// # Panics
/// Panics if the resulting date would be out of range.
impl Add<FixedOffset> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn add(self, rhs: FixedOffset) -> NaiveDateTime {
        self.checked_add_offset(rhs)
            .expect("`NaiveDateTime + FixedOffset` overflowed")
    }
}

/// Adds [`Months`] to a `NaiveDateTime`. The result is clamped to the last
/// valid day of the resulting month; see
/// [`NaiveDateTime::checked_add_months`].
///
/// # Panics
/// Panics if the resulting date would be out of range.
impl Add<Months> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn add(self, rhs: Months) -> NaiveDateTime {
        self.checked_add_months(rhs)
            .expect("`NaiveDateTime + Months` out of range")
    }
}

/// Subtracts a `TimeDelta` from a `NaiveDateTime`.
///
/// # Panics
/// Panics if the resulting date would be out of range. Consider using
/// [`NaiveDateTime::checked_sub_signed`] to get an `Option` instead.
impl Sub<TimeDelta> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn sub(self, rhs: TimeDelta) -> NaiveDateTime {
        self.checked_sub_signed(rhs)
            .expect("`NaiveDateTime - TimeDelta` overflowed")
    }
}

/// Subtracts a `core::time::Duration` (unsigned) from a `NaiveDateTime`.
///
/// # Panics
/// Panics if `rhs` does not fit in a [`TimeDelta`], or if the resulting
/// date would be out of range.
impl Sub<core::time::Duration> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn sub(self, rhs: core::time::Duration) -> NaiveDateTime {
        let rhs = TimeDelta::from_std(rhs)
            .expect("overflow converting from core::time::Duration to TimeDelta");
        self.checked_sub_signed(rhs)
            .expect("`NaiveDateTime - TimeDelta` overflowed")
    }
}

impl SubAssign<TimeDelta> for NaiveDateTime {
    fn sub_assign(&mut self, rhs: TimeDelta) {
        *self = *self - rhs;
    }
}

impl SubAssign<core::time::Duration> for NaiveDateTime {
    fn sub_assign(&mut self, rhs: core::time::Duration) {
        *self = *self - rhs;
    }
}

/// Subtracts a [`FixedOffset`] from the current datetime.
///
/// # Panics
/// Panics if the resulting date would be out of range.
impl Sub<FixedOffset> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn sub(self, rhs: FixedOffset) -> NaiveDateTime {
        self.checked_sub_offset(rhs)
            .expect("`NaiveDateTime - FixedOffset` overflowed")
    }
}

/// Subtracts [`Months`] from a `NaiveDateTime`. The result is clamped to
/// the last valid day of the resulting month; see
/// [`NaiveDateTime::checked_sub_months`].
///
/// # Panics
/// Panics if the resulting date would be out of range.
impl Sub<Months> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn sub(self, rhs: Months) -> NaiveDateTime {
        self.checked_sub_months(rhs)
            .expect("`NaiveDateTime - Months` out of range")
    }
}

/// Subtracts another `NaiveDateTime`, yielding the signed [`TimeDelta`]
/// between the two. Wraps [`NaiveDateTime::signed_duration_since`].
impl Sub<NaiveDateTime> for NaiveDateTime {
    type Output = TimeDelta;
    fn sub(self, rhs: NaiveDateTime) -> TimeDelta {
        self.signed_duration_since(rhs)
    }
}

/// Adds [`Days`] to a `NaiveDateTime`.
///
/// # Panics
/// Panics if the resulting date would be out of range. Consider using
/// [`NaiveDateTime::checked_add_days`] to get an `Option` instead.
impl Add<Days> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn add(self, days: Days) -> NaiveDateTime {
        self.checked_add_days(days)
            .expect("`NaiveDateTime + Days` out of range")
    }
}

/// Subtracts [`Days`] from a `NaiveDateTime`.
///
/// # Panics
/// Panics if the resulting date would be out of range. Consider using
/// [`NaiveDateTime::checked_sub_days`] to get an `Option` instead.
impl Sub<Days> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn sub(self, days: Days) -> NaiveDateTime {
        self.checked_sub_days(days)
            .expect("`NaiveDateTime - Days` out of range")
    }
}

/// The `Debug` output is the same as `dt.format("%Y-%m-%dT%H:%M:%S%.f")`.
impl fmt::Debug for NaiveDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.date, f)?;
        f.write_str("T")?;
        fmt::Debug::fmt(&self.time, f)
    }
}

/// The `Display` output is the same as `dt.format("%Y-%m-%d %H:%M:%S%.f")`.
impl fmt::Display for NaiveDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.date, f)?;
        f.write_str(" ")?;
        fmt::Display::fmt(&self.time, f)
    }
}

impl Default for NaiveDateTime {
    /// Defaults to 1970-01-01 00:00:00 (the Unix epoch, without a time
    /// zone), like `chrono`.
    fn default() -> Self {
        NaiveDateTime {
            date: NaiveDate::default(),
            time: NaiveTime::default(),
        }
    }
}

/// Parsing a `str` into a `NaiveDateTime` uses the format `%Y-%m-%dT%H:%M:%S%.f`.
impl core::str::FromStr for NaiveDateTime {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<NaiveDateTime> {
        const ITEMS: &[Item<'static>] = &[
            Item::Numeric(Numeric::Year, Pad::Zero),
            Item::Space(""),
            Item::Literal("-"),
            Item::Numeric(Numeric::Month, Pad::Zero),
            Item::Space(""),
            Item::Literal("-"),
            Item::Numeric(Numeric::Day, Pad::Zero),
            Item::Space(""),
            Item::Literal("T"),
            Item::Numeric(Numeric::Hour, Pad::Zero),
            Item::Space(""),
            Item::Literal(":"),
            Item::Numeric(Numeric::Minute, Pad::Zero),
            Item::Space(""),
            Item::Literal(":"),
            Item::Numeric(Numeric::Second, Pad::Zero),
            Item::Fixed(Fixed::Nanosecond),
            Item::Space(""),
        ];

        let mut parsed = Parsed::new();
        parse(&mut parsed, s, ITEMS.iter())?;
        parsed.to_naive_datetime_with_offset(0)
    }
}

/// `serde` support.
///
/// The default (de)serialization is the ISO 8601 string (`Debug`-style,
/// "T"-separated -- the same format accepted by `FromStr`; note this
/// differs from the space-separated `Display` format). This module also
/// exposes `ts_seconds`, `ts_milliseconds`,
/// `ts_microseconds` and `ts_nanoseconds` (and their `_option` variants),
/// intended for use with serde's `#[serde(with = "...")]` field attribute
/// to (de)serialize as a Unix timestamp instead.
#[cfg(feature = "serde")]
pub(crate) mod serde {
    use core::fmt;
    use serde::{de, ser};

    use super::NaiveDateTime;

    impl ser::Serialize for NaiveDateTime {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            // Serialized via `Debug` (ISO 8601, "T"-separated), not
            // `Display` (space-separated): `FromStr`/`Deserialize` below
            // only accept the "T"-separated form, so serializing via
            // `Display` would produce a string `Deserialize` cannot parse
            // back. Found via the differential-testing harness
            // (2026-07-13): this broke every single `NaiveDateTime` serde
            // round-trip. Matches chrono's own approach of serializing via
            // `Debug` rather than `Display` for this exact reason.
            struct FormatWrapped<'a>(&'a NaiveDateTime);
            impl fmt::Display for FormatWrapped<'_> {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Debug::fmt(self.0, f)
                }
            }
            serializer.collect_str(&FormatWrapped(self))
        }
    }

    struct NaiveDateTimeVisitor;

    impl de::Visitor<'_> for NaiveDateTimeVisitor {
        type Value = NaiveDateTime;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a formatted date and time string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(E::custom)
        }
    }

    impl<'de> de::Deserialize<'de> for NaiveDateTime {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(NaiveDateTimeVisitor)
        }
    }

    /// Builds a `de::Error` for a timestamp value that could not be
    /// converted back to a `NaiveDateTime`. Shared by every `ts_*`
    /// submodule below.
    pub(crate) fn invalid_ts<E: de::Error, T: fmt::Display>(value: T) -> E {
        E::custom(format_args!("value is not a legal timestamp: {value}"))
    }

    /// Generates a `$unit`/`$unit_option` pair of serde helper modules,
    /// (de)serializing a [`NaiveDateTime`] as a Unix timestamp counted in
    /// a fixed sub-second unit. `$units_per_sec` is how many `$unit`s make
    /// up one second (1 for seconds, 1_000 for milliseconds, and so on);
    /// `$get_value` extracts the raw `i64` count from a `&NaiveDateTime`.
    macro_rules! ts_unit {
        ($unit:ident, $unit_option:ident, $units_per_sec:expr, $get_value:expr, $expecting:literal) => {
            #[doc = concat!("(De)serialize a `NaiveDateTime` as a Unix timestamp in ", $expecting, ".")]
            pub mod $unit {
                use core::fmt;
                use serde::{de, ser};

                use super::super::NaiveDateTime;
                use super::invalid_ts;
                use crate::DateTime;

                const UNITS_PER_SEC: i64 = $units_per_sec;
                const NANOS_PER_UNIT: i64 = 1_000_000_000 / UNITS_PER_SEC;

                /// Serializes into an integer timestamp. Intended for use
                /// with serde's `serialize_with` attribute.
                pub fn serialize<S>(dt: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ser::Serializer,
                {
                    let get_value: fn(&NaiveDateTime) -> Result<i64, &'static str> = $get_value;
                    serializer.serialize_i64(get_value(dt).map_err(ser::Error::custom)?)
                }

                /// Deserializes from an integer timestamp. Intended for use
                /// with serde's `deserialize_with` attribute.
                pub fn deserialize<'de, D>(d: D) -> Result<NaiveDateTime, D::Error>
                where
                    D: de::Deserializer<'de>,
                {
                    d.deserialize_i64(TsVisitor)
                }

                pub(super) struct TsVisitor;

                impl de::Visitor<'_> for TsVisitor {
                    type Value = NaiveDateTime;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a unix timestamp")
                    }

                    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                        let secs = value.div_euclid(UNITS_PER_SEC);
                        let nsecs = (value.rem_euclid(UNITS_PER_SEC) * NANOS_PER_UNIT) as u32;
                        DateTime::from_timestamp(secs, nsecs)
                            .map(|dt| dt.naive_utc())
                            .ok_or_else(|| invalid_ts(value))
                    }

                    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                        let units_per_sec = UNITS_PER_SEC as u64;
                        let secs = (value / units_per_sec) as i64;
                        let nsecs = ((value % units_per_sec) * NANOS_PER_UNIT as u64) as u32;
                        DateTime::from_timestamp(secs, nsecs)
                            .map(|dt| dt.naive_utc())
                            .ok_or_else(|| invalid_ts(value))
                    }
                }
            }

            #[doc = concat!("(De)serialize an `Option<NaiveDateTime>` as a Unix timestamp in ", $expecting, ", or `None`.")]
            pub mod $unit_option {
                use core::fmt;
                use serde::{de, ser};

                use super::super::NaiveDateTime;
                use super::$unit::TsVisitor;

                /// Serializes into an integer timestamp, or `null`.
                pub fn serialize<S>(
                    opt: &Option<NaiveDateTime>,
                    serializer: S,
                ) -> Result<S::Ok, S::Error>
                where
                    S: ser::Serializer,
                {
                    let get_value: fn(&NaiveDateTime) -> Result<i64, &'static str> = $get_value;
                    match opt {
                        Some(dt) => {
                            serializer.serialize_some(&get_value(dt).map_err(ser::Error::custom)?)
                        }
                        None => serializer.serialize_none(),
                    }
                }

                fn expecting_str() -> &'static str {
                    concat!("a unix timestamp in ", $expecting, " or none")
                }

                struct OptionTsVisitor;

                impl<'de> de::Visitor<'de> for OptionTsVisitor {
                    type Value = Option<NaiveDateTime>;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str(expecting_str())
                    }

                    fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
                    where
                        D: de::Deserializer<'de>,
                    {
                        d.deserialize_i64(TsVisitor).map(Some)
                    }

                    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                        Ok(None)
                    }

                    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                        Ok(None)
                    }
                }

                /// Deserializes from an integer timestamp or `null`.
                /// Intended for use with serde's `deserialize_with`
                /// attribute.
                pub fn deserialize<'de, D>(d: D) -> Result<Option<NaiveDateTime>, D::Error>
                where
                    D: de::Deserializer<'de>,
                {
                    d.deserialize_option(OptionTsVisitor)
                }
            }
        };
    }

    ts_unit!(
        ts_seconds,
        ts_seconds_option,
        1,
        |dt| Ok(dt.and_utc().timestamp()),
        "seconds"
    );
    ts_unit!(
        ts_milliseconds,
        ts_milliseconds_option,
        1_000,
        |dt| Ok(dt.and_utc().timestamp_millis()),
        "milliseconds"
    );
    ts_unit!(
        ts_microseconds,
        ts_microseconds_option,
        1_000_000,
        |dt| Ok(dt.and_utc().timestamp_micros()),
        "microseconds"
    );
    ts_unit!(
        ts_nanoseconds,
        ts_nanoseconds_option,
        1_000_000_000,
        |dt| dt
            .and_utc()
            .timestamp_nanos_opt()
            .ok_or("value out of range for a timestamp with nanosecond precision"),
        "nanoseconds"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(h, mi, s).unwrap()
    }

    #[test]
    fn new_date_and_time_accessors_round_trip() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let time = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
        let d = NaiveDateTime::new(date, time);
        assert_eq!(d.date(), date);
        assert_eq!(d.time(), time);
        assert_eq!(date.and_time(time), d);
    }

    #[test]
    fn from_naivedate_gives_midnight() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let converted: NaiveDateTime = date.into();
        assert_eq!(converted.date(), date);
        assert_eq!(converted.time(), NaiveTime::MIN);
    }

    #[test]
    fn checked_add_signed_rolls_the_date_forward() {
        let d = dt(2023, 6, 15, 23, 0, 0);
        let result = d.checked_add_signed(TimeDelta::hours(2)).unwrap();
        assert_eq!(result, dt(2023, 6, 16, 1, 0, 0));
    }

    #[test]
    fn checked_sub_signed_rolls_the_date_backward() {
        let d = dt(2023, 6, 15, 1, 0, 0);
        let result = d.checked_sub_signed(TimeDelta::hours(2)).unwrap();
        assert_eq!(result, dt(2023, 6, 14, 23, 0, 0));
    }

    #[test]
    fn checked_add_signed_returns_none_past_max() {
        assert_eq!(NaiveDateTime::MAX.checked_add_signed(TimeDelta::nanoseconds(1)), None);
    }

    #[test]
    fn checked_sub_signed_returns_none_before_min() {
        assert_eq!(NaiveDateTime::MIN.checked_sub_signed(TimeDelta::nanoseconds(1)), None);
    }

    #[test]
    fn checked_add_sub_months_and_days_preserve_time_of_day() {
        let d = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap().and_hms_opt(12, 30, 0).unwrap();
        let plus_month = d.checked_add_months(Months::new(1)).unwrap();
        assert_eq!(plus_month.date(), NaiveDate::from_ymd_opt(2023, 2, 28).unwrap());
        assert_eq!(plus_month.time(), d.time());

        let plus_day = d.checked_add_days(Days::new(1)).unwrap();
        assert_eq!(plus_day.date(), NaiveDate::from_ymd_opt(2023, 2, 1).unwrap());
        assert_eq!(plus_day.time(), d.time());

        let minus_day = plus_day.checked_sub_days(Days::new(1)).unwrap();
        assert_eq!(minus_day, d);
    }

    #[test]
    fn signed_duration_since_basic() {
        let a = dt(2023, 6, 16, 1, 0, 0);
        let b = dt(2023, 6, 15, 23, 0, 0);
        assert_eq!(a.signed_duration_since(b), TimeDelta::hours(2));
        assert_eq!(b.signed_duration_since(a), TimeDelta::hours(-2));
    }

    #[test]
    fn signed_duration_since_accounts_for_leap_second() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let a = date.and_time(NaiveTime::from_hms_milli_opt(23, 59, 59, 1500).unwrap());
        let b = date.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap());
        assert_eq!(a.signed_duration_since(b), TimeDelta::milliseconds(1500));
    }

    #[test]
    fn checked_add_offset_rolls_date_forward() {
        let d = dt(2023, 6, 15, 23, 0, 0);
        let offset = FixedOffset::east_opt(2 * 3600).unwrap();
        let result = d.checked_add_offset(offset).unwrap();
        assert_eq!(result, dt(2023, 6, 16, 1, 0, 0));
    }

    #[test]
    fn checked_sub_offset_rolls_date_backward() {
        let d = dt(2023, 6, 15, 1, 0, 0);
        let offset = FixedOffset::east_opt(2 * 3600).unwrap();
        let result = d.checked_sub_offset(offset).unwrap();
        assert_eq!(result, dt(2023, 6, 14, 23, 0, 0));
    }

    #[test]
    fn checked_add_offset_returns_none_past_max_date() {
        let offset = FixedOffset::east_opt(3600).unwrap();
        assert_eq!(NaiveDateTime::MAX.checked_add_offset(offset), None);
    }

    #[test]
    fn and_utc_preserves_the_same_wall_clock_reading() {
        let d = dt(2023, 6, 15, 9, 30, 0);
        assert_eq!(d.and_utc().naive_utc(), d);
    }

    #[test]
    fn datelike_and_timelike_delegate_to_components() {
        let d = dt(2023, 6, 15, 9, 30, 45);
        assert_eq!(d.year(), 2023);
        assert_eq!(d.month(), 6);
        assert_eq!(d.day(), 15);
        assert_eq!(d.hour(), 9);
        assert_eq!(d.minute(), 30);
        assert_eq!(d.second(), 45);
    }

    #[test]
    fn with_year_and_with_hour_preserve_the_other_component() {
        let d = dt(2023, 6, 15, 9, 30, 45);
        let changed_year = d.with_year(2024).unwrap();
        assert_eq!(changed_year.year(), 2024);
        assert_eq!(changed_year.time(), d.time());

        let changed_hour = d.with_hour(15).unwrap();
        assert_eq!(changed_hour.hour(), 15);
        assert_eq!(changed_hour.date(), d.date());
    }

    #[test]
    fn add_sub_timedelta_operators() {
        let d = dt(2023, 6, 15, 23, 0, 0);
        let result = d + TimeDelta::hours(2);
        assert_eq!(result, dt(2023, 6, 16, 1, 0, 0));
        assert_eq!(result - TimeDelta::hours(2), d);
    }

    #[test]
    #[should_panic]
    fn add_timedelta_operator_panics_on_overflow() {
        let _ = NaiveDateTime::MAX + TimeDelta::nanoseconds(1);
    }

    #[test]
    fn add_sub_days_and_months_operators() {
        let d = dt(2023, 1, 31, 12, 0, 0);
        assert_eq!((d + Days::new(1)).date(), NaiveDate::from_ymd_opt(2023, 2, 1).unwrap());
        assert_eq!((d + Months::new(1)).date(), NaiveDate::from_ymd_opt(2023, 2, 28).unwrap());
    }

    #[test]
    fn sub_naivedatetime_operator_returns_timedelta() {
        let a = dt(2023, 6, 16, 1, 0, 0);
        let b = dt(2023, 6, 15, 23, 0, 0);
        assert_eq!(a - b, TimeDelta::hours(2));
    }

    #[test]
    fn default_is_unix_epoch_midnight() {
        let d = NaiveDateTime::default();
        assert_eq!(d.date(), NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
        assert_eq!(d.time(), NaiveTime::MIN);
    }

    #[test]
    fn debug_uses_t_separator_display_uses_space() {
        let d = dt(2023, 6, 15, 9, 30, 0);
        assert_eq!(format!("{d:?}"), "2023-06-15T09:30:00");
        assert_eq!(d.to_string(), "2023-06-15 09:30:00");
    }

    #[test]
    fn from_str_parses_t_separated_format_only() {
        assert_eq!("2023-06-15T09:30:00".parse::<NaiveDateTime>(), Ok(dt(2023, 6, 15, 9, 30, 0)));
        // Space-separated (the `Display` format) is not accepted by
        // `FromStr` -- only the "T"-separated (`Debug`) format is.
        assert!("2023-06-15 09:30:00".parse::<NaiveDateTime>().is_err());
    }

    #[test]
    fn debug_and_from_str_round_trip() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15)
            .unwrap()
            .and_hms_nano_opt(9, 30, 0, 123_456_789)
            .unwrap();
        assert_eq!(format!("{d:?}").parse::<NaiveDateTime>(), Ok(d));
    }

    #[test]
    fn ordering_compares_date_first_then_time() {
        let a = dt(2023, 6, 15, 23, 0, 0);
        let b = dt(2023, 6, 16, 0, 0, 0);
        assert!(a < b);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_serializes_using_debug_t_separator_not_display() {
        // Regression test for the real bug fixed on 2026-07-13:
        // serializing via `Display` (space-separated) produced a string
        // `FromStr` (which expects "T") could not parse back.
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap().and_hms_milli_opt(9, 30, 0, 500).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "\"2023-06-15T09:30:00.500\"");
        let back: NaiveDateTime = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn ts_seconds_serde_helper_round_trips() {
        #[derive(::serde::Serialize, ::serde::Deserialize, PartialEq, Debug)]
        struct Wrapper {
            #[serde(with = "super::serde::ts_seconds")]
            d: NaiveDateTime,
        }
        let original = Wrapper { d: dt(2023, 6, 15, 9, 30, 0) };
        let json = serde_json::to_string(&original).unwrap();
        let back: Wrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
    }
}
