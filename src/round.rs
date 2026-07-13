//! Rounding and truncating dates/times by a fixed span (subsecond digits or
//! an arbitrary [`TimeDelta`]).

use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, Sub};

use crate::datetime::DateTime;
use crate::naive::datetime::NaiveDateTime;
use crate::offset::TimeZone;
use crate::traits::Timelike;
use crate::duration::TimeDelta;

/// Extension trait to round or truncate a value's subsecond precision down
/// to a maximum number of fractional digits.
///
/// Rounding is useful to reduce noise when persisting to a lower-precision
/// format; truncating matches the default behaviour of `Display`
/// formatting (which never rounds up).
///
/// API aligned with `chrono::SubsecRound`.
pub trait SubsecRound {
    /// Returns a copy rounded to `digits` fractional digits (halfway values
    /// round away from zero). With 9 or more digits, `self` is returned
    /// unchanged.
    fn round_subsecs(self, digits: u16) -> Self;

    /// Returns a copy truncated to `digits` fractional digits. With 9 or
    /// more digits, `self` is returned unchanged.
    fn trunc_subsecs(self, digits: u16) -> Self;
}

impl<T> SubsecRound for T
where
    T: Timelike + Add<TimeDelta, Output = T> + Sub<TimeDelta, Output = T>,
{
    fn round_subsecs(self, digits: u16) -> T {
        let span = span_for_digits(digits);
        let delta_down = self.nanosecond() % span;
        if delta_down == 0 {
            return self;
        }
        let delta_up = span - delta_down;
        if delta_up <= delta_down {
            self + TimeDelta::nanoseconds(i64::from(delta_up))
        } else {
            self - TimeDelta::nanoseconds(i64::from(delta_down))
        }
    }

    fn trunc_subsecs(self, digits: u16) -> T {
        let span = span_for_digits(digits);
        let delta_down = self.nanosecond() % span;
        if delta_down == 0 {
            self
        } else {
            self - TimeDelta::nanoseconds(i64::from(delta_down))
        }
    }
}

/// Largest span, in nanoseconds, that still fits in `digits` fractional
/// digits (i.e. `10^(9 - min(9, digits))`).
const fn span_for_digits(digits: u16) -> u32 {
    match digits {
        0 => 1_000_000_000,
        1 => 100_000_000,
        2 => 10_000_000,
        3 => 1_000_000,
        4 => 100_000,
        5 => 10_000,
        6 => 1_000,
        7 => 100,
        8 => 10,
        _ => 1,
    }
}

/// Extension trait to round or truncate a date/time by an arbitrary
/// [`TimeDelta`] span (for example, the nearest 15 minutes).
///
/// # Limitations
/// The implementation converts both the span and the value to a nanosecond
/// count (via [`TimeDelta::num_nanoseconds`] and
/// [`DateTime::timestamp_nanos_opt`]), so it fails whenever either does not
/// fit in an `i64`, or when `duration` is zero or negative.
///
/// API aligned with `chrono::DurationRound`.
pub trait DurationRound: Sized {
    /// Error produced when rounding or truncating is not possible.
    type Err: fmt::Debug + fmt::Display;

    /// Returns a copy rounded to the nearest multiple of `duration`
    /// (halfway values round up).
    fn duration_round(self, duration: TimeDelta) -> Result<Self, Self::Err>;

    /// Returns a copy truncated down to the previous multiple of
    /// `duration`.
    fn duration_trunc(self, duration: TimeDelta) -> Result<Self, Self::Err>;

    /// Returns a copy rounded up to the next multiple of `duration` (the
    /// value itself, if it is already an exact multiple).
    fn duration_round_up(self, duration: TimeDelta) -> Result<Self, Self::Err>;
}

impl<Tz: TimeZone> DurationRound for DateTime<Tz> {
    type Err = RoundingError;

    fn duration_round(self, duration: TimeDelta) -> Result<Self, Self::Err> {
        duration_round(self.naive_local(), self, duration)
    }

    fn duration_trunc(self, duration: TimeDelta) -> Result<Self, Self::Err> {
        duration_trunc(self.naive_local(), self, duration)
    }

    fn duration_round_up(self, duration: TimeDelta) -> Result<Self, Self::Err> {
        duration_round_up(self.naive_local(), self, duration)
    }
}

impl DurationRound for NaiveDateTime {
    type Err = RoundingError;

    fn duration_round(self, duration: TimeDelta) -> Result<Self, Self::Err> {
        duration_round(self, self, duration)
    }

    fn duration_trunc(self, duration: TimeDelta) -> Result<Self, Self::Err> {
        duration_trunc(self, self, duration)
    }

    fn duration_round_up(self, duration: TimeDelta) -> Result<Self, Self::Err> {
        duration_round_up(self, self, duration)
    }
}

