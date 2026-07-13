//! `NaiveTime`: a time of day (hour, minute, second, nanosecond), without
//! an associated date or time zone.
//!
//! ## Leap seconds
//!
//! Like `chrono`, a `NaiveTime` can represent a leap second: the fractional
//! part (`frac`/`nanosecond`) is allowed to go from 1,000,000,000 up to
//! 1,999,999,999 when the second is 59, to mean "the leap second that
//! follows this whole second". Arithmetic (`overflowing_add_signed`,
//! `overflowing_sub_signed`, `signed_duration_since`) treats a value as an
//! ordinary second unless one of the operands is itself a leap second, in
//! which case it is counted once. This mirrors `chrono`'s documented
//! behaviour: it does not track real-world leap seconds, it only avoids
//! losing information when one is explicitly constructed.

use crate::duration::Duration;
use crate::format::{
    parse, parse_and_remainder, DelayedFormat, Fixed, Item, Numeric, Pad, ParseError, ParseResult,
    Parsed, StrftimeItems,
};
use crate::offset::FixedOffset;
use crate::traits::Timelike;
use core::borrow::Borrow;
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

const NANOS_PER_SEC: u32 = 1_000_000_000;

/// A time of day, with nanosecond precision and optional leap-second
/// representation, but no associated date or time zone.
///
/// API aligned with `chrono::NaiveTime`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct NaiveTime {
    /// Non-leap seconds since midnight, in `0..86_400`.
    secs: u32,
    /// Nanoseconds since `secs`, in `0..1_000_000_000`, or
    /// `1_000_000_000..2_000_000_000` to represent a leap second (only
    /// valid when `secs % 60 == 59`).
    frac: u32,
}

/// `arbitrary` support: picks minutes/seconds/nanoseconds independently
/// (occasionally producing a leap second), mirroring chrono's own strategy.
#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for NaiveTime {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<NaiveTime> {
        let mins = u.int_in_range(0..=1439)?;
        let mut secs = u.int_in_range(0..=60)?;
        let mut nano = u.int_in_range(0..=999_999_999)?;
        if secs == 60 {
            secs = 59;
            nano += NANOS_PER_SEC;
        }
        NaiveTime::from_num_seconds_from_midnight_opt(mins * 60 + secs, nano)
            .ok_or(arbitrary::Error::IncorrectFormat)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for NaiveTime {
    fn format(&self, fmt: defmt::Formatter) {
        let (hour, min, sec) = self.hms();
        let (sec, nano) = if self.frac >= NANOS_PER_SEC {
            (sec + 1, self.frac - NANOS_PER_SEC)
        } else {
            (sec, self.frac)
        };
        defmt::write!(fmt, "{:02}:{:02}:{:02}", hour, min, sec);
        if nano == 0 {
        } else if nano % 1_000_000 == 0 {
            defmt::write!(fmt, ".{:03}", nano / 1_000_000);
        } else if nano % 1_000 == 0 {
            defmt::write!(fmt, ".{:06}", nano / 1_000);
        } else {
            defmt::write!(fmt, ".{:09}", nano);
        }
    }
}

impl NaiveTime {
    /// Midnight, the earliest representable time.
    pub const MIN: NaiveTime = NaiveTime { secs: 0, frac: 0 };

    /// The latest representable non-leap time (23:59:59.999999999). Not
    /// public, matching `chrono`: a caller has no real use for this value,
    /// since `NaiveTime` arithmetic wraps around a single day.
    pub(crate) const MAX: NaiveTime =
        NaiveTime { secs: 23 * 3600 + 59 * 60 + 59, frac: 999_999_999 };

    /// Builds a time from hour, minute and second. No leap second is
    /// representable here; use the `_milli`/`_micro`/`_nano` variants for
    /// that. Returns `None` on invalid input.
    pub const fn from_hms_opt(hour: u32, min: u32, sec: u32) -> Option<NaiveTime> {
        NaiveTime::from_hms_nano_opt(hour, min, sec, 0)
    }

    /// Builds a time from hour, minute and second, like
    /// [`from_hms_opt`](Self::from_hms_opt).
    ///
    /// # Panics
    /// Panics on invalid input.
    #[deprecated(note = "use `from_hms_opt()` instead")]
    pub fn from_hms(hour: u32, min: u32, sec: u32) -> NaiveTime {
        Self::from_hms_opt(hour, min, sec).expect("invalid time")
    }

