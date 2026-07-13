//! `FixedOffset`: a time zone with a fixed offset from UTC.

use crate::format::scan;
use crate::format::{ParseError, OUT_OF_RANGE};
use crate::naive::{NaiveDate, NaiveDateTime};
use crate::offset::{MappedLocalTime, Offset, TimeZone};
use core::fmt;
use core::str::FromStr;

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// A time zone with a fixed offset from UTC, from UTC-23:59:59 to
/// UTC+23:59:59.
///
/// API aligned with `chrono::FixedOffset`. The preferred way to build a
/// `DateTime<FixedOffset>` is through the [`TimeZone`] methods on a
/// `FixedOffset` value (see [`east_opt`](Self::east_opt) /
/// [`west_opt`](Self::west_opt)).
///
/// `FixedOffset` has no `serde` impl in chrono (only `DateTime<Tz>` itself
/// does), so none is provided here either; `rkyv` support is provided.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, Hash, Debug))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct FixedOffset {
    local_minus_utc: i32,
}

/// `arbitrary` support: picks a uniformly random offset within the valid
/// range.
#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for FixedOffset {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<FixedOffset> {
        let secs = u.int_in_range(-86_399..=86_399)?;
        FixedOffset::east_opt(secs).ok_or(arbitrary::Error::IncorrectFormat)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for FixedOffset {
    fn format(&self, f: defmt::Formatter) {
        let offset = self.local_minus_utc;
        let (sign, offset) = if offset < 0 { ('-', -offset) } else { ('+', offset) };
        let sec = offset.rem_euclid(60);
        let mins = offset.div_euclid(60);
        let min = mins.rem_euclid(60);
        let hour = mins.div_euclid(60);
        if sec == 0 {
            defmt::write!(f, "{}{:02}:{:02}", sign, hour, min)
        } else {
            defmt::write!(f, "{}{:02}:{:02}:{:02}", sign, hour, min, sec)
        }
    }
}

impl FixedOffset {
    /// Builds a `FixedOffset` for the Eastern Hemisphere with the given
    /// difference, in seconds, from UTC. A negative value means the
    /// Western Hemisphere.
    ///
    /// # Panics
    /// Panics if `secs` is out of bounds (see [`east_opt`](Self::east_opt)).
    #[deprecated(note = "use `east_opt()` instead")]
    pub fn east(secs: i32) -> FixedOffset {
        FixedOffset::east_opt(secs).expect("FixedOffset::east out of bounds")
    }

    /// Builds a `FixedOffset` for the Eastern Hemisphere with the given
    /// difference, in seconds, from UTC. A negative value means the
    /// Western Hemisphere. Returns `None` if `secs` is not strictly
    /// between -86,400 and 86,400 (i.e. within +/-24h, exclusive).
    pub const fn east_opt(secs: i32) -> Option<FixedOffset> {
        if -86_400 < secs && secs < 86_400 {
            Some(FixedOffset { local_minus_utc: secs })
        } else {
            None
        }
    }

    /// Builds a `FixedOffset` for the Western Hemisphere with the given
    /// difference, in seconds, from UTC. A negative value means the
    /// Eastern Hemisphere.
    ///
    /// # Panics
    /// Panics if `secs` is out of bounds (see [`west_opt`](Self::west_opt)).
    #[deprecated(note = "use `west_opt()` instead")]
    pub fn west(secs: i32) -> FixedOffset {
        FixedOffset::west_opt(secs).expect("FixedOffset::west out of bounds")
    }

    /// Builds a `FixedOffset` for the Western Hemisphere with the given
    /// difference, in seconds, from UTC. A negative value means the
    /// Eastern Hemisphere. Returns `None` if `secs` is not strictly
    /// between -86,400 and 86,400.
    pub const fn west_opt(secs: i32) -> Option<FixedOffset> {
        if -86_400 < secs && secs < 86_400 {
            Some(FixedOffset { local_minus_utc: -secs })
        } else {
            None
        }
    }

    /// Number of seconds to add to UTC to get the local time.
    pub const fn local_minus_utc(&self) -> i32 {
        self.local_minus_utc
    }

    /// Number of seconds to add to the local time to get UTC.
    pub const fn utc_minus_local(&self) -> i32 {
        -self.local_minus_utc
    }
}

impl TimeZone for FixedOffset {
    type Offset = FixedOffset;

    fn from_offset(offset: &FixedOffset) -> FixedOffset {
        *offset
    }

    fn offset_from_local_date(&self, _local: &NaiveDate) -> MappedLocalTime<FixedOffset> {
        MappedLocalTime::Single(*self)
    }

    fn offset_from_local_datetime(&self, _local: &NaiveDateTime) -> MappedLocalTime<FixedOffset> {
        MappedLocalTime::Single(*self)
    }

    fn offset_from_utc_date(&self, _utc: &NaiveDate) -> FixedOffset {
        *self
    }

    fn offset_from_utc_datetime(&self, _utc: &NaiveDateTime) -> FixedOffset {
        *self
    }
}