fn duration_round<T>(naive: NaiveDateTime, original: T, duration: TimeDelta) -> Result<T, RoundingError>
where
    T: Timelike + Add<TimeDelta, Output = T> + Sub<TimeDelta, Output = T>,
{
    let span = duration.num_nanoseconds().filter(|&s| s > 0).ok_or(RoundingError::DurationExceedsLimit)?;
    let stamp = naive.and_utc().timestamp_nanos_opt().ok_or(RoundingError::TimestampExceedsLimit)?;
    let delta_down = stamp % span;
    if delta_down == 0 {
        return Ok(original);
    }
    let (delta_up, delta_down) = if delta_down < 0 {
        (delta_down.abs(), span - delta_down.abs())
    } else {
        (span - delta_down, delta_down)
    };
    if delta_up <= delta_down {
        Ok(original + TimeDelta::nanoseconds(delta_up))
    } else {
        Ok(original - TimeDelta::nanoseconds(delta_down))
    }
}

fn duration_trunc<T>(naive: NaiveDateTime, original: T, duration: TimeDelta) -> Result<T, RoundingError>
where
    T: Timelike + Add<TimeDelta, Output = T> + Sub<TimeDelta, Output = T>,
{
    let span = duration.num_nanoseconds().filter(|&s| s > 0).ok_or(RoundingError::DurationExceedsLimit)?;
    let stamp = naive.and_utc().timestamp_nanos_opt().ok_or(RoundingError::TimestampExceedsLimit)?;
    let delta_down = stamp % span;
    match delta_down.cmp(&0) {
        Ordering::Equal => Ok(original),
        Ordering::Greater => Ok(original - TimeDelta::nanoseconds(delta_down)),
        Ordering::Less => Ok(original - TimeDelta::nanoseconds(span - delta_down.abs())),
    }
}

fn duration_round_up<T>(naive: NaiveDateTime, original: T, duration: TimeDelta) -> Result<T, RoundingError>
where
    T: Timelike + Add<TimeDelta, Output = T> + Sub<TimeDelta, Output = T>,
{
    let span = duration.num_nanoseconds().filter(|&s| s > 0).ok_or(RoundingError::DurationExceedsLimit)?;
    let stamp = naive.and_utc().timestamp_nanos_opt().ok_or(RoundingError::TimestampExceedsLimit)?;
    let delta_down = stamp % span;
    match delta_down.cmp(&0) {
        Ordering::Equal => Ok(original),
        Ordering::Greater => Ok(original + TimeDelta::nanoseconds(span - delta_down)),
        Ordering::Less => Ok(original + TimeDelta::nanoseconds(delta_down.abs())),
    }
}

/// Error returned by [`DurationRound`] when rounding or truncating fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RoundingError {
    /// Kept for API parity with chrono; chrono itself notes this variant is
    /// no longer produced by its own implementation.
    DurationExceedsTimestamp,
    /// The rounding span, expressed in nanoseconds, does not fit in an
    /// `i64` (or is zero/negative).
    DurationExceedsLimit,
    /// The date/time's nanosecond timestamp does not fit in an `i64`.
    TimestampExceedsLimit,
}

impl fmt::Display for RoundingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoundingError::DurationExceedsTimestamp => {
                write!(f, "duration in nanoseconds exceeds timestamp")
            }
            RoundingError::DurationExceedsLimit => {
                write!(f, "duration exceeds num_nanoseconds limit")
            }
            RoundingError::TimestampExceedsLimit => {
                write!(f, "timestamp exceeds num_nanoseconds limit")
            }
        }
    }
}

