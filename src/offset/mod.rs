//! Time zones: computing the offset between a local time and UTC.
//!
//! API aligned with `chrono::offset`. Four operations are provided by the
//! [`TimeZone`] trait:
//! 1. Converting a local [`NaiveDateTime`] to a [`DateTime<Tz>`](crate::DateTime).
//! 2. Converting a UTC [`NaiveDateTime`] to a [`DateTime<Tz>`](crate::DateTime).
//! 3. Converting a [`DateTime<Tz>`](crate::DateTime) back to a local `NaiveDateTime`.
//! 4. Constructing [`DateTime<Tz>`](crate::DateTime) values from various
//!    offsets.
//!
//! # Deferred
//! The deprecated `Date<Tz>` type (superseded by `DateTime<Tz>` in chrono
//! itself since 0.4.23) is not implemented; the `TimeZone` trait methods
//! that only exist to build it (`ymd`, `ymd_opt`, `yo`, `yo_opt`,
//! `isoywd`, `isoywd_opt`, `from_local_date`, `from_utc_date`) are left
//! out accordingly. `datetime_from_str` is deferred to the formatting
//! step (step 7).

mod fixed;
pub use fixed::FixedOffset;
#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
pub use fixed::ArchivedFixedOffset;

mod local;
pub use local::Local;
#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
pub use local::ArchivedLocal;

mod utc;
pub use utc::Utc;
#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
pub use utc::ArchivedUtc;

use crate::datetime::DateTime;
use crate::naive::{NaiveDate, NaiveDateTime};

/// The result of mapping a local time to a concrete instant in a given
/// time zone: a single unambiguous result, an ambiguous result (the local
/// time falls in a "fold", e.g. when clocks are turned back for DST), or
/// no result at all (the local time falls in a "gap", e.g. when clocks
/// are turned forward).
///
/// API aligned with `chrono::MappedLocalTime` (formerly, and still,
/// available as [`LocalResult`], its historical name, via a type alias).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MappedLocalTime<T> {
    /// The local time maps to a single, unambiguous result.
    Single(T),

    /// The local time is ambiguous: it falls in a "fold" (e.g. when
    /// clocks are turned back). Contains `(earliest, latest)`.
    Ambiguous(T, T),

    /// The local time does not exist: it falls in a "gap" (e.g. when
    /// clocks are turned forward), or an error occurred while resolving
    /// it (missing time zone data, an OS error, overflow, ...).
    None,
}

/// Historical name of [`MappedLocalTime`]. See that type for documentation.
pub type LocalResult<T> = MappedLocalTime<T>;

impl<T> MappedLocalTime<T> {
    /// Returns `Some` if the mapping has a single, unambiguous result.
    pub fn single(self) -> Option<T> {
        match self {
            MappedLocalTime::Single(t) => Some(t),
            _ => None,
        }
    }

    /// Returns the earliest possible result, or `None` for a gap or an
    /// error.
    pub fn earliest(self) -> Option<T> {
        match self {
            MappedLocalTime::Single(t) | MappedLocalTime::Ambiguous(t, _) => Some(t),
            _ => None,
        }
    }

    /// Returns the latest possible result, or `None` for a gap or an
    /// error.
    pub fn latest(self) -> Option<T> {
        match self {
            MappedLocalTime::Single(t) | MappedLocalTime::Ambiguous(_, t) => Some(t),
            _ => None,
        }
    }

    /// Maps the contained value(s) with the given function.
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> MappedLocalTime<U> {
        match self {
            MappedLocalTime::None => MappedLocalTime::None,
            MappedLocalTime::Single(v) => MappedLocalTime::Single(f(v)),
            MappedLocalTime::Ambiguous(min, max) => MappedLocalTime::Ambiguous(f(min), f(max)),
        }
    }

