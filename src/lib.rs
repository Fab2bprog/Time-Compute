//! `time_compute` — a date/time computation library, with no external
//! dependency for its core (dates, times, durations). A few exceptions
//! are allowed: time zone handling (see [`offset`]), which relies on the
//! `tzdb`/`tz-rs` crates to read the IANA time zone database; and a
//! handful of optional, disabled-by-default crate features --
//! `serde`/`rkyv` (de)serialization (see the [`serde`] and [`rkyv`]
//! modules), `unstable-locales` (locale-aware formatting, see
//! [`Locale`]), `arbitrary` (fuzzing support), and `defmt` (compact
//! embedded-friendly formatting) -- each matching chrono's own optional
//! feature of the same name 1:1.
//!
//! Goal: offer an API as close as possible to `chrono`'s (same type names,
//! same methods, same behaviour), so that a project using `chrono` can
//! migrate by simply replacing `use chrono::...` with
//! `use time_compute::...`. The source code itself is entirely original.
//!
//! # Current status
//! - [`NaiveDate`]: a date without a time zone (proleptic Gregorian
//!   calendar), with day/month arithmetic and ISO 8601 weeks.
//! - [`NaiveTime`]: a time of day, with nanosecond precision and leap
//!   second support.
//! - [`NaiveDateTime`]: a date and time of day combined, without a time
//!   zone.
//! - [`TimeDelta`] (also available as [`Duration`], its historical name in
//!   `chrono`), [`Days`], [`Months`]: durations and calendar increments.
//! - [`Weekday`], [`Month`], [`Datelike`], [`Timelike`]: weekday, month,
//!   and access to a date's or time's components.
//! - [`Utc`], [`Local`], [`FixedOffset`], [`DateTime<Tz>`](DateTime),
//!   [`TimeZone`], [`Offset`], [`MappedLocalTime`]: time zones and
//!   timezone-aware date/times.
//! - [`format`]: `strftime`-style formatting and parsing (`StrftimeItems`,
//!   `Parsed`, RFC 2822/3339 helpers).
//! - [`WeekdaySet`]: a compact, `Copy` set of weekdays.
//! - [`round`]: rounding/truncating a date-time by a [`TimeDelta`] span or
//!   by a number of subsecond digits ([`DurationRound`], [`SubsecRound`]).
//! - [`serde`] (crate feature `serde`) and [`rkyv`] (crate features
//!   `rkyv`/`rkyv-16`/`rkyv-32`/`rkyv-64`/`rkyv-validation`): optional
//!   (de)serialization support, disabled by default.
//! - [`Locale`] (crate feature `unstable-locales`): locale-aware formatting
//!   via `NaiveDate::format_localized`/`DateTime::format_localized` (and
//!   the `_with_items` variants), matching chrono's own
//!   `unstable-locales` feature name and "unstable" semantics 1:1. Falls
//!   back to English-only formatting when the feature is disabled.
//! - Crate feature `arbitrary`: `arbitrary::Arbitrary` support (for
//!   fuzzing) on the same set of public types as chrono.
//! - Crate feature `defmt`: `defmt::Format` support (compact `Debug`-like
//!   output for embedded/`no_std` targets) on the same set of public
//!   types as chrono.

#![forbid(unsafe_code)]

mod buddhist_calendar;
mod calendar;
mod chinese_calendar;
mod datetime;
mod duration;
pub mod format;
mod japanese_era;
mod matariki;
mod month;
pub mod naive;
pub mod offset;
pub mod round;
mod traits;
mod weekday;
mod weekday_set;

pub use datetime::DateTime;
#[allow(deprecated)]
#[doc(no_inline)]
pub use datetime::{MAX_DATETIME, MIN_DATETIME};
pub use duration::{Days, Duration, Months, OutOfRangeError, TimeDelta};
pub use format::{ParseError, ParseResult, SecondsFormat};
#[cfg(feature = "unstable-locales")]
pub use format::Locale;
// `JapaneseEra`: a `time_compute` extension -- not part of chrono, see
// `japanese_era.rs`.
pub use japanese_era::JapaneseEra;
pub use month::{Month, OutOfRange, ParseMonthError};
#[doc(inline)]
pub use naive::{IsoWeek, NaiveDate, NaiveDateTime, NaiveTime, NaiveWeek};
#[doc(inline)]
pub use offset::{FixedOffset, Local, LocalResult, MappedLocalTime, Offset, TimeZone, Utc};
pub use round::{DurationRound, RoundingError, SubsecRound};
pub use traits::{Datelike, Timelike};
pub use weekday::{ParseWeekdayError, Weekday};
// `WeekdaySetIter` is intentionally not re-exported: real chrono keeps
// `weekday_set` private and only exposes `WeekdaySet` itself, so the
// iterator returned by `WeekdaySet::iter` is unnameable outside the crate
// (callers just consume it via `for`/adapters). Matched here for parity.
pub use weekday_set::WeekdaySet;

/// Serialization/Deserialization with `serde`.
///
/// [`DateTime<Tz>`](DateTime) (de)serializes to/from an RFC 3339 string by
/// default. This module provides alternatives for (de)serializing as a
/// Unix timestamp instead (`ts_seconds`, `ts_milliseconds`,
/// `ts_microseconds`, `ts_nanoseconds`, and their `_option` variants),
/// intended for use with serde's `#[serde(with = "...")]` field attribute.
///
/// *Available on crate feature `serde` only.*
#[cfg(feature = "serde")]
pub mod serde {
    pub use crate::datetime::serde::*;
}

/// Zero-copy (de)serialization with `rkyv`.
///
/// This module re-exports the `Archived*` versions of this crate's types.
///
/// *Available on crate features `rkyv`, `rkyv-16`, `rkyv-32`, or
/// `rkyv-64` only.*
#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
pub mod rkyv {
    pub use crate::datetime::ArchivedDateTime;
    pub use crate::duration::ArchivedTimeDelta;
    pub use crate::month::ArchivedMonth;
    pub use crate::naive::date::{ArchivedIsoWeek, ArchivedNaiveDate};
    pub use crate::naive::datetime::ArchivedNaiveDateTime;
    pub use crate::naive::time::ArchivedNaiveTime;
    pub use crate::offset::{ArchivedFixedOffset, ArchivedLocal, ArchivedUtc};
    pub use crate::weekday::ArchivedWeekday;

    /// Alias of [`ArchivedTimeDelta`].
    pub type ArchivedDuration = ArchivedTimeDelta;
}