    /// Builds a time from hour, minute, second and millisecond. `milli` may
    /// exceed 1,000 to represent a leap second (only when `sec == 59`).
    /// Returns `None` on invalid input.
    pub const fn from_hms_milli_opt(hour: u32, min: u32, sec: u32, milli: u32) -> Option<NaiveTime> {
        let nano = match milli.checked_mul(1_000_000) {
            Some(v) => v,
            None => return None,
        };
        NaiveTime::from_hms_nano_opt(hour, min, sec, nano)
    }

    /// Builds a time from hour, minute, second and millisecond, like
    /// [`from_hms_milli_opt`](Self::from_hms_milli_opt).
    ///
    /// # Panics
    /// Panics on invalid input.
    #[deprecated(note = "use `from_hms_milli_opt()` instead")]
    pub fn from_hms_milli(hour: u32, min: u32, sec: u32, milli: u32) -> NaiveTime {
        Self::from_hms_milli_opt(hour, min, sec, milli).expect("invalid time")
    }

    /// Builds a time from hour, minute, second and microsecond. `micro` may
    /// exceed 1,000,000 to represent a leap second (only when `sec == 59`).
    /// Returns `None` on invalid input.
    pub const fn from_hms_micro_opt(hour: u32, min: u32, sec: u32, micro: u32) -> Option<NaiveTime> {
        let nano = match micro.checked_mul(1_000) {
            Some(v) => v,
            None => return None,
        };
        NaiveTime::from_hms_nano_opt(hour, min, sec, nano)
    }

    /// Builds a time from hour, minute, second and microsecond, like
    /// [`from_hms_micro_opt`](Self::from_hms_micro_opt).
    ///
    /// # Panics
    /// Panics on invalid input.
    #[deprecated(note = "use `from_hms_micro_opt()` instead")]
    pub fn from_hms_micro(hour: u32, min: u32, sec: u32, micro: u32) -> NaiveTime {
        Self::from_hms_micro_opt(hour, min, sec, micro).expect("invalid time")
    }

    /// Builds a time from hour, minute, second and nanosecond. `nano` may
    /// exceed 1,000,000,000 (up to 1,999,999,999) to represent a leap
    /// second (only when `sec == 59`). Returns `None` on invalid input.
    pub const fn from_hms_nano_opt(hour: u32, min: u32, sec: u32, nano: u32) -> Option<NaiveTime> {
        if hour >= 24
            || min >= 60
            || sec >= 60
            || (nano >= NANOS_PER_SEC && sec != 59)
            || nano >= 2 * NANOS_PER_SEC
        {
            return None;
        }
        let secs = hour * 3600 + min * 60 + sec;
        Some(NaiveTime { secs, frac: nano })
    }

    /// Builds a time from hour, minute, second and nanosecond, like
    /// [`from_hms_nano_opt`](Self::from_hms_nano_opt).
    ///
    /// # Panics
    /// Panics on invalid input.
    #[deprecated(note = "use `from_hms_nano_opt()` instead")]
    pub fn from_hms_nano(hour: u32, min: u32, sec: u32, nano: u32) -> NaiveTime {
        Self::from_hms_nano_opt(hour, min, sec, nano).expect("invalid time")
    }

    /// Builds a time from the number of non-leap seconds since midnight and
    /// a nanosecond remainder. `nano` may exceed 1,000,000,000 to represent
    /// a leap second (only when `secs % 60 == 59`). Returns `None` on
    /// invalid input.
    pub const fn from_num_seconds_from_midnight_opt(secs: u32, nano: u32) -> Option<NaiveTime> {
        if secs >= 86_400 || nano >= 2 * NANOS_PER_SEC || (nano >= NANOS_PER_SEC && secs % 60 != 59)
        {
            return None;
        }
        Some(NaiveTime { secs, frac: nano })
    }

    /// Builds a time from the number of seconds since midnight, like
    /// [`from_num_seconds_from_midnight_opt`](Self::from_num_seconds_from_midnight_opt).
    ///
    /// # Panics
    /// Panics on invalid input.
    #[deprecated(note = "use `from_num_seconds_from_midnight_opt()` instead")]
    pub fn from_num_seconds_from_midnight(secs: u32, nano: u32) -> NaiveTime {
        Self::from_num_seconds_from_midnight_opt(secs, nano).expect("invalid time")
    }