    pub(crate) fn and_then<U, F: FnMut(T) -> Option<U>>(self, mut f: F) -> MappedLocalTime<U> {
        match self {
            MappedLocalTime::None => MappedLocalTime::None,
            MappedLocalTime::Single(v) => match f(v) {
                Some(new) => MappedLocalTime::Single(new),
                None => MappedLocalTime::None,
            },
            MappedLocalTime::Ambiguous(min, max) => match (f(min), f(max)) {
                (Some(min), Some(max)) => MappedLocalTime::Ambiguous(min, max),
                _ => MappedLocalTime::None,
            },
        }
    }
}

impl<T: core::fmt::Debug> MappedLocalTime<T> {
    /// Returns the single, unambiguous result, or panics.
    ///
    /// Best used with time zones where the mapping can never fail, like
    /// [`Utc`] and [`FixedOffset`] (though even `FixedOffset` can, in
    /// rare cases, produce an out-of-range `DateTime`).
    ///
    /// # Panics
    /// Panics if the local time falls in a fold or a gap, or if an error
    /// occurred.
    #[track_caller]
    pub fn unwrap(self) -> T {
        match self {
            MappedLocalTime::None => panic!("No such local time"),
            MappedLocalTime::Single(t) => t,
            MappedLocalTime::Ambiguous(t1, t2) => {
                panic!("Ambiguous local time, ranging from {t1:?} to {t2:?}")
            }
        }
    }
}

/// The offset from a local time to UTC, associated with a given
/// [`TimeZone`].
pub trait Offset: Sized + Clone + core::fmt::Debug {
    /// The fixed offset from UTC to the local time this value represents.
    fn fix(&self) -> FixedOffset;
}

/// A time zone: computes the offset(s) between UTC and local time.
///
/// The methods here are the primary constructors for [`DateTime<Tz>`](DateTime).
pub trait TimeZone: Sized + Clone {
    /// The associated offset type, used to cache the actual offset within
    /// date/time values. The original `TimeZone` can be recovered from it
    /// via [`from_offset`](Self::from_offset).
    type Offset: Offset;

    /// Builds a new `DateTime` from year/month/day/hour/minute/second
    /// components and the current time zone. Assumes the proleptic
    /// Gregorian calendar, with year 0 being 1 BCE.
    ///
    /// Returns `MappedLocalTime::None` on invalid input.
    fn with_ymd_and_hms(
        &self,
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
        sec: u32,
    ) -> MappedLocalTime<DateTime<Self>> {
        match NaiveDate::from_ymd_opt(year, month, day).and_then(|d| d.and_hms_opt(hour, min, sec))
        {
            Some(dt) => self.from_local_datetime(&dt),
            None => MappedLocalTime::None,
        }
    }

    /// Builds a new `DateTime` from the number of non-leap seconds since
    /// January 1, 1970 0:00:00 UTC (a "UNIX timestamp") and a nanosecond
    /// remainder.
    ///
    /// # Panics
    /// Panics on out-of-range input; see [`timestamp_opt`](Self::timestamp_opt)
    /// for a non-panicking version.
    #[deprecated(note = "use `timestamp_opt()` instead")]
    fn timestamp(&self, secs: i64, nsecs: u32) -> DateTime<Self> {
        self.timestamp_opt(secs, nsecs).unwrap()
    }

    /// Builds a new `DateTime` from the number of non-leap seconds since
    /// the Unix epoch and a nanosecond remainder. The nanosecond part may
    /// exceed 1,000,000,000 to represent a leap second (only when
    /// `secs % 60 == 59`).
    ///
    /// Returns `MappedLocalTime::None` on out-of-range input.
    fn timestamp_opt(&self, secs: i64, nsecs: u32) -> MappedLocalTime<DateTime<Self>> {
        match DateTime::from_timestamp(secs, nsecs) {
            Some(dt) => MappedLocalTime::Single(self.from_utc_datetime(&dt.naive_utc())),
            None => MappedLocalTime::None,
        }
    }

