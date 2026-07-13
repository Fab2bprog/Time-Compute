//! Signed durations ([`TimeDelta`], also available under the name
//! [`Duration`] via a type alias, exactly like in `chrono`) and calendar
//! increments (`Days`, `Months`).

use crate::calendar::div_floor;
use core::fmt;
use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

const NANOS_PER_SEC: i32 = 1_000_000_000;
const NANOS_PER_MILLI: i32 = 1_000_000;
const NANOS_PER_MICRO: i32 = 1_000;
const MICROS_PER_SEC: i64 = 1_000_000;
const MILLIS_PER_SEC: i64 = 1_000;
const SECS_PER_MINUTE: i64 = 60;
const SECS_PER_HOUR: i64 = 3_600;
const SECS_PER_DAY: i64 = 86_400;
const SECS_PER_WEEK: i64 = 604_800;

/// A signed span of time, with nanosecond precision.
///
/// API aligned with `chrono::TimeDelta` (the type historically, and still,
/// available under the name [`Duration`] via a type alias, exactly as in
/// `chrono`): same constructors, same `num_*`/`subsec_*` accessors, same
/// operators.
///
/// Internal representation: `secs` carries the sign of the duration,
/// `nanos` always lies in `0..1_000_000_000` (a positive "remainder"). For
/// example, -1.5 seconds is stored as `secs = -2, nanos = 500_000_000`.
///
/// The representable range is restricted to `i64::MAX` milliseconds in
/// either direction (so `MIN` is `-i64::MAX` milliseconds, not
/// `i64::MIN`), which keeps sign-flipping (`-x`, [`abs`](Self::abs))
/// always safe. This matches `chrono` exactly, including the fact that the
/// range is not perfectly symmetric once sub-millisecond precision is
/// taken into account (see [`MIN`](Self::MIN)/[`MAX`](Self::MAX)).
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TimeDelta {
    secs: i64,
    nanos: i32,
}

/// `arbitrary` support: generates a value uniformly within `MIN..=MAX`,
/// mirroring chrono's own strategy (pick `secs`/`nanos` within the widest
/// possible range for each, then reject the rare combination that still
/// falls outside `MIN..=MAX`).
#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for TimeDelta {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<TimeDelta> {
        let secs: i64 = u.int_in_range(TimeDelta::MIN.secs..=TimeDelta::MAX.secs)?;
        let nanos: i32 = u.int_in_range(0..=(NANOS_PER_SEC - 1))?;
        TimeDelta::new(secs, nanos as u32).ok_or(arbitrary::Error::IncorrectFormat)
    }
}

impl TimeDelta {
    /// The minimum possible `TimeDelta`: `-i64::MAX` milliseconds.
    pub const MIN: Self = TimeDelta {
        secs: -i64::MAX / MILLIS_PER_SEC - 1,
        nanos: NANOS_PER_SEC + (-i64::MAX % MILLIS_PER_SEC) as i32 * NANOS_PER_MILLI,
    };

    /// The maximum possible `TimeDelta`: `i64::MAX` milliseconds.
    pub const MAX: Self = TimeDelta {
        secs: i64::MAX / MILLIS_PER_SEC,
        nanos: (i64::MAX % MILLIS_PER_SEC) as i32 * NANOS_PER_MILLI,
    };

    /// A zero-length duration.
    pub const fn zero() -> Self {
        TimeDelta { secs: 0, nanos: 0 }
    }

    /// `true` if the duration is zero.
    pub const fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

    /// Builds a duration from a number of whole seconds and a nanosecond
    /// remainder (`0..1_000_000_000`). Returns `None` if the duration would
    /// be out of the representable range.
    pub const fn new(secs: i64, nanos: u32) -> Option<TimeDelta> {
        if secs < Self::MIN.secs
            || secs > Self::MAX.secs
            || nanos >= NANOS_PER_SEC as u32
            || (secs == Self::MAX.secs && nanos > Self::MAX.nanos as u32)
            || (secs == Self::MIN.secs && nanos < Self::MIN.nanos as u32)
        {
            return None;
        }
        Some(TimeDelta { secs, nanos: nanos as i32 })
    }

    /// Builds a duration from a number of weeks. Returns `None` on
    /// overflow.
    pub const fn try_weeks(weeks: i64) -> Option<TimeDelta> {
        match weeks.checked_mul(SECS_PER_WEEK) {
            Some(secs) => TimeDelta::try_seconds(secs),
            None => None,
        }
    }