    pub(crate) fn hms(&self) -> (u32, u32, u32) {
        let sec = self.secs % 60;
        let mins = self.secs / 60;
        let min = mins % 60;
        let hour = mins / 60;
        (hour, min, sec)
    }

    /// Adds a signed [`Duration`], wrapping around a single day. Also
    /// returns the (signed) number of seconds that were carried into, or
    /// out of, the day; a caller adding this to a date can ignore it if it
    /// is `0`.
    ///
    /// Never panics or fails: excess days are simply reported back instead
    /// of being applied to `self`.
    pub const fn overflowing_add_signed(&self, rhs: Duration) -> (NaiveTime, i64) {
        let mut secs = self.secs as i64;
        let mut frac = self.frac as i32;
        let secs_to_add = rhs.num_seconds();
        let frac_to_add = rhs.subsec_nanos();

        // If `self` is a leap second, decide whether `rhs` keeps us within
        // it (in which case we can return immediately) or escapes it (in
        // which case we fold the leap second away and continue below as if
        // there had been no leap second at all).
        if frac >= NANOS_PER_SEC as i32 {
            if secs_to_add > 0 || (frac_to_add > 0 && frac >= 2 * NANOS_PER_SEC as i32 - frac_to_add)
            {
                frac -= NANOS_PER_SEC as i32;
            } else if secs_to_add < 0 {
                frac -= NANOS_PER_SEC as i32;
                secs += 1;
            } else {
                return (NaiveTime { secs: self.secs, frac: (frac + frac_to_add) as u32 }, 0);
            }
        }

        secs += secs_to_add;
        frac += frac_to_add;

        if frac < 0 {
            frac += NANOS_PER_SEC as i32;
            secs -= 1;
        } else if frac >= NANOS_PER_SEC as i32 {
            frac -= NANOS_PER_SEC as i32;
            secs += 1;
        }

        let secs_in_day = secs.rem_euclid(86_400);
        let remaining = secs - secs_in_day;
        (NaiveTime { secs: secs_in_day as u32, frac: frac as u32 }, remaining)
    }

    /// Subtracts a signed [`Duration`], wrapping around a single day. See
    /// [`overflowing_add_signed`](Self::overflowing_add_signed).
    pub const fn overflowing_sub_signed(&self, rhs: Duration) -> (NaiveTime, i64) {
        let (time, carried) = self.overflowing_add_signed(rhs.neg_const());
        (time, -carried)
    }

    /// Signed duration between two times (`self - rhs`), always within +/-
    /// 1 day. Never panics.
    pub const fn signed_duration_since(self, rhs: NaiveTime) -> Duration {
        let mut secs = self.secs as i64 - rhs.secs as i64;
        let frac = self.frac as i64 - rhs.frac as i64;

        // Account for a leap second on either side that hasn't been
        // reflected in `secs` yet.
        if self.secs > rhs.secs && rhs.frac >= NANOS_PER_SEC {
            secs += 1;
        } else if self.secs < rhs.secs && self.frac >= NANOS_PER_SEC {
            secs -= 1;
        }

        let secs_from_frac = crate::calendar::div_floor(frac, NANOS_PER_SEC as i64);
        let frac_rem = (frac - secs_from_frac * NANOS_PER_SEC as i64) as u32;

        match Duration::new(secs + secs_from_frac, frac_rem) {
            Some(d) => d,
            None => panic!("NaiveTime::signed_duration_since: result out of range"),
        }
    }

    /// Non-`Timelike` duplicate of
    /// [`Timelike::num_seconds_from_midnight`], usable from `const fn`
    /// contexts elsewhere in the crate (trait methods cannot be `const`
    /// in stable Rust).
    pub(crate) const fn num_seconds_from_midnight(&self) -> u32 {
        self.secs
    }

    /// Non-`Timelike` duplicate of [`Timelike::nanosecond`], usable from
    /// `const fn` contexts elsewhere in the crate (trait methods cannot
    /// be `const` in stable Rust).
    pub(crate) const fn nanosecond(&self) -> u32 {
        self.frac
    }