    /// Builds a new `DateTime` from the number of non-leap milliseconds
    /// since the Unix epoch.
    ///
    /// # Panics
    /// Panics on out-of-range input; see
    /// [`timestamp_millis_opt`](Self::timestamp_millis_opt) for a
    /// non-panicking version.
    #[deprecated(note = "use `timestamp_millis_opt()` instead")]
    fn timestamp_millis(&self, millis: i64) -> DateTime<Self> {
        self.timestamp_millis_opt(millis).unwrap()
    }

    /// Builds a new `DateTime` from the number of non-leap milliseconds
    /// since the Unix epoch. Returns `MappedLocalTime::None` on
    /// out-of-range input.
    fn timestamp_millis_opt(&self, millis: i64) -> MappedLocalTime<DateTime<Self>> {
        match DateTime::from_timestamp_millis(millis) {
            Some(dt) => MappedLocalTime::Single(self.from_utc_datetime(&dt.naive_utc())),
            None => MappedLocalTime::None,
        }
    }

    /// Builds a new `DateTime` from the number of non-leap nanoseconds
    /// since the Unix epoch. Never fails.
    fn timestamp_nanos(&self, nanos: i64) -> DateTime<Self> {
        self.from_utc_datetime(&DateTime::from_timestamp_nanos(nanos).naive_utc())
    }

    /// Builds a new `DateTime` from the number of non-leap microseconds
    /// since the Unix epoch. Returns `MappedLocalTime::None` on
    /// out-of-range input.
    fn timestamp_micros(&self, micros: i64) -> MappedLocalTime<DateTime<Self>> {
        match DateTime::from_timestamp_micros(micros) {
            Some(dt) => MappedLocalTime::Single(self.from_utc_datetime(&dt.naive_utc())),
            None => MappedLocalTime::None,
        }
    }

    /// Reconstructs the time zone from one of its offsets.
    fn from_offset(offset: &Self::Offset) -> Self;

    /// Computes the offset(s) for a given local `NaiveDate`, at midnight.
    fn offset_from_local_date(&self, local: &NaiveDate) -> MappedLocalTime<Self::Offset>;

    /// Computes the offset(s) for a given local `NaiveDateTime`.
    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> MappedLocalTime<Self::Offset>;

