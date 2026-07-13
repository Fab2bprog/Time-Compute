//! "Naive" date/time types, i.e. without an associated time zone.

pub mod date;
pub mod datetime;
pub mod iter;
pub mod time;
pub mod week;

#[allow(deprecated)]
pub use date::{IsoWeek, NaiveDate, MAX_DATE, MIN_DATE};
#[allow(deprecated)]
pub use datetime::{NaiveDateTime, MAX_DATETIME, MIN_DATETIME};
pub use iter::{NaiveDateDaysIterator, NaiveDateWeeksIterator};
pub use time::NaiveTime;
pub use week::NaiveWeek;

/// (De)serialization of `NaiveDateTime` in alternate formats.
///
/// The various modules in here are intended to be used with serde's
/// `#[serde(with = "...")]` field attribute to serialize as something
/// other than the default ISO 8601 string.
#[cfg(feature = "serde")]
pub mod serde {
    pub use super::datetime::serde::*;
}