    /// Builds a duration from a number of weeks, like
    /// [`try_weeks`](Self::try_weeks).
    ///
    /// # Panics
    /// Panics when the duration would be out of bounds.
    pub const fn weeks(weeks: i64) -> TimeDelta {
        match TimeDelta::try_weeks(weeks) {
            Some(d) => d,
            None => panic!("TimeDelta::weeks out of bounds"),
        }
    }

    /// Builds a duration from a number of days. Returns `None` on overflow.
    pub const fn try_days(days: i64) -> Option<TimeDelta> {
        match days.checked_mul(SECS_PER_DAY) {
            Some(secs) => TimeDelta::try_seconds(secs),
            None => None,
        }
    }

    /// Builds a duration from a number of days, like
    /// [`try_days`](Self::try_days).
    ///
    /// # Panics
    /// Panics when the duration would be out of bounds.
    pub const fn days(days: i64) -> TimeDelta {
        match TimeDelta::try_days(days) {
            Some(d) => d,
            None => panic!("TimeDelta::days out of bounds"),
        }
    }

    /// Builds a duration from a number of hours. Returns `None` on
    /// overflow.
    pub const fn try_hours(hours: i64) -> Option<TimeDelta> {
        match hours.checked_mul(SECS_PER_HOUR) {
            Some(secs) => TimeDelta::try_seconds(secs),
            None => None,
        }
    }

    /// Builds a duration from a number of hours, like
    /// [`try_hours`](Self::try_hours).
    ///
    /// # Panics
    /// Panics when the duration would be out of bounds.
    pub const fn hours(hours: i64) -> TimeDelta {
        match TimeDelta::try_hours(hours) {
            Some(d) => d,
            None => panic!("TimeDelta::hours out of bounds"),
        }
    }

    /// Builds a duration from a number of minutes. Returns `None` on
    /// overflow.
    pub const fn try_minutes(minutes: i64) -> Option<TimeDelta> {
        match minutes.checked_mul(SECS_PER_MINUTE) {
            Some(secs) => TimeDelta::try_seconds(secs),
            None => None,
        }
    }

    /// Builds a duration from a number of minutes, like
    /// [`try_minutes`](Self::try_minutes).
    ///
    /// # Panics
    /// Panics when the duration would be out of bounds.
    pub const fn minutes(minutes: i64) -> TimeDelta {
        match TimeDelta::try_minutes(minutes) {
            Some(d) => d,
            None => panic!("TimeDelta::minutes out of bounds"),
        }
    }

    /// Builds a duration from a whole number of seconds. Returns `None` on
    /// overflow.
    pub const fn try_seconds(secs: i64) -> Option<TimeDelta> {
        TimeDelta::new(secs, 0)
    }

    /// Builds a duration from a whole number of seconds, like
    /// [`try_seconds`](Self::try_seconds).
    ///
    /// # Panics
    /// Panics when `secs` is outside the representable range (beyond
    /// `i64::MAX / 1000` in absolute value).
    pub const fn seconds(secs: i64) -> TimeDelta {
        match TimeDelta::try_seconds(secs) {
            Some(d) => d,
            None => panic!("TimeDelta::seconds out of bounds"),
        }
    }

    /// Builds a duration from a (signed) number of milliseconds. Returns
    /// `None` on overflow.
    pub const fn try_milliseconds(millis: i64) -> Option<TimeDelta> {
        // `MAX` is aligned to `i64::MAX` milliseconds, so only the lower
        // bound needs an explicit check here.
        if millis < -i64::MAX {
            return None;
        }
        let secs = div_floor(millis, MILLIS_PER_SEC);
        // `secs * MILLIS_PER_SEC` cannot use plain `*`: at the extreme lower
        // bound (`millis` close to `-i64::MAX`, `secs` close to
        // `TimeDelta::MIN.secs`), that intermediate product alone falls
        // just *below* `i64::MIN`, even though the final `rem` (always in
        // `0..1000`) and the resulting `TimeDelta` are both perfectly
        // valid. Found via `cargo test` (2026-07-13):
        // `try_milliseconds(-i64::MAX)` -- which per `TimeDelta::MIN`'s own
        // doc comment must succeed -- panicked with "attempt to multiply
        // with overflow" instead. `wrapping_mul`/`wrapping_sub` compute
        // exactly (mod 2^64), and since the true mathematical result is
        // guaranteed in-range, the wrapped computation yields the exact
        // right answer.
        let rem = millis.wrapping_sub(secs.wrapping_mul(MILLIS_PER_SEC)); // 0..1000
        Some(TimeDelta { secs, nanos: (rem as i32) * NANOS_PER_MILLI })
    }

