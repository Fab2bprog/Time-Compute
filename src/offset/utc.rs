//! `Utc`: the UTC (Coordinated Universal Time) time zone.

use crate::datetime::DateTime;
use crate::naive::{NaiveDate, NaiveDateTime};
use crate::offset::{FixedOffset, MappedLocalTime, Offset, TimeZone};
use core::fmt;

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// The UTC time zone. The most efficient time zone when the local time is
/// not needed. Also used as its own [`Offset`] (a dummy, zero-sized one).
///
/// API aligned with `chrono::Utc`. The preferred way to build a
/// `DateTime<Utc>` is through the [`TimeZone`] methods on `Utc` (see
/// [`with_ymd_and_hms`](TimeZone::with_ymd_and_hms),
/// [`timestamp_opt`](TimeZone::timestamp_opt)).
///
/// `Utc` has no `serde` impl in chrono (only `DateTime<Tz>` itself does),
/// so none is provided here either; `rkyv` support is provided.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Utc;

impl Utc {
    /// Returns the current date and time in UTC.
    #[must_use]
    pub fn now() -> DateTime<Utc> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before Unix epoch");
        DateTime::from_timestamp(now.as_secs() as i64, now.subsec_nanos()).unwrap()
    }
}

impl TimeZone for Utc {
    type Offset = Utc;

    fn from_offset(_offset: &Utc) -> Utc {
        Utc
    }

    fn offset_from_local_date(&self, _local: &NaiveDate) -> MappedLocalTime<Utc> {
        MappedLocalTime::Single(Utc)
    }

    fn offset_from_local_datetime(&self, _local: &NaiveDateTime) -> MappedLocalTime<Utc> {
        MappedLocalTime::Single(Utc)
    }

    fn offset_from_utc_date(&self, _utc: &NaiveDate) -> Utc {
        Utc
    }

    fn offset_from_utc_datetime(&self, _utc: &NaiveDateTime) -> Utc {
        Utc
    }
}

impl Offset for Utc {
    fn fix(&self) -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }
}

impl fmt::Debug for Utc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Z")
    }
}

impl fmt::Display for Utc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UTC")
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Utc {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "Z");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Datelike;

    #[test]
    fn fix_returns_zero_offset() {
        assert_eq!(Utc.fix(), FixedOffset::east_opt(0).unwrap());
    }

    #[test]
    fn debug_and_display_formats() {
        assert_eq!(format!("{:?}", Utc), "Z");
        assert_eq!(Utc.to_string(), "UTC");
    }

    #[test]
    fn timezone_trait_methods_are_trivial() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(Utc.offset_from_local_date(&date).single(), Some(Utc));
        assert_eq!(Utc.offset_from_utc_date(&date), Utc);
        assert_eq!(Utc::from_offset(&Utc), Utc);
    }

    #[test]
    fn now_returns_a_plausible_recent_date() {
        let now = Utc::now();
        // Sanity bound rather than an exact check (no fixed clock
        // available here): this crate did not exist before 2020, and
        // (short of a wildly wrong system clock) will not run after
        // year 9999.
        assert!(now.year() > 2020);
        assert!(now.year() < 9999);
    }
}