impl std::error::Error for RoundingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::naive::date::NaiveDate;
    use crate::naive::time::NaiveTime;

    fn ndt(h: u32, mi: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2023, 6, 15).unwrap().and_hms_opt(h, mi, s).unwrap()
    }

    #[test]
    fn round_subsecs_exact_halfway_rounds_away_from_zero() {
        let t = NaiveTime::from_hms_nano_opt(12, 0, 0, 500_000_000).unwrap();
        assert_eq!(t.round_subsecs(0), NaiveTime::from_hms_opt(12, 0, 1).unwrap());
        assert_eq!(t.round_subsecs(1), t); // already exact at 1 fractional digit
    }

    #[test]
    fn round_subsecs_rounds_to_the_closer_multiple() {
        let below = NaiveTime::from_hms_milli_opt(12, 0, 0, 100).unwrap();
        assert_eq!(below.round_subsecs(0), NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        let above = NaiveTime::from_hms_milli_opt(12, 0, 0, 900).unwrap();
        assert_eq!(above.round_subsecs(0), NaiveTime::from_hms_opt(12, 0, 1).unwrap());
    }

    #[test]
    fn round_subsecs_is_a_no_op_at_9_or_more_digits() {
        let t = NaiveTime::from_hms_nano_opt(12, 0, 0, 123_456_789).unwrap();
        assert_eq!(t.round_subsecs(9), t);
        assert_eq!(t.round_subsecs(20), t);
    }

    #[test]
    fn trunc_subsecs_always_rounds_toward_zero() {
        let t = NaiveTime::from_hms_nano_opt(12, 0, 0, 999_000_000).unwrap();
        assert_eq!(t.trunc_subsecs(0), NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        assert_eq!(t.trunc_subsecs(1), NaiveTime::from_hms_milli_opt(12, 0, 0, 900).unwrap());
        assert_eq!(t.trunc_subsecs(2), NaiveTime::from_hms_milli_opt(12, 0, 0, 990).unwrap());
        assert_eq!(t.trunc_subsecs(9), t);
    }

    #[test]
    fn round_and_trunc_subsecs_also_work_on_naivedatetime() {
        let dt = ndt(12, 0, 0) + TimeDelta::milliseconds(600);
        assert_eq!(dt.trunc_subsecs(1).time().nanosecond(), 600_000_000);
        // 600ms is already an exact multiple of one decimal digit (100ms),
        // so rounding to 1 digit leaves it unchanged at 12:00:00.6 --
        // it does not round up to the next second.
        assert_eq!(dt.round_subsecs(1).time(), NaiveTime::from_hms_milli_opt(12, 0, 0, 600).unwrap());
    }

    #[test]
    fn duration_round_rounds_to_the_nearer_multiple() {
        // 12:40:00 is 40 minutes past noon, 20 minutes before 13:00 --
        // closer to the next hour. Whole-day contributions to the
        // underlying Unix timestamp are always a multiple of 3600s
        // (86_400 / 3600 = 24 exactly), so this reasoning holds
        // regardless of which calendar day is used.
        assert_eq!(ndt(12, 40, 0).duration_round(TimeDelta::hours(1)).unwrap(), ndt(13, 0, 0));
        // 12:10:00 is 10 minutes past noon -- closer to the previous hour.
        assert_eq!(ndt(12, 10, 0).duration_round(TimeDelta::hours(1)).unwrap(), ndt(12, 0, 0));
    }

    #[test]
    fn duration_trunc_always_rounds_down() {
        assert_eq!(ndt(12, 40, 0).duration_trunc(TimeDelta::hours(1)).unwrap(), ndt(12, 0, 0));
        assert_eq!(ndt(12, 10, 0).duration_trunc(TimeDelta::hours(1)).unwrap(), ndt(12, 0, 0));
    }

    #[test]
    fn duration_round_up_always_rounds_up_unless_already_exact() {
        assert_eq!(ndt(12, 40, 0).duration_round_up(TimeDelta::hours(1)).unwrap(), ndt(13, 0, 0));
        assert_eq!(ndt(12, 10, 0).duration_round_up(TimeDelta::hours(1)).unwrap(), ndt(13, 0, 0));
        assert_eq!(ndt(12, 0, 0).duration_round_up(TimeDelta::hours(1)).unwrap(), ndt(12, 0, 0));
    }

    #[test]
    fn all_three_are_no_ops_on_an_exact_multiple() {
        let exact = ndt(12, 0, 0);
        assert_eq!(exact.duration_round(TimeDelta::hours(1)).unwrap(), exact);
        assert_eq!(exact.duration_trunc(TimeDelta::hours(1)).unwrap(), exact);
        assert_eq!(exact.duration_round_up(TimeDelta::hours(1)).unwrap(), exact);
    }

    #[test]
    fn duration_round_rejects_zero_or_negative_span() {
        let d = ndt(12, 0, 0);
        assert_eq!(d.duration_round(TimeDelta::zero()), Err(RoundingError::DurationExceedsLimit));
        assert_eq!(d.duration_round(TimeDelta::hours(-1)), Err(RoundingError::DurationExceedsLimit));
    }

    #[test]
    fn duration_round_also_works_on_datetime_utc() {
        let d = ndt(12, 40, 0).and_utc();
        let rounded = d.duration_round(TimeDelta::hours(1)).unwrap();
        assert_eq!(rounded.naive_utc(), ndt(13, 0, 0));
    }

    #[test]
    fn rounding_error_display() {
        assert_eq!(
            RoundingError::DurationExceedsLimit.to_string(),
            "duration exceeds num_nanoseconds limit"
        );
        assert_eq!(
            RoundingError::TimestampExceedsLimit.to_string(),
            "timestamp exceeds num_nanoseconds limit"
        );
    }
}