impl Offset for FixedOffset {
    fn fix(&self) -> FixedOffset {
        *self
    }
}

/// Same format as `Debug`: `+HH:MM`, `-HH:MM`, or `+HH:MM:SS`/`-HH:MM:SS`
/// when the offset has a non-zero second component.
impl fmt::Debug for FixedOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let offset = self.local_minus_utc;
        let (sign, offset) = if offset < 0 { ('-', -offset) } else { ('+', offset) };
        let sec = offset.rem_euclid(60);
        let mins = offset.div_euclid(60);
        let min = mins.rem_euclid(60);
        let hour = mins.div_euclid(60);
        if sec == 0 {
            write!(f, "{sign}{hour:02}:{min:02}")
        } else {
            write!(f, "{sign}{hour:02}:{min:02}:{sec:02}")
        }
    }
}

impl fmt::Display for FixedOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Parses a string such as `+09:00`, `-0400` or `+02` into a `FixedOffset`.
/// The colon is optional, and any amount of whitespace is allowed around it.
impl FromStr for FixedOffset {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, offset) = scan::timezone_offset(s, scan::colon_or_space, false, false, true)?;
        Self::east_opt(offset).ok_or(OUT_OF_RANGE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn east_opt_validity_bounds() {
        assert!(FixedOffset::east_opt(0).is_some());
        assert!(FixedOffset::east_opt(86_399).is_some());
        assert!(FixedOffset::east_opt(-86_399).is_some());
        assert!(FixedOffset::east_opt(86_400).is_none());
        assert!(FixedOffset::east_opt(-86_400).is_none());
    }

    #[test]
    fn west_opt_validity_bounds() {
        assert!(FixedOffset::west_opt(0).is_some());
        assert!(FixedOffset::west_opt(86_399).is_some());
        assert!(FixedOffset::west_opt(-86_399).is_some());
        assert!(FixedOffset::west_opt(86_400).is_none());
        assert!(FixedOffset::west_opt(-86_400).is_none());
    }

    #[test]
    fn east_and_west_are_mirror_images() {
        let east = FixedOffset::east_opt(3600).unwrap();
        let west = FixedOffset::west_opt(3600).unwrap();
        assert_eq!(east.local_minus_utc(), 3600);
        assert_eq!(west.local_minus_utc(), -3600);
        assert_eq!(east.local_minus_utc(), -west.local_minus_utc());
    }

    #[test]
    fn local_minus_utc_and_utc_minus_local_are_opposites() {
        let offset = FixedOffset::east_opt(3600).unwrap();
        assert_eq!(offset.local_minus_utc(), 3600);
        assert_eq!(offset.utc_minus_local(), -3600);
    }

    #[test]
    fn fix_returns_self() {
        let offset = FixedOffset::east_opt(3600).unwrap();
        assert_eq!(offset.fix(), offset);
    }

    #[test]
    fn debug_and_display_format_hh_mm() {
        assert_eq!(format!("{:?}", FixedOffset::east_opt(0).unwrap()), "+00:00");
        assert_eq!(FixedOffset::east_opt(0).unwrap().to_string(), "+00:00");
        assert_eq!(format!("{:?}", FixedOffset::east_opt(3600).unwrap()), "+01:00");
        assert_eq!(format!("{:?}", FixedOffset::west_opt(3600).unwrap()), "-01:00");
        // India Standard Time, a well-known half-hour offset.
        assert_eq!(format!("{:?}", FixedOffset::east_opt(19_800).unwrap()), "+05:30");
    }

    #[test]
    fn debug_shows_seconds_component_only_when_nonzero() {
        assert_eq!(format!("{:?}", FixedOffset::east_opt(3661).unwrap()), "+01:01:01");
    }

    #[test]
    fn from_str_parses_with_and_without_colon() {
        assert_eq!(FixedOffset::from_str("+09:00").unwrap(), FixedOffset::east_opt(9 * 3600).unwrap());
        assert_eq!(FixedOffset::from_str("-0400").unwrap(), FixedOffset::west_opt(4 * 3600).unwrap());
    }

    #[test]
    fn from_str_requires_minutes() {
        // Unlike the permissive `%#z`/`TimezoneOffsetPermissive` parser
        // used elsewhere, `FixedOffset::from_str` calls `timezone_offset`
        // with `allow_missing_minutes = false`, so hours alone (no
        // minutes) is rejected rather than defaulting to `:00`.
        assert!(FixedOffset::from_str("+02").is_err());
    }

    #[test]
    fn from_str_rejects_garbage() {
        assert!(FixedOffset::from_str("garbage").is_err());
        assert!(FixedOffset::from_str("").is_err());
    }

    #[test]
    fn timezone_trait_methods_return_self_regardless_of_date() {
        let offset = FixedOffset::east_opt(3600).unwrap();
        let date = crate::naive::NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(offset.offset_from_local_date(&date).single(), Some(offset));
        assert_eq!(offset.offset_from_utc_date(&date), offset);
        assert_eq!(FixedOffset::from_offset(&offset), offset);
    }
}