    /// Builds a duration from a (signed) number of milliseconds, like
    /// [`try_milliseconds`](Self::try_milliseconds).
    ///
    /// # Panics
    /// Panics when the duration would be out of bounds.
    pub const fn milliseconds(millis: i64) -> TimeDelta {
        match TimeDelta::try_milliseconds(millis) {
            Some(d) => d,
            None => panic!("TimeDelta::milliseconds out of bounds"),
        }
    }

    /// Builds a duration from a (signed) number of microseconds. This can
    /// never overflow the representable range, so this is infallible.
    pub const fn microseconds(micros: i64) -> TimeDelta {
        let secs = div_floor(micros, MICROS_PER_SEC);
        // See the comment in `try_milliseconds` above: `secs * MICROS_PER_SEC`
        // can itself fall outside `i64`'s range at the extremes (e.g.
        // `micros == i64::MIN`) even though `rem` and the resulting
        // `TimeDelta` are always valid, so wrapping arithmetic is required.
        let rem = micros.wrapping_sub(secs.wrapping_mul(MICROS_PER_SEC)); // 0..1_000_000
        TimeDelta { secs, nanos: (rem as i32) * NANOS_PER_MICRO }
    }

    /// Builds a duration from a (signed) number of nanoseconds. This can
    /// never overflow the representable range, so this is infallible.
    pub const fn nanoseconds(nanos_in: i64) -> TimeDelta {
        let secs = div_floor(nanos_in, NANOS_PER_SEC as i64);
        // See the comment in `try_milliseconds` above.
        let rem = nanos_in.wrapping_sub(secs.wrapping_mul(NANOS_PER_SEC as i64)); // 0..NANOS_PER_SEC
        TimeDelta { secs, nanos: rem as i32 }
    }

    /// Whole number of weeks (truncated toward zero).
    pub const fn num_weeks(&self) -> i64 {
        self.num_days() / 7
    }

    /// Whole number of days (truncated toward zero).
    pub const fn num_days(&self) -> i64 {
        self.num_seconds() / SECS_PER_DAY
    }

    /// Whole number of hours (truncated toward zero).
    pub const fn num_hours(&self) -> i64 {
        self.num_seconds() / SECS_PER_HOUR
    }

    /// Whole number of minutes (truncated toward zero).
    pub const fn num_minutes(&self) -> i64 {
        self.num_seconds() / SECS_PER_MINUTE
    }

    /// Whole number of seconds (truncated toward zero), like `chrono`.
    pub const fn num_seconds(&self) -> i64 {
        if self.secs < 0 && self.nanos > 0 {
            self.secs + 1
        } else {
            self.secs
        }
    }

    /// The duration as a fractional number of seconds.
    pub fn as_seconds_f64(self) -> f64 {
        self.secs as f64 + self.nanos as f64 / NANOS_PER_SEC as f64
    }

    /// The duration as a fractional number of seconds.
    pub fn as_seconds_f32(self) -> f32 {
        self.secs as f32 + self.nanos as f32 / NANOS_PER_SEC as f32
    }

    /// Total number of milliseconds. Never overflows, thanks to the bounds
    /// enforced by the constructors.
    pub const fn num_milliseconds(&self) -> i64 {
        self.num_seconds() * MILLIS_PER_SEC + (self.subsec_nanos() / NANOS_PER_MILLI) as i64
    }

    /// Number of milliseconds in the fractional part of the duration (i.e.
    /// `subsec_millis() + num_seconds() * 1_000 == num_milliseconds()`).
    pub const fn subsec_millis(&self) -> i32 {
        self.subsec_nanos() / NANOS_PER_MILLI
    }

    /// Total number of microseconds, or `None` on overflow.
    pub const fn num_microseconds(&self) -> Option<i64> {
        let secs_part = match self.num_seconds().checked_mul(MICROS_PER_SEC) {
            Some(v) => v,
            None => return None,
        };
        secs_part.checked_add((self.subsec_nanos() / NANOS_PER_MICRO) as i64)
    }

    /// Number of microseconds in the fractional part of the duration (i.e.
    /// `subsec_micros() + num_seconds() * 1_000_000 == num_microseconds()`).
    pub const fn subsec_micros(&self) -> i32 {
        self.subsec_nanos() / NANOS_PER_MICRO
    }

    /// Total number of nanoseconds, or `None` on overflow.
    pub const fn num_nanoseconds(&self) -> Option<i64> {
        let secs_part = match self.num_seconds().checked_mul(NANOS_PER_SEC as i64) {
            Some(v) => v,
            None => return None,
        };
        secs_part.checked_add(self.subsec_nanos() as i64)
    }