    /// Adds a given [`FixedOffset`] to the current time, and returns the
    /// number of days that should be added to a date as a result (`-1`,
    /// `0`, or `1`, since an offset is always less than 24h). Preserves
    /// leap seconds, unlike [`overflowing_add_signed`](Self::overflowing_add_signed).
    pub(crate) const fn overflowing_add_offset(&self, offset: FixedOffset) -> (NaiveTime, i32) {
        let secs = self.secs as i32 + offset.local_minus_utc();
        let days = secs.div_euclid(86_400);
        let secs = secs.rem_euclid(86_400);
        (NaiveTime { secs: secs as u32, frac: self.frac }, days)
    }

    /// Subtracts a given [`FixedOffset`] from the current time. See
    /// [`overflowing_add_offset`](Self::overflowing_add_offset).
    pub(crate) const fn overflowing_sub_offset(&self, offset: FixedOffset) -> (NaiveTime, i32) {
        let secs = self.secs as i32 - offset.local_minus_utc();
        let days = secs.div_euclid(86_400);
        let secs = secs.rem_euclid(86_400);
        (NaiveTime { secs: secs as u32, frac: self.frac }, days)
    }

    /// Parses a `NaiveTime` from a string using a user-specified format. See
    /// the [`crate::format::strftime`] module for the supported escape
    /// sequences.
    pub fn parse_from_str(s: &str, fmt: &str) -> ParseResult<NaiveTime> {
        let mut parsed = Parsed::new();
        parse(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_naive_time()
    }

    /// Parses a `NaiveTime` from a string using a user-specified format,
    /// returning the value and a slice with the remaining, unparsed portion
    /// of the string.
    pub fn parse_and_remainder<'a>(s: &'a str, fmt: &str) -> ParseResult<(NaiveTime, &'a str)> {
        let mut parsed = Parsed::new();
        let remainder = parse_and_remainder(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_naive_time().map(|t| (t, remainder))
    }

    /// Formats the time with the specified formatting items.
    #[must_use]
    pub fn format_with_items<'a, I, B>(&self, items: I) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        DelayedFormat::new(None, Some(*self), items)
    }

    /// Formats the time with the specified format string. See the
    /// [`crate::format::strftime`] module for the supported escape
    /// sequences.
    #[must_use]
    pub fn format<'a>(&self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'a>> {
        self.format_with_items(StrftimeItems::new(fmt))
    }
}

impl Timelike for NaiveTime {
    fn hour(&self) -> u32 {
        self.hms().0
    }

    fn minute(&self) -> u32 {
        self.hms().1
    }

    fn second(&self) -> u32 {
        self.hms().2
    }

    fn nanosecond(&self) -> u32 {
        self.frac
    }

    fn with_hour(&self, hour: u32) -> Option<NaiveTime> {
        if hour >= 24 {
            return None;
        }
        Some(NaiveTime { secs: hour * 3600 + self.secs % 3600, ..*self })
    }

    fn with_minute(&self, min: u32) -> Option<NaiveTime> {
        if min >= 60 {
            return None;
        }
        Some(NaiveTime { secs: self.secs / 3600 * 3600 + min * 60 + self.secs % 60, ..*self })
    }

    fn with_second(&self, sec: u32) -> Option<NaiveTime> {
        if sec >= 60 {
            return None;
        }
        Some(NaiveTime { secs: self.secs / 60 * 60 + sec, ..*self })
    }

    fn with_nanosecond(&self, nano: u32) -> Option<NaiveTime> {
        if nano >= 2 * NANOS_PER_SEC {
            return None;
        }
        Some(NaiveTime { frac: nano, ..*self })
    }

    fn num_seconds_from_midnight(&self) -> u32 {
        self.secs
    }
}

impl Add<Duration> for NaiveTime {
    type Output = NaiveTime;
    fn add(self, rhs: Duration) -> NaiveTime {
        self.overflowing_add_signed(rhs).0
    }
}

