//! `Local`: the system's local time zone.
//!
//! This is the one place in the crate that leans on an external
//! dependency (the zero-dependency rule has a single, explicitly
//! authorized exception for time zone handling). Time zone data and
//! system local-zone detection come from
//! the `tzdb`/`tz-rs` crates (a maintained reader for the IANA time zone
//! database); everything else here -- the `TimeZone` plumbing, the
//! fold/gap disambiguation, the fallback policy -- is original code.

use crate::naive::{NaiveDate, NaiveDateTime, NaiveTime};
use crate::offset::utc::Utc;
use crate::offset::{FixedOffset, MappedLocalTime, TimeZone};
use crate::traits::{Datelike, Timelike};
use crate::DateTime;

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// The system's local time zone.
///
/// API aligned with `chrono::Local`. The preferred way to get the current
/// local time is [`Local::now()`](Self::now).
///
/// `Local` has no `serde` impl in chrono (only `DateTime<Tz>` itself
/// does), so none is provided here either; `rkyv` support is provided.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq)),
    archive_attr(derive(Clone, Copy, Debug))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Local;

impl Local {
    /// Returns the current date and time in the local time zone.
    #[must_use]
    pub fn now() -> DateTime<Local> {
        Utc::now().with_timezone(&Local)
    }
}

/// Returns the system's local time zone, falling back to UTC if it cannot
/// be determined -- the same fallback policy `chrono` itself uses ("default
/// to UTC if no local timezone can be found").
fn local_tz() -> tz::TimeZoneRef<'static> {
    tzdb::local_tz().unwrap_or_else(tz::TimeZoneRef::utc)
}

/// Unix timestamp (seconds) of a `NaiveDateTime`, treated as already being
/// in UTC. Ignores any leap-second fraction, which is irrelevant to time
/// zone offset resolution.
fn to_utc_secs(dt: &NaiveDateTime) -> i64 {
    dt.date().to_epoch_days() as i64 * 86_400 + dt.time().num_seconds_from_midnight() as i64
}

impl TimeZone for Local {
    type Offset = FixedOffset;

    fn from_offset(_offset: &FixedOffset) -> Local {
        Local
    }

    fn offset_from_local_date(&self, local: &NaiveDate) -> MappedLocalTime<FixedOffset> {
        self.offset_from_local_datetime(&local.and_time(NaiveTime::MIN))
    }

    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> MappedLocalTime<FixedOffset> {
        let tz = local_tz();
        let found = match tz::DateTime::find(
            local.year(),
            local.month() as u8,
            local.day() as u8,
            local.hour() as u8,
            local.minute() as u8,
            local.second() as u8,
            0,
            tz,
        ) {
            Ok(found) => found,
            Err(_) => return MappedLocalTime::None,
        };

        let to_fixed = |dt: &tz::DateTime| FixedOffset::east_opt(dt.local_time_type().ut_offset());
        let requested = (local.hour(), local.minute(), local.second());
        let matches_requested = |dt: &tz::DateTime| {
            (dt.hour() as u32, dt.minute() as u32, dt.second() as u32) == requested
        };

        if let Some(unique) = found.unique() {
            return match to_fixed(&unique) {
                Some(off) => MappedLocalTime::Single(off),
                None => MappedLocalTime::None,
            };
        }

        // No unique result: either a fold (the wall-clock time is
        // ambiguous, e.g. clocks turned back for DST) or a gap (the
        // wall-clock time was skipped entirely, e.g. clocks turned
        // forward). `tz-rs` reports both cases the same way (`earliest`
        // and `latest` both present, `unique` absent), returning the
        // *boundary* times of the transition in the gap case. The two
        // cases are told apart by checking whether the returned wall
        // time still matches what was actually requested.
        match (found.earliest(), found.latest()) {
            (Some(e), Some(l)) if matches_requested(&e) && matches_requested(&l) => {
                match (to_fixed(&e), to_fixed(&l)) {
                    (Some(off_e), Some(off_l)) => MappedLocalTime::Ambiguous(off_e, off_l),
                    _ => MappedLocalTime::None,
                }
            }
            // Either a gap (wall time does not exist), or the two
            // candidates disagree with the request: no valid mapping.
            _ => MappedLocalTime::None,
        }
    }

    fn offset_from_utc_date(&self, utc: &NaiveDate) -> FixedOffset {
        self.offset_from_utc_datetime(&utc.and_time(NaiveTime::MIN))
    }

    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> FixedOffset {
        let tz = local_tz();
        let secs = to_utc_secs(utc);
        let offset = tz
            .find_local_time_type(secs)
            .expect("unable to determine the local time zone offset")
            .ut_offset();
        FixedOffset::east_opt(offset).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `Local` depends on the system's actual configured time zone (via
    // `tzdb`/`tz-rs`), which can be anything wherever `cargo test` runs.
    // These tests are deliberately written to hold true regardless of
    // which time zone that is -- no test here assumes a specific UTC
    // offset.

    #[test]
    fn debug_format_is_the_unit_struct_name() {
        assert_eq!(format!("{Local:?}"), "Local");
    }

    #[test]
    fn from_offset_always_returns_local() {
        let offset = FixedOffset::east_opt(3600).unwrap();
        let _: Local = Local::from_offset(&offset);
    }

    #[test]
    fn now_returns_a_plausible_recent_date() {
        let now = Local::now();
        // Sanity bound rather than an exact check: this crate did not
        // exist before 2020, and (short of a wildly wrong system clock)
        // will not run after year 9999.
        assert!(now.year() > 2020);
        assert!(now.year() < 9999);
    }

    #[test]
    fn round_trip_through_local_preserves_the_utc_instant() {
        // Converting a UTC instant to `Local` and back to `Utc` must
        // preserve the same absolute instant, regardless of the system's
        // configured time zone -- only the wall-clock reading and offset
        // should change along the way.
        let utc = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let local = utc.with_timezone(&Local);
        let back = local.with_timezone(&Utc);
        assert_eq!(back, utc);
    }

    #[test]
    fn offset_from_utc_date_and_datetime_agree_at_midnight() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let offset_from_date = Local.offset_from_utc_date(&date);
        let offset_from_datetime = Local.offset_from_utc_datetime(&date.and_time(NaiveTime::MIN));
        assert_eq!(offset_from_date, offset_from_datetime);
    }
}