    /// The fractional part of the duration, in nanoseconds. Has the same
    /// sign as the duration as a whole (unlike the internal `nanos` field,
    /// which is always non-negative): for a duration of -0.5s this returns
    /// -500_000_000, consistent with [`num_seconds`](Self::num_seconds)
    /// (which truncates towards zero, so returns `0` for -0.5s).
    pub const fn subsec_nanos(&self) -> i32 {
        if self.secs < 0 && self.nanos > 0 {
            self.nanos - NANOS_PER_SEC
        } else {
            self.nanos
        }
    }

    /// Negates the duration. Same result as the `Neg` operator, but usable
    /// in a `const` context (operator traits cannot be `const` in stable
    /// Rust).
    pub(crate) const fn neg_const(self) -> TimeDelta {
        if self.nanos == 0 {
            TimeDelta { secs: -self.secs, nanos: 0 }
        } else {
            TimeDelta { secs: -self.secs - 1, nanos: NANOS_PER_SEC - self.nanos }
        }
    }

    /// Checked addition: returns `None` on capacity overflow instead of
    /// panicking.
    pub const fn checked_add(&self, rhs: &TimeDelta) -> Option<TimeDelta> {
        let mut secs = match self.secs.checked_add(rhs.secs) {
            Some(v) => v,
            None => return None,
        };
        let mut nanos = self.nanos + rhs.nanos;
        if nanos >= NANOS_PER_SEC {
            nanos -= NANOS_PER_SEC;
            secs = match secs.checked_add(1) {
                Some(v) => v,
                None => return None,
            };
        }
        TimeDelta::new(secs, nanos as u32)
    }

    /// Checked subtraction: returns `None` on capacity overflow instead of
    /// panicking.
    pub const fn checked_sub(&self, rhs: &TimeDelta) -> Option<TimeDelta> {
        let mut secs = match self.secs.checked_sub(rhs.secs) {
            Some(v) => v,
            None => return None,
        };
        let mut nanos = self.nanos - rhs.nanos;
        if nanos < 0 {
            nanos += NANOS_PER_SEC;
            secs = match secs.checked_sub(1) {
                Some(v) => v,
                None => return None,
            };
        }
        TimeDelta::new(secs, nanos as u32)
    }

    /// Multiplies the duration by an `i32`, returning `None` if the result
    /// would not fit in an `i64` number of seconds.
    ///
    /// Note: like `chrono`, the check performed here is against the range
    /// of `i64` seconds, not against [`TimeDelta::MIN`]/[`TimeDelta::MAX`]
    /// (which are narrower, being expressed in milliseconds); this is a
    /// deliberate compatibility choice rather than an oversight.
    pub const fn checked_mul(&self, rhs: i32) -> Option<TimeDelta> {
        let total_nanos = self.nanos as i64 * rhs as i64;
        let extra_secs = div_floor(total_nanos, NANOS_PER_SEC as i64);
        let nanos = (total_nanos - extra_secs * NANOS_PER_SEC as i64) as i32;
        let secs: i128 = self.secs as i128 * rhs as i128 + extra_secs as i128;
        if secs <= i64::MIN as i128 || secs >= i64::MAX as i128 {
            return None;
        }
        Some(TimeDelta { secs: secs as i64, nanos })
    }

    /// Divides the duration by an `i32`, returning `None` if `rhs` is zero.
    pub const fn checked_div(&self, rhs: i32) -> Option<TimeDelta> {
        if rhs == 0 {
            return None;
        }
        let secs = self.secs / rhs as i64;
        let carry = self.secs % rhs as i64;
        let extra_nanos = carry * NANOS_PER_SEC as i64 / rhs as i64;
        let nanos = self.nanos / rhs + extra_nanos as i32;
        let (secs, nanos) = if nanos < 0 {
            (secs - 1, nanos + NANOS_PER_SEC)
        } else if nanos >= NANOS_PER_SEC {
            (secs + 1, nanos - NANOS_PER_SEC)
        } else {
            (secs, nanos)
        };
        Some(TimeDelta { secs, nanos })
    }

    /// The duration as an absolute (non-negative) value.
    pub const fn abs(&self) -> TimeDelta {
        if self.secs < 0 && self.nanos != 0 {
            TimeDelta { secs: (self.secs + 1).abs(), nanos: NANOS_PER_SEC - self.nanos }
        } else {
            TimeDelta { secs: self.secs.abs(), nanos: self.nanos }
        }
    }