    /// Converts a local `NaiveDateTime` to a timezone-aware `DateTime`, if
    /// possible.
    // This name mirrors chrono's own `TimeZone::from_local_datetime` and
    // cannot be renamed to satisfy `wrong_self_convention` without breaking
    // API compatibility with chrono, which is a hard requirement of this
    // crate.
    #[allow(clippy::wrong_self_convention)]
    fn from_local_datetime(&self, local: &NaiveDateTime) -> MappedLocalTime<DateTime<Self>> {
        self.offset_from_local_datetime(local).and_then(|off| {
            local
                .checked_sub_offset(off.fix())
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, off))
        })
    }

    /// Computes the offset for a given UTC `NaiveDate`. Cannot fail.
    fn offset_from_utc_date(&self, utc: &NaiveDate) -> Self::Offset;

    /// Computes the offset for a given UTC `NaiveDateTime`. Cannot fail.
    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset;

    /// Converts a UTC `NaiveDateTime` to the local time. The UTC timeline
    /// is continuous, so this cannot fail (though it may produce a
    /// duplicate local time during a DST fold).
    // Same rationale as `from_local_datetime` above: the name is dictated by
    // chrono's public API and must not change.
    #[allow(clippy::wrong_self_convention)]
    fn from_utc_datetime(&self, utc: &NaiveDateTime) -> DateTime<Self> {
        DateTime::from_naive_utc_and_offset(*utc, self.offset_from_utc_datetime(utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{Datelike, Timelike};

    #[test]
    fn single_earliest_latest_on_single_variant() {
        let m = MappedLocalTime::Single(5);
        assert_eq!(m.single(), Some(5));
        assert_eq!(m.earliest(), Some(5));
        assert_eq!(m.latest(), Some(5));
    }

    #[test]
    fn single_earliest_latest_on_ambiguous_variant() {
        let m = MappedLocalTime::Ambiguous(1, 2);
        assert_eq!(m.single(), None);
        assert_eq!(m.earliest(), Some(1));
        assert_eq!(m.latest(), Some(2));
    }

    #[test]
    fn single_earliest_latest_on_none_variant() {
        let m: MappedLocalTime<i32> = MappedLocalTime::None;
        assert_eq!(m.single(), None);
        assert_eq!(m.earliest(), None);
        assert_eq!(m.latest(), None);
    }

    #[test]
    fn map_transforms_the_contained_values() {
        assert_eq!(MappedLocalTime::Single(5).map(|x| x * 2), MappedLocalTime::Single(10));
        assert_eq!(MappedLocalTime::Ambiguous(1, 2).map(|x| x * 2), MappedLocalTime::Ambiguous(2, 4));
        let none: MappedLocalTime<i32> = MappedLocalTime::None;
        assert_eq!(none.map(|x| x * 2), MappedLocalTime::None);
    }

    #[test]
    fn and_then_propagates_failure() {
        assert_eq!(MappedLocalTime::Single(5).and_then(|x| Some(x + 1)), MappedLocalTime::Single(6));
        assert_eq!(MappedLocalTime::Single(5).and_then(|_: i32| None::<i32>), MappedLocalTime::None);
        assert_eq!(
            MappedLocalTime::Ambiguous(1, 2).and_then(|x| Some(x + 1)),
            MappedLocalTime::Ambiguous(2, 3)
        );
    }

    #[test]
    fn unwrap_returns_the_single_value() {
        assert_eq!(MappedLocalTime::Single(5).unwrap(), 5);
    }

    #[test]
    #[should_panic]
    fn unwrap_panics_on_none() {
        let m: MappedLocalTime<i32> = MappedLocalTime::None;
        m.unwrap();
    }

    #[test]
    #[should_panic]
    fn unwrap_panics_on_ambiguous() {
        MappedLocalTime::Ambiguous(1, 2).unwrap();
    }

    #[test]
    fn local_result_is_an_alias() {
        let m: LocalResult<i32> = MappedLocalTime::Single(5);
        assert_eq!(m, MappedLocalTime::Single(5));
    }

    // Default `TimeZone` trait method bodies, exercised via `Utc` (a
    // fully deterministic time zone, unlike `Local` which depends on the
    // system's configuration).

    #[test]
    fn with_ymd_and_hms_builds_the_expected_datetime() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 12, 30, 0).single().unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.hour(), 12);
    }

    #[test]
    fn with_ymd_and_hms_rejects_invalid_calendar_dates() {
        assert_eq!(Utc.with_ymd_and_hms(2023, 2, 30, 0, 0, 0).single(), None);
    }

    #[test]
    fn timestamp_opt_and_deprecated_timestamp_agree() {
        let opt = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        #[allow(deprecated)]
        let panicking = Utc.timestamp(1_700_000_000, 0);
        assert_eq!(opt, panicking);
    }

    #[test]
    fn timestamp_millis_micros_and_nanos_agree_on_a_round_value() {
        let from_secs = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        let from_millis = Utc.timestamp_millis_opt(1_700_000_000_000).single().unwrap();
        let from_micros = Utc.timestamp_micros(1_700_000_000_000_000).single().unwrap();
        let from_nanos = Utc.timestamp_nanos(1_700_000_000_000_000_000);
        assert_eq!(from_secs, from_millis);
        assert_eq!(from_secs, from_micros);
        assert_eq!(from_secs, from_nanos);
    }

    #[test]
    fn from_local_datetime_and_from_utc_datetime_are_identity_for_utc() {
        let naive = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap().and_hms_opt(12, 30, 0).unwrap();
        let from_local = Utc.from_local_datetime(&naive).single().unwrap();
        let from_utc = Utc.from_utc_datetime(&naive);
        assert_eq!(from_local, from_utc);
        assert_eq!(from_utc.naive_utc(), naive);
    }
}