impl AddAssign<Duration> for NaiveTime {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub<Duration> for NaiveTime {
    type Output = NaiveTime;
    fn sub(self, rhs: Duration) -> NaiveTime {
        self.overflowing_sub_signed(rhs).0
    }
}

impl SubAssign<Duration> for NaiveTime {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

/// Add a `core::time::Duration` (unsigned) to a `NaiveTime`. Wraps around a
/// single day, like the signed [`Duration`] operators.
impl Add<core::time::Duration> for NaiveTime {
    type Output = NaiveTime;
    fn add(self, rhs: core::time::Duration) -> NaiveTime {
        // Values beyond a couple of days are irrelevant since this wraps;
        // reduce first so the conversion to `Duration` cannot overflow.
        let secs = (rhs.as_secs() % (2 * 86_400)) as i64;
        let d = Duration::new(secs, rhs.subsec_nanos()).expect("duration in range");
        self.overflowing_add_signed(d).0
    }
}

impl AddAssign<core::time::Duration> for NaiveTime {
    fn add_assign(&mut self, rhs: core::time::Duration) {
        *self = *self + rhs;
    }
}

/// Subtract a `core::time::Duration` (unsigned) from a `NaiveTime`. Wraps
/// around a single day, like the signed [`Duration`] operators.
impl Sub<core::time::Duration> for NaiveTime {
    type Output = NaiveTime;
    fn sub(self, rhs: core::time::Duration) -> NaiveTime {
        let secs = (rhs.as_secs() % (2 * 86_400)) as i64;
        let d = Duration::new(secs, rhs.subsec_nanos()).expect("duration in range");
        self.overflowing_sub_signed(d).0
    }
}

impl SubAssign<core::time::Duration> for NaiveTime {
    fn sub_assign(&mut self, rhs: core::time::Duration) {
        *self = *self - rhs;
    }
}

impl Sub<NaiveTime> for NaiveTime {
    type Output = Duration;
    fn sub(self, rhs: NaiveTime) -> Duration {
        self.signed_duration_since(rhs)
    }
}

/// Add a [`FixedOffset`] to a `NaiveTime`. Wraps around a single day.
impl Add<FixedOffset> for NaiveTime {
    type Output = NaiveTime;
    fn add(self, rhs: FixedOffset) -> NaiveTime {
        self.overflowing_add_offset(rhs).0
    }
}

/// Subtract a [`FixedOffset`] from a `NaiveTime`. Wraps around a single day.
impl Sub<FixedOffset> for NaiveTime {
    type Output = NaiveTime;
    fn sub(self, rhs: FixedOffset) -> NaiveTime {
        self.overflowing_sub_offset(rhs).0
    }
}

impl fmt::Debug for NaiveTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (hour, min, sec) = self.hms();
        let (sec, nano) = if self.frac >= NANOS_PER_SEC {
            (sec + 1, self.frac - NANOS_PER_SEC)
        } else {
            (sec, self.frac)
        };
        write!(f, "{hour:02}:{min:02}:{sec:02}")?;
        if nano == 0 {
            Ok(())
        } else if nano % 1_000_000 == 0 {
            write!(f, ".{:03}", nano / 1_000_000)
        } else if nano % 1_000 == 0 {
            write!(f, ".{:06}", nano / 1_000)
        } else {
            write!(f, ".{nano:09}")
        }
    }
}

/// Same output as `Debug`, equivalent to `%H:%M:%S%.f`.
impl fmt::Display for NaiveTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Default for NaiveTime {
    /// Defaults to midnight, 00:00:00.
    fn default() -> Self {
        NaiveTime::MIN
    }
}

/// Parsing a `str` into a `NaiveTime` uses the format `%H:%M:%S%.f`, with
/// the seconds and sub-seconds optional.
impl core::str::FromStr for NaiveTime {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<NaiveTime> {
        const HOUR_AND_MINUTE: &[Item<'static>] = &[
            Item::Numeric(Numeric::Hour, Pad::Zero),
            Item::Space(""),
            Item::Literal(":"),
            Item::Numeric(Numeric::Minute, Pad::Zero),
        ];
        const SECOND_AND_NANOS: &[Item<'static>] = &[
            Item::Space(""),
            Item::Literal(":"),
            Item::Numeric(Numeric::Second, Pad::Zero),
            Item::Fixed(Fixed::Nanosecond),
            Item::Space(""),
        ];
        const TRAILING_WHITESPACE: [Item<'static>; 1] = [Item::Space("")];

        let mut parsed = Parsed::new();
        let s = parse_and_remainder(&mut parsed, s, HOUR_AND_MINUTE.iter())?;
        // Seconds are optional, don't fail if parsing them doesn't succeed.
        let s = parse_and_remainder(&mut parsed, s, SECOND_AND_NANOS.iter()).unwrap_or(s);
        parse(&mut parsed, s, TRAILING_WHITESPACE.iter())?;
        parsed.to_naive_time()
    }
}

/// `serde` support: serializes as the ISO 8601-ish string (`Display`/`FromStr`).
#[cfg(feature = "serde")]
mod serde_impl {
    use super::NaiveTime;
    use core::fmt;
    use serde::{de, ser};