    /// Creates a `TimeDelta` from a `core::time::Duration` (unsigned).
    /// Returns `Err` if the source value is larger than
    /// [`TimeDelta::MAX`].
    pub const fn from_std(duration: core::time::Duration) -> Result<TimeDelta, OutOfRangeError> {
        if duration.as_secs() > Self::MAX.secs as u64 {
            return Err(OutOfRangeError(()));
        }
        match TimeDelta::new(duration.as_secs() as i64, duration.subsec_nanos()) {
            Some(d) => Ok(d),
            None => Err(OutOfRangeError(())),
        }
    }

    /// Creates a `core::time::Duration` from a `TimeDelta`. Returns `Err`
    /// if `self` is negative, since `core::time::Duration` is unsigned.
    pub const fn to_std(&self) -> Result<core::time::Duration, OutOfRangeError> {
        if self.secs < 0 {
            return Err(OutOfRangeError(()));
        }
        Ok(core::time::Duration::new(self.secs as u64, self.nanos as u32))
    }
}

impl Neg for TimeDelta {
    type Output = TimeDelta;
    fn neg(self) -> TimeDelta {
        self.neg_const()
    }
}

impl Add for TimeDelta {
    type Output = TimeDelta;
    fn add(self, rhs: TimeDelta) -> TimeDelta {
        self.checked_add(&rhs)
            .expect("`TimeDelta + TimeDelta` overflowed")
    }
}

impl Sub for TimeDelta {
    type Output = TimeDelta;
    fn sub(self, rhs: TimeDelta) -> TimeDelta {
        self.checked_sub(&rhs)
            .expect("`TimeDelta - TimeDelta` overflowed")
    }
}

impl AddAssign for TimeDelta {
    fn add_assign(&mut self, rhs: TimeDelta) {
        *self = *self + rhs;
    }
}

impl SubAssign for TimeDelta {
    fn sub_assign(&mut self, rhs: TimeDelta) {
        *self = *self - rhs;
    }
}

impl Mul<i32> for TimeDelta {
    type Output = TimeDelta;
    fn mul(self, rhs: i32) -> TimeDelta {
        self.checked_mul(rhs).expect("`TimeDelta * i32` overflowed")
    }
}

impl Div<i32> for TimeDelta {
    type Output = TimeDelta;
    fn div(self, rhs: i32) -> TimeDelta {
        self.checked_div(rhs).expect("`i32` is zero")
    }
}

impl<'a> core::iter::Sum<&'a TimeDelta> for TimeDelta {
    fn sum<I: Iterator<Item = &'a TimeDelta>>(iter: I) -> TimeDelta {
        iter.fold(TimeDelta::zero(), |acc, x| acc + *x)
    }
}

impl core::iter::Sum<TimeDelta> for TimeDelta {
    fn sum<I: Iterator<Item = TimeDelta>>(iter: I) -> TimeDelta {
        iter.fold(TimeDelta::zero(), |acc, x| acc + x)
    }
}

/// Formats the duration using the [ISO 8601] duration format.
///
/// [ISO 8601]: https://en.wikipedia.org/wiki/ISO_8601#Durations
impl fmt::Display for TimeDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Technically speaking, a negative duration is not valid ISO 8601,
        // but we print it anyway (with a leading `-`).
        let (abs, sign) = if self.secs < 0 { (-*self, "-") } else { (*self, "") };

        write!(f, "{sign}P")?;
        if abs.secs == 0 && abs.nanos == 0 {
            return f.write_str("0D");
        }

        write!(f, "T{}", abs.secs)?;

        if abs.nanos > 0 {
            // Number of significant digits, after stripping trailing
            // zeros.
            let mut figures = 9usize;
            let mut fraction_digits = abs.nanos;
            loop {
                let div = fraction_digits / 10;
                let last_digit = fraction_digits % 10;
                if last_digit != 0 {
                    break;
                }
                fraction_digits = div;
                figures -= 1;
            }
            write!(f, ".{fraction_digits:0figures$}")?;
        }
        f.write_str("S")
    }
}

/// Alias of [`TimeDelta`], kept for compatibility with `chrono::Duration`
/// (the historical name of this type).
pub type Duration = TimeDelta;

/// `serde` support: serializes as a `(secs, nanos)` tuple, like `chrono`.
#[cfg(feature = "serde")]
mod serde_impl {
    use super::TimeDelta;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    impl Serialize for TimeDelta {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            <(i64, i32) as Serialize>::serialize(&(self.secs, self.nanos), serializer)
        }
    }

    impl<'de> Deserialize<'de> for TimeDelta {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let (secs, nanos) = <(i64, i32) as Deserialize>::deserialize(deserializer)?;
            TimeDelta::new(secs, nanos as u32).ok_or(Error::custom("TimeDelta out of bounds"))
        }
    }
}

/// Error returned by [`TimeDelta::from_std`]/[`TimeDelta::to_std`] when the
/// source value is out of range for the target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutOfRangeError(());

impl fmt::Display for OutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("source duration value is out of range for the target type")
    }
}

impl core::error::Error for OutOfRangeError {}

/// An increment expressed in days, used with
/// [`NaiveDate::checked_add_days`](crate::NaiveDate::checked_add_days) and
/// related methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Days(pub(crate) u64);

impl Days {
    pub const fn new(num_days: u64) -> Self {
        Days(num_days)
    }
}

/// An increment expressed in months (calendar arithmetic, distinct from
/// [`TimeDelta`]), used with
/// [`NaiveDate::checked_add_months`](crate::NaiveDate::checked_add_months).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Months(pub(crate) u32);

impl Months {
    pub const fn new(num_months: u32) -> Self {
        Months(num_months)
    }

    pub const fn as_u32(&self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_validity_bounds() {
        assert!(TimeDelta::new(0, 0).is_some());
        assert!(TimeDelta::new(0, 999_999_999).is_some());
        assert!(TimeDelta::new(0, 1_000_000_000).is_none());
        assert_eq!(TimeDelta::new(TimeDelta::MAX.secs, TimeDelta::MAX.nanos as u32), Some(TimeDelta::MAX));
        assert_eq!(TimeDelta::new(TimeDelta::MAX.secs, TimeDelta::MAX.nanos as u32 + 1), None);
        assert_eq!(TimeDelta::new(TimeDelta::MAX.secs + 1, 0), None);
        assert_eq!(TimeDelta::new(TimeDelta::MIN.secs, TimeDelta::MIN.nanos as u32), Some(TimeDelta::MIN));
        assert_eq!(TimeDelta::new(TimeDelta::MIN.secs, TimeDelta::MIN.nanos as u32 - 1), None);
        assert_eq!(TimeDelta::new(TimeDelta::MIN.secs - 1, 0), None);
    }

    #[test]
    fn zero_is_zero() {
        assert!(TimeDelta::zero().is_zero());
        assert!(!TimeDelta::seconds(1).is_zero());
        assert!(!TimeDelta::seconds(-1).is_zero());
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(TimeDelta::default(), TimeDelta::zero());
    }

    #[test]
    fn calendar_unit_constructors_convert_to_seconds() {
        assert_eq!(TimeDelta::weeks(1).num_seconds(), 7 * 24 * 3600);
        assert_eq!(TimeDelta::days(1).num_seconds(), 24 * 3600);
        assert_eq!(TimeDelta::hours(1).num_seconds(), 3600);
        assert_eq!(TimeDelta::minutes(1).num_seconds(), 60);
        assert_eq!(TimeDelta::seconds(1).num_seconds(), 1);
    }

    #[test]
    fn negative_calendar_units_work_too() {
        assert_eq!(TimeDelta::days(-1).num_seconds(), -24 * 3600);
        assert_eq!(TimeDelta::hours(-1).num_hours(), -1);
    }

    #[test]
    #[should_panic]
    fn weeks_panics_on_overflow() {
        let _ = TimeDelta::weeks(i64::MAX);
    }

    #[test]
    fn try_weeks_returns_none_on_overflow() {
        assert!(TimeDelta::try_weeks(i64::MAX).is_none());
        assert!(TimeDelta::try_weeks(1).is_some());
    }

    #[test]
    fn milliseconds_constructor() {
        assert_eq!(TimeDelta::milliseconds(1500).num_milliseconds(), 1500);
        assert_eq!(TimeDelta::milliseconds(-1500).num_milliseconds(), -1500);
        assert_eq!(TimeDelta::milliseconds(0).num_milliseconds(), 0);
    }

    #[test]
    fn try_milliseconds_rejects_below_min() {
        // `-i64::MAX` milliseconds is *exactly* `TimeDelta::MIN` (see its
        // doc comment), so this must succeed, not panic. Regression test
        // for a real bug found via `cargo test` (2026-07-13): the internal
        // `secs * MILLIS_PER_SEC` computation overflowed `i64` at this
        // exact extreme, even though the final result is valid.
        assert_eq!(TimeDelta::try_milliseconds(-i64::MAX), Some(TimeDelta::MIN));
        assert!(TimeDelta::try_milliseconds(i64::MIN).is_none());
    }

    #[test]
    fn microseconds_and_nanoseconds_are_infallible_round_trips() {
        assert_eq!(TimeDelta::microseconds(1_500_000).num_microseconds(), Some(1_500_000));
        assert_eq!(TimeDelta::microseconds(-1_500_000).num_microseconds(), Some(-1_500_000));
        assert_eq!(TimeDelta::nanoseconds(1_500_000_000).num_nanoseconds(), Some(1_500_000_000));
        assert_eq!(TimeDelta::nanoseconds(-1_500_000_000).num_nanoseconds(), Some(-1_500_000_000));
    }

    #[test]
    fn microseconds_and_nanoseconds_do_not_panic_at_i64_extremes() {
        // Same bug class as `try_milliseconds_rejects_below_min`: these two
        // constructors are documented as infallible (the *resulting*
        // `TimeDelta` always fits), but their internal `secs * SCALE`
        // computation must not itself overflow `i64` at `i64::MIN`/`MAX`.
        assert_eq!(TimeDelta::microseconds(i64::MIN).num_microseconds(), Some(i64::MIN));
        assert_eq!(TimeDelta::microseconds(i64::MAX).num_microseconds(), Some(i64::MAX));
        assert_eq!(TimeDelta::nanoseconds(i64::MIN).num_nanoseconds(), Some(i64::MIN));
        assert_eq!(TimeDelta::nanoseconds(i64::MAX).num_nanoseconds(), Some(i64::MAX));
    }

    #[test]
    fn num_seconds_truncates_toward_zero_for_negative_subsecond() {
        let d = TimeDelta::milliseconds(-500);
        assert_eq!(d.num_seconds(), 0);
        assert_eq!(d.num_milliseconds(), -500);

        let d2 = TimeDelta::milliseconds(-1500);
        assert_eq!(d2.num_seconds(), -1);
        assert_eq!(d2.num_milliseconds(), -1500);
    }

    #[test]
    fn subsec_nanos_has_same_sign_as_whole_duration() {
        assert_eq!(TimeDelta::milliseconds(-500).subsec_nanos(), -500_000_000);
        assert_eq!(TimeDelta::milliseconds(500).subsec_nanos(), 500_000_000);
        assert_eq!(TimeDelta::milliseconds(-1500).subsec_nanos(), -500_000_000);
    }

    #[test]
    fn num_weeks_truncates_toward_zero() {
        assert_eq!(TimeDelta::days(14).num_weeks(), 2);
        assert_eq!(TimeDelta::days(13).num_weeks(), 1);
        assert_eq!(TimeDelta::days(-13).num_weeks(), -1);
    }

    #[test]
    fn num_nanoseconds_and_microseconds_overflow_return_none_for_max() {
        assert_eq!(TimeDelta::MAX.num_nanoseconds(), None);
        assert_eq!(TimeDelta::MAX.num_microseconds(), None);
    }

    #[test]
    fn num_milliseconds_never_overflows_for_max_and_min() {
        assert_eq!(TimeDelta::MAX.num_milliseconds(), i64::MAX);
        assert_eq!(TimeDelta::MIN.num_milliseconds(), -i64::MAX);
    }

    #[test]
    fn as_seconds_f64_and_f32() {
        let d = TimeDelta::milliseconds(1500);
        assert!((d.as_seconds_f64() - 1.5).abs() < 1e-9);
        assert!((d.as_seconds_f32() - 1.5).abs() < 1e-6);
        let neg = TimeDelta::milliseconds(-1500);
        assert!((neg.as_seconds_f64() - (-1.5)).abs() < 1e-9);
    }

    #[test]
    fn checked_add_overflows_past_max() {
        assert!(TimeDelta::MAX.checked_add(&TimeDelta::nanoseconds(1)).is_none());
        assert_eq!(TimeDelta::MAX.checked_add(&TimeDelta::zero()), Some(TimeDelta::MAX));
    }

    #[test]
    fn checked_sub_overflows_past_min() {
        assert!(TimeDelta::MIN.checked_sub(&TimeDelta::nanoseconds(1)).is_none());
        assert_eq!(TimeDelta::MIN.checked_sub(&TimeDelta::zero()), Some(TimeDelta::MIN));
    }

    #[test]
    #[should_panic]
    fn add_operator_panics_on_overflow() {
        let _ = TimeDelta::MAX + TimeDelta::nanoseconds(1);
    }

    #[test]
    fn checked_mul_basic() {
        assert_eq!(TimeDelta::seconds(3).checked_mul(2), Some(TimeDelta::seconds(6)));
        assert_eq!(TimeDelta::seconds(3).checked_mul(-2), Some(TimeDelta::seconds(-6)));
    }

    #[test]
    fn checked_div_basic_and_by_zero() {
        assert_eq!(TimeDelta::seconds(6).checked_div(2), Some(TimeDelta::seconds(3)));
        assert_eq!(TimeDelta::seconds(6).checked_div(0), None);
        assert_eq!(TimeDelta::seconds(-6).checked_div(2), Some(TimeDelta::seconds(-3)));
    }

    #[test]
    #[should_panic]
    fn div_operator_panics_on_division_by_zero() {
        let _ = TimeDelta::seconds(1) / 0;
    }

    #[test]
    fn abs_is_always_non_negative() {
        assert_eq!(TimeDelta::seconds(-5).abs(), TimeDelta::seconds(5));
        assert_eq!(TimeDelta::seconds(5).abs(), TimeDelta::seconds(5));
        assert_eq!(TimeDelta::milliseconds(-1500).abs(), TimeDelta::milliseconds(1500));
        assert_eq!(TimeDelta::zero().abs(), TimeDelta::zero());
    }

    #[test]
    fn neg_flips_sign() {
        assert_eq!(-TimeDelta::seconds(5), TimeDelta::seconds(-5));
        assert_eq!(-TimeDelta::seconds(-5), TimeDelta::seconds(5));
        assert_eq!(-TimeDelta::zero(), TimeDelta::zero());
        assert_eq!(-TimeDelta::milliseconds(1500), TimeDelta::milliseconds(-1500));
    }

    #[test]
    fn from_std_and_to_std_round_trip() {
        let std_dur = core::time::Duration::new(5, 500_000_000);
        let td = TimeDelta::from_std(std_dur).unwrap();
        assert_eq!(td.num_seconds(), 5);
        assert_eq!(td.subsec_nanos(), 500_000_000);
        let back = td.to_std().unwrap();
        assert_eq!(back, std_dur);
    }

    #[test]
    fn to_std_rejects_negative_duration() {
        assert!(TimeDelta::seconds(-1).to_std().is_err());
    }

    #[test]
    fn from_std_rejects_too_large_duration() {
        let too_big = core::time::Duration::new(u64::MAX, 0);
        assert!(TimeDelta::from_std(too_big).is_err());
    }

    #[test]
    fn display_zero_duration() {
        assert_eq!(TimeDelta::zero().to_string(), "P0D");
    }

    #[test]
    fn display_positive_whole_seconds() {
        assert_eq!(TimeDelta::seconds(5).to_string(), "PT5S");
    }

    #[test]
    fn display_negative_duration_has_leading_minus() {
        assert_eq!(TimeDelta::seconds(-5).to_string(), "-PT5S");
    }

    #[test]
    fn display_strips_trailing_zero_fraction_digits() {
        assert_eq!(TimeDelta::milliseconds(1500).to_string(), "PT1.5S");
        assert_eq!(TimeDelta::microseconds(1_000_500).to_string(), "PT1.0005S");
        assert_eq!(TimeDelta::nanoseconds(1_000_000_001).to_string(), "PT1.000000001S");
    }

    #[test]
    fn ordering_matches_actual_duration_length() {
        assert!(TimeDelta::seconds(-1) < TimeDelta::seconds(0));
        assert!(TimeDelta::seconds(0) < TimeDelta::seconds(1));
        assert!(TimeDelta::milliseconds(-500) < TimeDelta::milliseconds(500));
        assert!(TimeDelta::MIN < TimeDelta::MAX);
    }

    #[test]
    fn sum_over_iterator_of_timedeltas() {
        let deltas = [TimeDelta::seconds(1), TimeDelta::seconds(2), TimeDelta::seconds(3)];
        let total: TimeDelta = deltas.iter().sum();
        assert_eq!(total, TimeDelta::seconds(6));
        let total_owned: TimeDelta = deltas.into_iter().sum();
        assert_eq!(total_owned, TimeDelta::seconds(6));
    }

    #[test]
    fn add_assign_and_sub_assign() {
        let mut d = TimeDelta::seconds(10);
        d += TimeDelta::seconds(5);
        assert_eq!(d, TimeDelta::seconds(15));
        d -= TimeDelta::seconds(20);
        assert_eq!(d, TimeDelta::seconds(-5));
    }

    #[test]
    fn days_and_months_constructors() {
        assert_eq!(Days::new(5).0, 5);
        assert_eq!(Months::new(3).as_u32(), 3);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip_as_tuple() {
        let d = TimeDelta::new(12345, 6789).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "[12345,6789]");
        let back: TimeDelta = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_deserialize_rejects_out_of_range_tuple() {
        let json = format!("[{},0]", i64::MAX);
        let result: Result<TimeDelta, _> = serde_json::from_str(&json);
        assert!(result.is_err());
    }
}