    impl ser::Serialize for NaiveTime {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            serializer.collect_str(self)
        }
    }

    struct NaiveTimeVisitor;

    impl de::Visitor<'_> for NaiveTimeVisitor {
        type Value = NaiveTime;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a formatted time string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(E::custom)
        }
    }

    impl<'de> de::Deserialize<'de> for NaiveTime {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(NaiveTimeVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hms_opt_validity() {
        assert!(NaiveTime::from_hms_opt(23, 59, 59).is_some());
        assert!(NaiveTime::from_hms_opt(24, 0, 0).is_none());
        assert!(NaiveTime::from_hms_opt(0, 60, 0).is_none());
        assert!(NaiveTime::from_hms_opt(0, 0, 60).is_none());
    }

    #[test]
    fn from_hms_nano_opt_validity_and_leap_second_bounds() {
        assert!(NaiveTime::from_hms_nano_opt(23, 59, 59, 1_999_999_999).is_some());
        assert!(NaiveTime::from_hms_nano_opt(23, 59, 59, 2_000_000_000).is_none());
        // Leap-second nanoseconds are only accepted when `sec == 59`.
        assert!(NaiveTime::from_hms_nano_opt(23, 59, 58, 1_000_000_000).is_none());
        assert!(NaiveTime::from_hms_nano_opt(24, 0, 0, 0).is_none());
        assert!(NaiveTime::from_hms_nano_opt(0, 60, 0, 0).is_none());
        assert!(NaiveTime::from_hms_nano_opt(0, 0, 60, 0).is_none());
    }

    #[test]
    fn from_hms_milli_and_micro_opt_reject_overflow() {
        assert_eq!(NaiveTime::from_hms_milli_opt(0, 0, 0, u32::MAX), None);
        assert_eq!(NaiveTime::from_hms_micro_opt(0, 0, 0, u32::MAX), None);
    }

    #[test]
    fn from_hms_milli_and_micro_opt_agree_with_nano() {
        assert_eq!(
            NaiveTime::from_hms_milli_opt(1, 2, 3, 500),
            NaiveTime::from_hms_nano_opt(1, 2, 3, 500_000_000)
        );
        assert_eq!(
            NaiveTime::from_hms_micro_opt(1, 2, 3, 500),
            NaiveTime::from_hms_nano_opt(1, 2, 3, 500_000)
        );
    }

    #[test]
    fn from_num_seconds_from_midnight_opt_validity() {
        assert_eq!(NaiveTime::from_num_seconds_from_midnight_opt(0, 0), Some(NaiveTime::MIN));
        assert!(NaiveTime::from_num_seconds_from_midnight_opt(86_400, 0).is_none());
        // 86_399 seconds = 23:59:59, the only second allowed to carry a
        // leap-second nanosecond value.
        assert!(NaiveTime::from_num_seconds_from_midnight_opt(86_399, 1_000_000_000).is_some());
        assert!(NaiveTime::from_num_seconds_from_midnight_opt(0, 1_000_000_000).is_none());
    }

    #[test]
    fn overflowing_add_signed_no_wrap_returns_zero_carry() {
        let t = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        assert_eq!(
            t.overflowing_add_signed(Duration::hours(1)),
            (NaiveTime::from_hms_opt(11, 0, 0).unwrap(), 0)
        );
    }

    #[test]
    fn overflowing_add_signed_wraps_forward_across_midnight() {
        let t = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let (result, carry) = t.overflowing_add_signed(Duration::hours(2));
        assert_eq!(result, NaiveTime::from_hms_opt(1, 0, 0).unwrap());
        // The caller *adds* this to the date, so +86400 means "one day
        // forward".
        assert_eq!(carry, 86_400);
    }

    #[test]
    fn overflowing_sub_signed_wraps_backward_across_midnight() {
        let t = NaiveTime::from_hms_opt(1, 0, 0).unwrap();
        let (result, carry) = t.overflowing_sub_signed(Duration::hours(2));
        assert_eq!(result, NaiveTime::from_hms_opt(23, 0, 0).unwrap());
        // The caller *subtracts* this from the date
        // (`date.checked_sub_signed(carry)`), so a positive carry here
        // means "go back one day" -- the opposite sign convention from
        // `overflowing_add_signed`, because of how each is consumed.
        assert_eq!(carry, 86_400);
    }

    #[test]
    fn overflowing_add_signed_stays_within_leap_second() {
        // 23:59:60.5 (leap second, half a second in) + 200ms should still
        // be within the same leap second (.7), with no carry.
        let leap = NaiveTime::from_hms_milli_opt(23, 59, 59, 1500).unwrap();
        let (result, carry) = leap.overflowing_add_signed(Duration::milliseconds(200));
        assert!(result.nanosecond() >= 1_000_000_000, "should still be within the leap second");
        assert_eq!(carry, 0);
    }

    #[test]
    fn overflowing_add_signed_escapes_leap_second_into_next_day() {
        // 23:59:60.9 + 200ms crosses past the leap second into the next day.
        let leap = NaiveTime::from_hms_milli_opt(23, 59, 59, 1900).unwrap();
        let (result, carry) = leap.overflowing_add_signed(Duration::milliseconds(200));
        assert_eq!(result, NaiveTime::from_hms_milli_opt(0, 0, 0, 100).unwrap());
        assert_eq!(carry, 86_400);
    }

    #[test]
    fn signed_duration_since_basic() {
        let a = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let b = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
        assert_eq!(a.signed_duration_since(b), Duration::minutes(90));
        assert_eq!(b.signed_duration_since(a), Duration::minutes(-90));
    }

    #[test]
    fn overflowing_add_offset_wraps_and_reports_day_shift() {
        let t = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let offset = FixedOffset::east_opt(2 * 3600).unwrap();
        let (result, days) = t.overflowing_add_offset(offset);
        assert_eq!(result, NaiveTime::from_hms_opt(1, 0, 0).unwrap());
        assert_eq!(days, 1);
    }

    #[test]
    fn overflowing_sub_offset_wraps_and_reports_day_shift() {
        let t = NaiveTime::from_hms_opt(1, 0, 0).unwrap();
        let offset = FixedOffset::east_opt(2 * 3600).unwrap();
        let (result, days) = t.overflowing_sub_offset(offset);
        assert_eq!(result, NaiveTime::from_hms_opt(23, 0, 0).unwrap());
        assert_eq!(days, -1);
    }

    #[test]
    fn timelike_accessors() {
        let t = NaiveTime::from_hms_nano_opt(13, 45, 30, 123).unwrap();
        assert_eq!(t.hour(), 13);
        assert_eq!(t.minute(), 45);
        assert_eq!(t.second(), 30);
        assert_eq!(t.nanosecond(), 123);
        assert_eq!(t.hour12(), (true, 1)); // 13:00 -> 1 PM
        assert_eq!(NaiveTime::from_hms_opt(0, 0, 0).unwrap().hour12(), (false, 12)); // midnight
        assert_eq!(t.num_seconds_from_midnight(), 13 * 3600 + 45 * 60 + 30);
    }

    #[test]
    fn second_never_reports_60_even_for_a_leap_second() {
        // Per the `Timelike::second` contract: leap seconds must be
        // inspected via `nanosecond()` or formatting, not `second()`.
        let leap = NaiveTime::from_hms_milli_opt(23, 59, 59, 1500).unwrap();
        assert_eq!(leap.second(), 59);
        assert!(leap.nanosecond() >= 1_000_000_000);
    }

    #[test]
    fn with_hour_minute_second_mutators() {
        let t = NaiveTime::from_hms_opt(10, 20, 30).unwrap();
        assert_eq!(t.with_hour(5), NaiveTime::from_hms_opt(5, 20, 30));
        assert_eq!(t.with_hour(24), None);
        assert_eq!(t.with_minute(45), NaiveTime::from_hms_opt(10, 45, 30));
        assert_eq!(t.with_minute(60), None);
        assert_eq!(t.with_second(0), NaiveTime::from_hms_opt(10, 20, 0));
        assert_eq!(t.with_second(60), None);
    }

    #[test]
    fn with_nanosecond_mutator() {
        let t = NaiveTime::from_hms_opt(10, 20, 30).unwrap();
        assert_eq!(t.with_nanosecond(500), NaiveTime::from_hms_nano_opt(10, 20, 30, 500));
        assert_eq!(t.with_nanosecond(2_000_000_000), None);
    }

    #[test]
    fn add_sub_duration_operators_wrap_without_panicking() {
        let t = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        assert_eq!(t + Duration::hours(2), NaiveTime::from_hms_opt(1, 0, 0).unwrap());
        let t2 = NaiveTime::from_hms_opt(1, 0, 0).unwrap();
        assert_eq!(t2 - Duration::hours(2), NaiveTime::from_hms_opt(23, 0, 0).unwrap());
        let mut t3 = t;
        t3 += Duration::hours(2);
        assert_eq!(t3, NaiveTime::from_hms_opt(1, 0, 0).unwrap());
        t3 -= Duration::hours(2);
        assert_eq!(t3, NaiveTime::from_hms_opt(23, 0, 0).unwrap());
    }

    #[test]
    fn add_sub_std_duration_operators() {
        let t = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let std_dur = core::time::Duration::from_secs(2 * 3600);
        assert_eq!(t + std_dur, NaiveTime::from_hms_opt(1, 0, 0).unwrap());
        let t2 = NaiveTime::from_hms_opt(1, 0, 0).unwrap();
        assert_eq!(t2 - std_dur, NaiveTime::from_hms_opt(23, 0, 0).unwrap());
    }

    #[test]
    fn sub_naivetime_operator_returns_duration() {
        let a = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let b = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        assert_eq!(a - b, Duration::hours(2));
    }

    #[test]
    fn add_sub_fixedoffset_operators_wrap() {
        let t = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let offset = FixedOffset::east_opt(2 * 3600).unwrap();
        assert_eq!(t + offset, NaiveTime::from_hms_opt(1, 0, 0).unwrap());
        let t2 = NaiveTime::from_hms_opt(1, 0, 0).unwrap();
        assert_eq!(t2 - offset, NaiveTime::from_hms_opt(23, 0, 0).unwrap());
    }

    #[test]
    fn display_formats_fractional_seconds_with_minimal_precision() {
        assert_eq!(NaiveTime::from_hms_opt(1, 2, 3).unwrap().to_string(), "01:02:03");
        assert_eq!(NaiveTime::from_hms_milli_opt(1, 2, 3, 500).unwrap().to_string(), "01:02:03.500");
        assert_eq!(NaiveTime::from_hms_micro_opt(1, 2, 3, 500).unwrap().to_string(), "01:02:03.000500");
        assert_eq!(NaiveTime::from_hms_nano_opt(1, 2, 3, 500).unwrap().to_string(), "01:02:03.000000500");
    }

    #[test]
    fn debug_and_display_show_leap_second_as_60() {
        let leap = NaiveTime::from_hms_milli_opt(23, 59, 59, 1500).unwrap();
        assert_eq!(leap.to_string(), "23:59:60.500");
        assert_eq!(format!("{leap:?}"), "23:59:60.500");
    }

    #[test]
    fn default_is_midnight() {
        assert_eq!(NaiveTime::default(), NaiveTime::MIN);
        assert_eq!(NaiveTime::default(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn from_str_parses_hh_mm_and_optional_seconds() {
        assert_eq!("12:30".parse::<NaiveTime>(), Ok(NaiveTime::from_hms_opt(12, 30, 0).unwrap()));
        assert_eq!("12:30:45".parse::<NaiveTime>(), Ok(NaiveTime::from_hms_opt(12, 30, 45).unwrap()));
        assert_eq!(
            "12:30:45.5".parse::<NaiveTime>(),
            Ok(NaiveTime::from_hms_milli_opt(12, 30, 45, 500).unwrap())
        );
        assert!("garbage".parse::<NaiveTime>().is_err());
    }

    #[test]
    fn display_and_from_str_round_trip() {
        let t = NaiveTime::from_hms_nano_opt(23, 59, 59, 123_456_789).unwrap();
        assert_eq!(t.to_string().parse::<NaiveTime>(), Ok(t));
    }

    #[test]
    fn ordering_matches_time_of_day_order() {
        let a = NaiveTime::from_hms_opt(1, 0, 0).unwrap();
        let b = NaiveTime::from_hms_opt(2, 0, 0).unwrap();
        assert!(a < b);
        assert!(NaiveTime::MIN < b);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let t = NaiveTime::from_hms_milli_opt(12, 30, 45, 500).unwrap();
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"12:30:45.500\"");
        let back: NaiveTime = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}
