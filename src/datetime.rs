//! `DateTime<Tz>`: a date and time of day, together with a time zone.

use crate::duration::{Days, Months, TimeDelta};
use crate::format::{
    parse, parse_and_remainder, parse_rfc3339, write_rfc2822, write_rfc3339, DelayedFormat, Fixed,
    Item, ParseError, ParseResult, Parsed, SecondsFormat, StrftimeItems,
};
#[cfg(feature = "unstable-locales")]
use crate::format::Locale;
use crate::naive::{IsoWeek, NaiveDate, NaiveDateTime, NaiveTime};
use crate::offset::{FixedOffset, Local, MappedLocalTime, Offset, TimeZone, Utc};
use crate::traits::{Datelike, Timelike};
use crate::weekday::Weekday;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash;
use core::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// Day number (since 0001-01-01, the proleptic Gregorian epoch) of the
/// Unix epoch, 1970-01-01. Matches
/// `NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().num_days_from_ce()`.
const UNIX_EPOCH_DAY: i64 = 719_163;

/// A date and time of day, together with a time zone.
///
/// API aligned with `chrono::DateTime`. Most of the time you will build
/// one through the [`TimeZone`] methods (e.g. [`Utc.with_ymd_and_hms(...)`](TimeZone::with_ymd_and_hms))
/// rather than the `from_*` constructors here, which are lower-level.
///
/// # Deferred
/// The deprecated `Date<Tz>` type is not implemented (see
/// `crate::offset`'s module documentation) -- a known, accepted gap since
/// it is itself deprecated in `chrono`.
#[derive(Clone)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq, PartialOrd))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct DateTime<Tz: TimeZone> {
    datetime: NaiveDateTime,
    offset: Tz::Offset,
}

// Note that `Arbitrary` cannot simply be derived for `DateTime<Tz>`, due to
// the nontrivial bound `<Tz as TimeZone>::Offset: Arbitrary`.
#[cfg(feature = "arbitrary")]
impl<'a, Tz> arbitrary::Arbitrary<'a> for DateTime<Tz>
where
    Tz: TimeZone,
    <Tz as TimeZone>::Offset: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<DateTime<Tz>> {
        let datetime = NaiveDateTime::arbitrary(u)?;
        let offset = <Tz as TimeZone>::Offset::arbitrary(u)?;
        Ok(DateTime::from_naive_utc_and_offset(datetime, offset))
    }
}

#[cfg(feature = "defmt")]
impl<Tz: TimeZone> defmt::Format for DateTime<Tz>
where
    Tz::Offset: defmt::Format,
{
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}{}", self.overflowing_naive_local(), self.offset);
    }
}

/// Deprecated alias for [`DateTime::<Utc>::MIN_UTC`].
#[deprecated(note = "use `DateTime::<Utc>::MIN_UTC` instead")]
pub const MIN_DATETIME: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;

/// Deprecated alias for [`DateTime::<Utc>::MAX_UTC`].
#[deprecated(note = "use `DateTime::<Utc>::MAX_UTC` instead")]
pub const MAX_DATETIME: DateTime<Utc> = DateTime::<Utc>::MAX_UTC;

impl<Tz: TimeZone> DateTime<Tz> {
    /// Builds a `DateTime` from its raw components: a `NaiveDateTime` *in
    /// UTC*, and an offset. Low-level; prefer
    /// [`TimeZone::from_local_datetime`] or
    /// [`NaiveDateTime::and_local_timezone`] for regular use.
    pub const fn from_naive_utc_and_offset(datetime: NaiveDateTime, offset: Tz::Offset) -> DateTime<Tz> {
        DateTime { datetime, offset }
    }

    /// Builds a `DateTime` from its raw components, like
    /// [`from_naive_utc_and_offset`](Self::from_naive_utc_and_offset).
    #[deprecated(
        note = "use `TimeZone::from_utc_datetime()` or `DateTime::from_naive_utc_and_offset` instead"
    )]
    pub fn from_utc(datetime: NaiveDateTime, offset: Tz::Offset) -> DateTime<Tz> {
        DateTime { datetime, offset }
    }

    /// Builds a `DateTime` from a *local* `NaiveDateTime` and an offset.
    ///
    /// # Panics
    /// Panics if converting `datetime` to UTC would be out of range.
    #[deprecated(
        note = "use `TimeZone::from_local_datetime()` or `NaiveDateTime::and_local_timezone` instead"
    )]
    pub fn from_local(datetime: NaiveDateTime, offset: Tz::Offset) -> DateTime<Tz> {
        let datetime_utc = datetime - offset.fix();
        DateTime { datetime: datetime_utc, offset }
    }

    /// The date component, in the local time zone.
    ///
    /// # Panics
    /// Panics if the offset from UTC would push the local date outside
    /// the representable range of a [`NaiveDate`].
    pub fn date_naive(&self) -> NaiveDate {
        self.naive_local().date()
    }

    /// The time-of-day component, in the local time zone.
    pub fn time(&self) -> NaiveTime {
        self.datetime.time() + self.offset.fix()
    }

    /// The number of non-leap seconds since January 1, 1970 0:00:00 UTC
    /// (a "UNIX timestamp").
    pub const fn timestamp(&self) -> i64 {
        let gregorian_day = self.datetime.date().num_days_from_ce() as i64;
        let seconds_from_midnight = self.datetime.time().num_seconds_from_midnight() as i64;
        (gregorian_day - UNIX_EPOCH_DAY) * 86_400 + seconds_from_midnight
    }

    /// The number of non-leap milliseconds since the Unix epoch.
    pub const fn timestamp_millis(&self) -> i64 {
        let as_ms = self.timestamp() * 1000;
        as_ms + self.timestamp_subsec_millis() as i64
    }

    /// The number of non-leap microseconds since the Unix epoch.
    pub const fn timestamp_micros(&self) -> i64 {
        let as_us = self.timestamp() * 1_000_000;
        as_us + self.timestamp_subsec_micros() as i64
    }

    /// The number of non-leap nanoseconds since the Unix epoch.
    ///
    /// # Panics
    /// An `i64` of nanoseconds spans only ~584 years; panics if `self` is
    /// outside that range. See
    /// [`timestamp_nanos_opt`](Self::timestamp_nanos_opt) for a
    /// non-panicking version.
    #[deprecated(note = "use `timestamp_nanos_opt()` instead")]
    pub const fn timestamp_nanos(&self) -> i64 {
        match self.timestamp_nanos_opt() {
            Some(n) => n,
            None => {
                panic!("value can not be represented in a timestamp with nanosecond precision")
            }
        }
    }

    /// The number of non-leap nanoseconds since the Unix epoch, or `None`
    /// if out of range (an `i64` of nanoseconds spans only ~584 years).
    pub const fn timestamp_nanos_opt(&self) -> Option<i64> {
        let mut timestamp = self.timestamp();
        let mut subsec_nanos = self.timestamp_subsec_nanos() as i64;
        // Avoid a temporary underflow: shift the split point by one
        // second for negative timestamps.
        if timestamp < 0 {
            subsec_nanos -= 1_000_000_000;
            timestamp += 1;
        }
        match timestamp.checked_mul(1_000_000_000) {
            Some(v) => v.checked_add(subsec_nanos),
            None => None,
        }
    }

    /// Milliseconds since the last whole non-leap second (may exceed 999
    /// for a leap second).
    pub const fn timestamp_subsec_millis(&self) -> u32 {
        self.timestamp_subsec_nanos() / 1_000_000
    }

    /// Microseconds since the last whole non-leap second.
    pub const fn timestamp_subsec_micros(&self) -> u32 {
        self.timestamp_subsec_nanos() / 1_000
    }

    /// Nanoseconds since the last whole non-leap second.
    pub const fn timestamp_subsec_nanos(&self) -> u32 {
        self.datetime.time().nanosecond()
    }

    /// The offset from UTC in effect at this instant.
    pub const fn offset(&self) -> &Tz::Offset {
        &self.offset
    }

    /// The time zone associated with this value.
    pub fn timezone(&self) -> Tz {
        TimeZone::from_offset(&self.offset)
    }

    /// Changes the associated time zone, preserving the instant in time
    /// (not the wall-clock reading).
    pub fn with_timezone<Tz2: TimeZone>(&self, tz: &Tz2) -> DateTime<Tz2> {
        tz.from_utc_datetime(&self.datetime)
    }

    /// Fixes the current offset, dropping the associated time zone in
    /// favor of a plain [`FixedOffset`]. Useful to convert a generic
    /// `DateTime<Tz>` to `DateTime<FixedOffset>`.
    pub fn fixed_offset(&self) -> DateTime<FixedOffset> {
        self.with_timezone(&self.offset().fix())
    }

    /// Converts to `DateTime<Utc>`, dropping the offset and time zone.
    pub const fn to_utc(&self) -> DateTime<Utc> {
        DateTime { datetime: self.datetime, offset: Utc }
    }

    /// Adds a signed [`TimeDelta`]. Returns `None` if the resulting date
    /// would be out of range.
    pub fn checked_add_signed(self, rhs: TimeDelta) -> Option<DateTime<Tz>> {
        let datetime = self.datetime.checked_add_signed(rhs)?;
        let tz = self.timezone();
        Some(tz.from_utc_datetime(&datetime))
    }

    /// Adds [`Months`], clamping to the last valid day of the resulting
    /// month. Returns `None` if the resulting date would be out of range,
    /// or if the local time at the resulting date does not exist or is
    /// ambiguous (e.g. a DST transition).
    pub fn checked_add_months(self, months: Months) -> Option<DateTime<Tz>> {
        self.overflowing_naive_local()
            .checked_add_months(months)?
            .and_local_timezone(Tz::from_offset(&self.offset))
            .single()
    }

    /// Subtracts a signed [`TimeDelta`]. Returns `None` if the resulting
    /// date would be out of range.
    pub fn checked_sub_signed(self, rhs: TimeDelta) -> Option<DateTime<Tz>> {
        let datetime = self.datetime.checked_sub_signed(rhs)?;
        let tz = self.timezone();
        Some(tz.from_utc_datetime(&datetime))
    }

    /// Subtracts [`Months`]. See
    /// [`checked_add_months`](Self::checked_add_months).
    pub fn checked_sub_months(self, months: Months) -> Option<DateTime<Tz>> {
        self.overflowing_naive_local()
            .checked_sub_months(months)?
            .and_local_timezone(Tz::from_offset(&self.offset))
            .single()
    }

    /// Adds a duration in [`Days`] to the date part. Returns `None` if
    /// the resulting date would be out of range, or the local time does
    /// not exist or is ambiguous.
    pub fn checked_add_days(self, days: Days) -> Option<Self> {
        if days == Days::new(0) {
            return Some(self);
        }
        self.overflowing_naive_local()
            .checked_add_days(days)
            .and_then(|dt| self.timezone().from_local_datetime(&dt).single())
            .filter(|dt| dt <= &DateTime::<Utc>::MAX_UTC)
    }

    /// Subtracts a duration in [`Days`] from the date part. See
    /// [`checked_add_days`](Self::checked_add_days).
    pub fn checked_sub_days(self, days: Days) -> Option<Self> {
        self.overflowing_naive_local()
            .checked_sub_days(days)
            .and_then(|dt| self.timezone().from_local_datetime(&dt).single())
            .filter(|dt| dt >= &DateTime::<Utc>::MIN_UTC)
    }

    /// Signed duration since another `DateTime` (`self - rhs`). Never
    /// overflows or underflows. Accepts `rhs` by value or by reference.
    pub fn signed_duration_since<Tz2: TimeZone>(self, rhs: impl Borrow<DateTime<Tz2>>) -> TimeDelta {
        self.datetime.signed_duration_since(rhs.borrow().datetime)
    }

    /// A view of the naive UTC datetime (i.e. without the offset applied).
    pub const fn naive_utc(&self) -> NaiveDateTime {
        self.datetime
    }

    /// A view of the naive local datetime (i.e. with the offset applied).
    ///
    /// # Panics
    /// Panics if the offset would push the value outside the
    /// representable range of a [`NaiveDateTime`].
    pub fn naive_local(&self) -> NaiveDateTime {
        self.datetime
            .checked_add_offset(self.offset.fix())
            .expect("Local time out of range for `NaiveDateTime`")
    }

    /// Like [`naive_local`](Self::naive_local), but never panics: the
    /// result may briefly use the buffer space just outside
    /// `NaiveDateTime`'s normal range. Meant as an internal intermediate
    /// value only.
    fn overflowing_naive_local(&self) -> NaiveDateTime {
        self.datetime.overflowing_add_offset(self.offset.fix())
    }

    /// Elapsed whole years from `base` to `self`. Returns `None` if
    /// `base > self`.
    pub fn years_since(&self, base: Self) -> Option<u32> {
        let mut years = self.year() - base.year();
        let earlier_time =
            (self.month(), self.day(), self.time()) < (base.month(), base.day(), base.time());
        if earlier_time {
            years -= 1;
        }
        if years >= 0 {
            Some(years as u32)
        } else {
            None
        }
    }

    /// Age in whole years, as of `on` -- e.g. `self` is a date/time of
    /// birth and `on` is a reference instant such as "now". Returns `None`
    /// if `on` is before `self`.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// This method has **no equivalent in chrono**: it is a `time_compute`-only
    /// addition, layered on top of the frozen, chrono-compatible API surface,
    /// inspired by WLanguage's `Age()` function. Exactly
    /// [`years_since`](Self::years_since) with the arguments in the order
    /// that reads naturally here (`date_of_birth.age(now)` rather than
    /// `now.years_since(date_of_birth)`).
    ///
    /// `DateTime<Tz>` never reads the system clock on its own -- pass
    /// "now" explicitly, e.g. `date_of_birth.age(Utc::now())` or
    /// `date_of_birth.age(Local::now())`. See also [`NaiveDate::age`] for
    /// the date-only equivalent (e.g. via `date_of_birth.age(Utc::now().date_naive())`).
    #[must_use]
    pub fn age(&self, on: Self) -> Option<u32> {
        on.years_since(self.clone())
    }

    /// Sets the time-of-day part, keeping the date. Returns
    /// `MappedLocalTime::None` if this would push the value out of range.
    pub fn with_time(&self, time: NaiveTime) -> MappedLocalTime<Self> {
        self.timezone()
            .from_local_datetime(&self.overflowing_naive_local().date().and_time(time))
    }

    /// The smallest representable `DateTime<Utc>`.
    pub const MIN_UTC: DateTime<Utc> = DateTime { datetime: NaiveDateTime::MIN, offset: Utc };

    /// The largest representable `DateTime<Utc>`.
    pub const MAX_UTC: DateTime<Utc> = DateTime { datetime: NaiveDateTime::MAX, offset: Utc };
}

impl<Tz: TimeZone> DateTime<Tz>
where
    Tz::Offset: fmt::Display,
{
    /// Returns an RFC 2822 date and time string such as `Tue, 1 Jul 2003 10:52:37 +0200`.
    ///
    /// # Panics
    /// Panics if the date cannot be represented in RFC 2822 (years 0 through 9999).
    #[must_use]
    pub fn to_rfc2822(&self) -> String {
        let mut result = String::with_capacity(32);
        write_rfc2822(&mut result, self.overflowing_naive_local(), self.offset.fix())
            .expect("date cannot be represented by RFC 2822");
        result
    }

    /// Returns an RFC 3339 and ISO 8601 date and time string such as `1996-12-19T16:39:57-08:00`.
    #[must_use]
    pub fn to_rfc3339(&self) -> String {
        let mut result = String::with_capacity(32);
        let naive = self.overflowing_naive_local();
        let offset = self.offset.fix();
        write_rfc3339(&mut result, naive, offset, SecondsFormat::AutoSi, false)
            .expect("writing rfc3339 datetime to string should never fail");
        result
    }

    /// Returns an RFC 3339 and ISO 8601 date and time string with sub-seconds
    /// formatted as per `SecondsFormat`.
    ///
    /// If `use_z` is true and the timezone is UTC (offset 0), uses `Z` as
    /// per [`Fixed::TimezoneOffsetColonZ`]. If `use_z` is false, uses
    /// [`Fixed::TimezoneOffsetColon`].
    #[must_use]
    pub fn to_rfc3339_opts(&self, secform: SecondsFormat, use_z: bool) -> String {
        let mut result = String::with_capacity(38);
        write_rfc3339(&mut result, self.naive_local(), self.offset.fix(), secform, use_z)
            .expect("writing rfc3339 datetime to string should never fail");
        result
    }

    /// Formats the combined date, time and time zone with the specified formatting items.
    #[must_use]
    pub fn format_with_items<'a, I, B>(&self, items: I) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        let local = self.overflowing_naive_local();
        DelayedFormat::new_with_offset(Some(local.date()), Some(local.time()), &self.offset, items)
    }

    /// Formats the combined date, time and time zone with the specified
    /// format string. See the [`crate::format::strftime`] module for the
    /// supported escape sequences.
    #[must_use]
    pub fn format<'a>(&self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'a>> {
        self.format_with_items(StrftimeItems::new(fmt))
    }

    /// Formats the combined date and time with the specified formatting
    /// items and locale.
    ///
    /// Requires the `unstable-locales` feature.
    #[cfg(feature = "unstable-locales")]
    #[must_use]
    pub fn format_localized_with_items<'a, I, B>(&self, items: I, locale: Locale) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        let local = self.overflowing_naive_local();
        DelayedFormat::new_with_offset_and_locale(
            Some(local.date()),
            Some(local.time()),
            &self.offset,
            items,
            locale,
        )
    }

    /// Formats the combined date and time with the specified format string
    /// and locale. See the [`crate::format::strftime`] module for the
    /// supported escape sequences.
    ///
    /// Requires the `unstable-locales` feature.
    #[cfg(feature = "unstable-locales")]
    #[must_use]
    pub fn format_localized<'a>(&self, fmt: &'a str, locale: Locale) -> DelayedFormat<StrftimeItems<'a>> {
        self.format_localized_with_items(StrftimeItems::new_with_locale(fmt, locale), locale)
    }
}

impl DateTime<Utc> {
    /// Builds a `DateTime<Utc>` from a UNIX timestamp in seconds. A
    /// convenience wrapper around [`from_timestamp`](Self::from_timestamp)
    /// useful with [`Iterator::map`].
    pub const fn from_timestamp_secs(secs: i64) -> Option<Self> {
        Self::from_timestamp(secs, 0)
    }

    /// Builds a `DateTime<Utc>` from a UNIX timestamp in seconds and a
    /// nanosecond remainder. The nanosecond part may exceed 1,000,000,000
    /// to represent a leap second (only when `secs % 60 == 59`). Returns
    /// `None` on out-of-range input.
    pub const fn from_timestamp(secs: i64, nsecs: u32) -> Option<Self> {
        let days = secs.div_euclid(86_400) + UNIX_EPOCH_DAY;
        let secs_of_day = secs.rem_euclid(86_400);
        if days < i32::MIN as i64 || days > i32::MAX as i64 {
            return None;
        }
        let date = match NaiveDate::from_num_days_from_ce_opt(days as i32) {
            Some(d) => d,
            None => return None,
        };
        let time = match NaiveTime::from_num_seconds_from_midnight_opt(secs_of_day as u32, nsecs) {
            Some(t) => t,
            None => return None,
        };
        Some(date.and_time(time).and_utc())
    }

    /// Builds a `DateTime<Utc>` from a UNIX timestamp in milliseconds.
    pub const fn from_timestamp_millis(millis: i64) -> Option<Self> {
        let secs = millis.div_euclid(1000);
        let nsecs = millis.rem_euclid(1000) as u32 * 1_000_000;
        Self::from_timestamp(secs, nsecs)
    }

    /// Builds a `DateTime<Utc>` from a UNIX timestamp in microseconds.
    pub const fn from_timestamp_micros(micros: i64) -> Option<Self> {
        let secs = micros.div_euclid(1_000_000);
        let nsecs = micros.rem_euclid(1_000_000) as u32 * 1000;
        Self::from_timestamp(secs, nsecs)
    }

    /// Builds a `DateTime<Utc>` from a UNIX timestamp in nanoseconds.
    /// Never fails: an `i64` of nanoseconds always fits.
    pub const fn from_timestamp_nanos(nanos: i64) -> Self {
        let secs = nanos.div_euclid(1_000_000_000);
        let nsecs = nanos.rem_euclid(1_000_000_000) as u32;
        match Self::from_timestamp(secs, nsecs) {
            Some(dt) => dt,
            None => panic!("timestamp in nanos is always in range"),
        }
    }

    /// The Unix epoch, 1970-01-01 00:00:00 UTC.
    pub const UNIX_EPOCH: Self = match NaiveDate::from_ymd_opt(1970, 1, 1) {
        Some(d) => d.and_time(NaiveTime::MIN).and_utc(),
        None => panic!("unreachable"),
    };
}

impl DateTime<FixedOffset> {
    /// Parses an RFC 2822 date-and-time string (such as
    /// `Tue, 1 Jul 2003 10:52:37 +0200`) into a `DateTime<FixedOffset>`.
    pub fn parse_from_rfc2822(s: &str) -> ParseResult<DateTime<FixedOffset>> {
        const ITEMS: &[Item<'static>] = &[Item::Fixed(Fixed::RFC2822)];
        let mut parsed = Parsed::new();
        parse(&mut parsed, s, ITEMS.iter())?;
        parsed.to_datetime()
    }

    /// Parses an RFC 3339 date-and-time string into a `DateTime<FixedOffset>`.
    pub fn parse_from_rfc3339(s: &str) -> ParseResult<DateTime<FixedOffset>> {
        parse_rfc3339(s)
    }

    /// Parses a string from a user-specified format into a
    /// `DateTime<FixedOffset>`. Note that this method *requires* a timezone
    /// in the input string; see
    /// [`NaiveDateTime::parse_from_str`](crate::NaiveDateTime::parse_from_str)
    /// for a version that does not. See the [`crate::format::strftime`]
    /// module for the supported escape sequences.
    pub fn parse_from_str(s: &str, fmt: &str) -> ParseResult<DateTime<FixedOffset>> {
        let mut parsed = Parsed::new();
        parse(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_datetime()
    }

    /// Parses a string from a user-specified format into a
    /// `DateTime<FixedOffset>`, returning the value and a slice with the
    /// remaining, unparsed portion of the string. Note that this method
    /// *requires* a timezone in the input string.
    pub fn parse_and_remainder<'a>(
        s: &'a str,
        fmt: &str,
    ) -> ParseResult<(DateTime<FixedOffset>, &'a str)> {
        let mut parsed = Parsed::new();
        let remainder = parse_and_remainder(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_datetime().map(|d| (d, remainder))
    }
}

impl Default for DateTime<Utc> {
    fn default() -> Self {
        Utc.from_utc_datetime(&NaiveDateTime::default())
    }
}

impl Default for DateTime<Local> {
    fn default() -> Self {
        Local.from_utc_datetime(&NaiveDateTime::default())
    }
}

impl Default for DateTime<FixedOffset> {
    fn default() -> Self {
        FixedOffset::west_opt(0)
            .unwrap()
            .from_utc_datetime(&NaiveDateTime::default())
    }
}

/// Converts a `DateTime<Utc>` into a `DateTime<FixedOffset>` with a fixed
/// offset of zero.
impl From<DateTime<Utc>> for DateTime<FixedOffset> {
    fn from(src: DateTime<Utc>) -> Self {
        src.with_timezone(&FixedOffset::east_opt(0).unwrap())
    }
}

/// Converts a `DateTime<Utc>` into a `DateTime<Local>`.
impl From<DateTime<Utc>> for DateTime<Local> {
    fn from(src: DateTime<Utc>) -> Self {
        src.with_timezone(&Local)
    }
}

/// Converts a `DateTime<FixedOffset>` into a `DateTime<Utc>`.
impl From<DateTime<FixedOffset>> for DateTime<Utc> {
    fn from(src: DateTime<FixedOffset>) -> Self {
        src.with_timezone(&Utc)
    }
}

/// Converts a `DateTime<FixedOffset>` into a `DateTime<Local>`.
impl From<DateTime<FixedOffset>> for DateTime<Local> {
    fn from(src: DateTime<FixedOffset>) -> Self {
        src.with_timezone(&Local)
    }
}

/// Converts a `DateTime<Local>` into a `DateTime<Utc>`.
impl From<DateTime<Local>> for DateTime<Utc> {
    fn from(src: DateTime<Local>) -> Self {
        src.with_timezone(&Utc)
    }
}

/// Converts a `DateTime<Local>` into a `DateTime<FixedOffset>`.
impl From<DateTime<Local>> for DateTime<FixedOffset> {
    fn from(src: DateTime<Local>) -> Self {
        let fixed = src.offset().fix();
        src.with_timezone(&fixed)
    }
}

/// Maps the local datetime with the given conversion function, staying
/// within the same time zone.
fn map_local<Tz: TimeZone, F>(dt: &DateTime<Tz>, mut f: F) -> Option<DateTime<Tz>>
where
    F: FnMut(NaiveDateTime) -> Option<NaiveDateTime>,
{
    f(dt.overflowing_naive_local())
        .and_then(|datetime| dt.timezone().from_local_datetime(&datetime).single())
        .filter(|dt| dt >= &DateTime::<Utc>::MIN_UTC && dt <= &DateTime::<Utc>::MAX_UTC)
}

impl<Tz: TimeZone> Datelike for DateTime<Tz> {
    fn year(&self) -> i32 {
        self.overflowing_naive_local().year()
    }
    fn month(&self) -> u32 {
        self.overflowing_naive_local().month()
    }
    fn month0(&self) -> u32 {
        self.overflowing_naive_local().month0()
    }
    fn day(&self) -> u32 {
        self.overflowing_naive_local().day()
    }
    fn day0(&self) -> u32 {
        self.overflowing_naive_local().day0()
    }
    fn ordinal(&self) -> u32 {
        self.overflowing_naive_local().ordinal()
    }
    fn ordinal0(&self) -> u32 {
        self.overflowing_naive_local().ordinal0()
    }
    fn weekday(&self) -> Weekday {
        self.overflowing_naive_local().weekday()
    }
    fn iso_week(&self) -> IsoWeek {
        self.overflowing_naive_local().iso_week()
    }

    fn with_year(&self, year: i32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| match dt.year() == year {
            true => Some(dt),
            false => dt.with_year(year),
        })
    }

    fn with_month(&self, month: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_month(month))
    }

    fn with_month0(&self, month0: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_month0(month0))
    }

    fn with_day(&self, day: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_day(day))
    }

    fn with_day0(&self, day0: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_day0(day0))
    }

    fn with_ordinal(&self, ordinal: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_ordinal(ordinal))
    }

    fn with_ordinal0(&self, ordinal0: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_ordinal0(ordinal0))
    }
}

impl<Tz: TimeZone> Timelike for DateTime<Tz> {
    fn hour(&self) -> u32 {
        self.overflowing_naive_local().hour()
    }
    fn minute(&self) -> u32 {
        self.overflowing_naive_local().minute()
    }
    fn second(&self) -> u32 {
        self.overflowing_naive_local().second()
    }
    fn nanosecond(&self) -> u32 {
        self.overflowing_naive_local().nanosecond()
    }

    fn with_hour(&self, hour: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_hour(hour))
    }

    fn with_minute(&self, min: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_minute(min))
    }

    fn with_second(&self, sec: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_second(sec))
    }

    fn with_nanosecond(&self, nano: u32) -> Option<DateTime<Tz>> {
        map_local(self, |dt| dt.with_nanosecond(nano))
    }
}

// `Tz` itself is not stored (only `Tz::Offset` is), so `DateTime` can be
// `Copy` whenever the offset is.
impl<Tz: TimeZone> Copy for DateTime<Tz> where Tz::Offset: Copy {}

impl<Tz: TimeZone, Tz2: TimeZone> PartialEq<DateTime<Tz2>> for DateTime<Tz> {
    fn eq(&self, other: &DateTime<Tz2>) -> bool {
        self.datetime == other.datetime
    }
}

impl<Tz: TimeZone> Eq for DateTime<Tz> {}

impl<Tz: TimeZone, Tz2: TimeZone> PartialOrd<DateTime<Tz2>> for DateTime<Tz> {
    /// Compares two datetimes by the instant they represent, ignoring the
    /// time zone.
    fn partial_cmp(&self, other: &DateTime<Tz2>) -> Option<Ordering> {
        self.datetime.partial_cmp(&other.datetime)
    }
}

impl<Tz: TimeZone> Ord for DateTime<Tz> {
    fn cmp(&self, other: &DateTime<Tz>) -> Ordering {
        self.datetime.cmp(&other.datetime)
    }
}

impl<Tz: TimeZone> hash::Hash for DateTime<Tz> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.datetime.hash(state)
    }
}

/// Adds a `TimeDelta` to a `DateTime`.
///
/// # Panics
/// Panics if the resulting date would be out of range. Consider
/// [`DateTime::checked_add_signed`] for an `Option` instead.
impl<Tz: TimeZone> Add<TimeDelta> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn add(self, rhs: TimeDelta) -> DateTime<Tz> {
        self.checked_add_signed(rhs)
            .expect("`DateTime + TimeDelta` overflowed")
    }
}

/// Adds a `core::time::Duration` (unsigned) to a `DateTime`.
impl<Tz: TimeZone> Add<core::time::Duration> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn add(self, rhs: core::time::Duration) -> DateTime<Tz> {
        let rhs = TimeDelta::from_std(rhs)
            .expect("overflow converting from core::time::Duration to TimeDelta");
        self.checked_add_signed(rhs)
            .expect("`DateTime + TimeDelta` overflowed")
    }
}

impl<Tz: TimeZone> AddAssign<TimeDelta> for DateTime<Tz> {
    fn add_assign(&mut self, rhs: TimeDelta) {
        let datetime = self
            .datetime
            .checked_add_signed(rhs)
            .expect("`DateTime + TimeDelta` overflowed");
        let tz = self.timezone();
        *self = tz.from_utc_datetime(&datetime);
    }
}

impl<Tz: TimeZone> AddAssign<core::time::Duration> for DateTime<Tz> {
    fn add_assign(&mut self, rhs: core::time::Duration) {
        let rhs = TimeDelta::from_std(rhs)
            .expect("overflow converting from core::time::Duration to TimeDelta");
        *self += rhs;
    }
}

/// Adds a [`FixedOffset`] to the datetime value (the offset field itself
/// is unchanged; only the represented UTC instant shifts).
///
/// # Panics
/// Panics if the resulting date would be out of range.
impl<Tz: TimeZone> Add<FixedOffset> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn add(mut self, rhs: FixedOffset) -> DateTime<Tz> {
        self.datetime = self
            .naive_utc()
            .checked_add_offset(rhs)
            .expect("`DateTime + FixedOffset` overflowed");
        self
    }
}

/// Adds [`Months`] to a `DateTime`.
///
/// # Panics
/// Panics if the resulting date would be out of range, or the local time
/// does not exist or is ambiguous.
impl<Tz: TimeZone> Add<Months> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn add(self, rhs: Months) -> Self::Output {
        self.checked_add_months(rhs)
            .expect("`DateTime + Months` out of range")
    }
}

/// Subtracts a `TimeDelta` from a `DateTime`.
impl<Tz: TimeZone> Sub<TimeDelta> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn sub(self, rhs: TimeDelta) -> DateTime<Tz> {
        self.checked_sub_signed(rhs)
            .expect("`DateTime - TimeDelta` overflowed")
    }
}

/// Subtracts a `core::time::Duration` (unsigned) from a `DateTime`.
impl<Tz: TimeZone> Sub<core::time::Duration> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn sub(self, rhs: core::time::Duration) -> DateTime<Tz> {
        let rhs = TimeDelta::from_std(rhs)
            .expect("overflow converting from core::time::Duration to TimeDelta");
        self.checked_sub_signed(rhs)
            .expect("`DateTime - TimeDelta` overflowed")
    }
}

impl<Tz: TimeZone> SubAssign<TimeDelta> for DateTime<Tz> {
    fn sub_assign(&mut self, rhs: TimeDelta) {
        let datetime = self
            .datetime
            .checked_sub_signed(rhs)
            .expect("`DateTime - TimeDelta` overflowed");
        let tz = self.timezone();
        *self = tz.from_utc_datetime(&datetime);
    }
}

impl<Tz: TimeZone> SubAssign<core::time::Duration> for DateTime<Tz> {
    fn sub_assign(&mut self, rhs: core::time::Duration) {
        let rhs = TimeDelta::from_std(rhs)
            .expect("overflow converting from core::time::Duration to TimeDelta");
        *self -= rhs;
    }
}

/// Subtracts a [`FixedOffset`] from the datetime value.
impl<Tz: TimeZone> Sub<FixedOffset> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn sub(mut self, rhs: FixedOffset) -> DateTime<Tz> {
        self.datetime = self
            .naive_utc()
            .checked_sub_offset(rhs)
            .expect("`DateTime - FixedOffset` overflowed");
        self
    }
}

/// Subtracts [`Months`] from a `DateTime`.
impl<Tz: TimeZone> Sub<Months> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn sub(self, rhs: Months) -> Self::Output {
        self.checked_sub_months(rhs)
            .expect("`DateTime - Months` out of range")
    }
}

impl<Tz: TimeZone> Sub<DateTime<Tz>> for DateTime<Tz> {
    type Output = TimeDelta;
    fn sub(self, rhs: DateTime<Tz>) -> TimeDelta {
        self.signed_duration_since(rhs)
    }
}

impl<Tz: TimeZone> Sub<&DateTime<Tz>> for DateTime<Tz> {
    type Output = TimeDelta;
    fn sub(self, rhs: &DateTime<Tz>) -> TimeDelta {
        self.signed_duration_since(rhs)
    }
}

/// Adds [`Days`] to a `DateTime`.
impl<Tz: TimeZone> Add<Days> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn add(self, days: Days) -> Self::Output {
        self.checked_add_days(days)
            .expect("`DateTime + Days` out of range")
    }
}

/// Subtracts [`Days`] from a `DateTime`.
impl<Tz: TimeZone> Sub<Days> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn sub(self, days: Days) -> Self::Output {
        self.checked_sub_days(days)
            .expect("`DateTime - Days` out of range")
    }
}

impl<Tz: TimeZone> fmt::Debug for DateTime<Tz> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.overflowing_naive_local(), f)?;
        fmt::Debug::fmt(&self.offset, f)
    }
}

impl<Tz: TimeZone> fmt::Display for DateTime<Tz>
where
    Tz::Offset: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.overflowing_naive_local(), f)?;
        f.write_str(" ")?;
        fmt::Display::fmt(&self.offset, f)
    }
}

/// Parsing a `str` into a `DateTime<Utc>` uses a relaxed form of RFC 3339
/// (see the `DateTime<FixedOffset>` `FromStr` impl), then converts to UTC.
impl core::str::FromStr for DateTime<Utc> {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<DateTime<Utc>> {
        s.parse::<DateTime<FixedOffset>>().map(|dt| dt.with_timezone(&Utc))
    }
}

/// Parsing a `str` into a `DateTime<Local>` uses a relaxed form of RFC 3339
/// (see the `DateTime<FixedOffset>` `FromStr` impl), then converts to the
/// system's local time zone.
impl core::str::FromStr for DateTime<Local> {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<DateTime<Local>> {
        s.parse::<DateTime<FixedOffset>>().map(|dt| dt.with_timezone(&Local))
    }
}

impl From<std::time::SystemTime> for DateTime<Utc> {
    fn from(t: std::time::SystemTime) -> DateTime<Utc> {
        let (sec, nsec) = match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
            Err(e) => {
                let dur = e.duration();
                let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
                if nsec == 0 {
                    (-sec, 0)
                } else {
                    (-sec - 1, 1_000_000_000 - nsec)
                }
            }
        };
        Utc.timestamp_opt(sec, nsec).unwrap()
    }
}

impl From<std::time::SystemTime> for DateTime<Local> {
    fn from(t: std::time::SystemTime) -> DateTime<Local> {
        DateTime::<Utc>::from(t).with_timezone(&Local)
    }
}

impl<Tz: TimeZone> From<DateTime<Tz>> for std::time::SystemTime {
    fn from(dt: DateTime<Tz>) -> std::time::SystemTime {
        let sec = dt.timestamp();
        let nsec = dt.timestamp_subsec_nanos();
        if sec < 0 {
            std::time::UNIX_EPOCH - std::time::Duration::new((-sec) as u64, 0)
                + std::time::Duration::new(0, nsec)
        } else {
            std::time::UNIX_EPOCH + std::time::Duration::new(sec as u64, nsec)
        }
    }
}

/// `serde` support.
///
/// `DateTime<Tz>` (for any `Tz`) serializes as an RFC 3339 string, with an
/// ISO 8601-style extension allowing years outside 0..=9999 (a leading `+`
/// or `-`). Deserializing back requires a concrete offset type
/// (`FixedOffset`, `Utc`, or `Local`).
///
/// This module also exposes `ts_seconds`, `ts_milliseconds`,
/// `ts_microseconds` and `ts_nanoseconds` (and their `_option` variants)
/// for `DateTime<Utc>`, intended for use with serde's
/// `#[serde(with = "...")]` field attribute to (de)serialize as a Unix
/// timestamp instead. Re-exported at the crate root as `time_compute::serde`.
#[cfg(feature = "serde")]
pub(crate) mod serde {
    use core::fmt;
    use serde::{de, ser};

    use super::DateTime;
    use crate::format::{write_rfc3339, SecondsFormat};
    use crate::offset::{FixedOffset, Local, Offset, TimeZone, Utc};

    impl<Tz: TimeZone> ser::Serialize for DateTime<Tz> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            struct FormatIso8601<'a, Tz: TimeZone> {
                inner: &'a DateTime<Tz>,
            }

            impl<Tz: TimeZone> fmt::Display for FormatIso8601<'_, Tz> {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    let naive = self.inner.naive_local();
                    let offset = self.inner.offset.fix();
                    write_rfc3339(f, naive, offset, SecondsFormat::AutoSi, true)
                }
            }

            serializer.collect_str(&FormatIso8601 { inner: self })
        }
    }

    struct DateTimeVisitor;

    impl de::Visitor<'_> for DateTimeVisitor {
        type Value = DateTime<FixedOffset>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an RFC 3339 formatted date and time string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(E::custom)
        }
    }

    impl<'de> de::Deserialize<'de> for DateTime<FixedOffset> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(DateTimeVisitor)
        }
    }

    impl<'de> de::Deserialize<'de> for DateTime<Utc> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(DateTimeVisitor).map(|dt| dt.with_timezone(&Utc))
        }
    }

    impl<'de> de::Deserialize<'de> for DateTime<Local> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(DateTimeVisitor).map(|dt| dt.with_timezone(&Local))
        }
    }

    /// Builds a `de::Error` for a timestamp value that could not be
    /// converted back to a `DateTime<Utc>`. Shared by every `ts_*`
    /// submodule below.
    pub(crate) fn invalid_ts<E: de::Error, T: fmt::Display>(value: T) -> E {
        E::custom(format_args!("value is not a legal timestamp: {value}"))
    }

    /// Same expansion as the `ts_unit!` macro in `naive::datetime::serde`,
    /// but for `DateTime<Utc>` directly (no `.and_utc()`/`.naive_utc()`
    /// round trip needed).
    macro_rules! ts_unit {
        ($unit:ident, $unit_option:ident, $units_per_sec:expr, $get_value:expr, $expecting:literal) => {
            #[doc = concat!("(De)serialize a `DateTime<Utc>` as a Unix timestamp in ", $expecting, ".")]
            pub mod $unit {
                use core::fmt;
                use serde::{de, ser};

                use super::invalid_ts;
                use crate::{DateTime, Utc};

                const UNITS_PER_SEC: i64 = $units_per_sec;
                const NANOS_PER_UNIT: i64 = 1_000_000_000 / UNITS_PER_SEC;

                /// Serializes into an integer timestamp. Intended for use
                /// with serde's `serialize_with` attribute.
                pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ser::Serializer,
                {
                    let get_value: fn(&DateTime<Utc>) -> Result<i64, &'static str> = $get_value;
                    serializer.serialize_i64(get_value(dt).map_err(ser::Error::custom)?)
                }

                /// Deserializes from an integer timestamp. Intended for use
                /// with serde's `deserialize_with` attribute.
                pub fn deserialize<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
                where
                    D: de::Deserializer<'de>,
                {
                    d.deserialize_i64(TsVisitor)
                }

                pub(super) struct TsVisitor;

                impl de::Visitor<'_> for TsVisitor {
                    type Value = DateTime<Utc>;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a unix timestamp")
                    }

                    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                        let secs = value.div_euclid(UNITS_PER_SEC);
                        let nsecs = (value.rem_euclid(UNITS_PER_SEC) * NANOS_PER_UNIT) as u32;
                        DateTime::from_timestamp(secs, nsecs).ok_or_else(|| invalid_ts(value))
                    }

                    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                        let units_per_sec = UNITS_PER_SEC as u64;
                        let secs = (value / units_per_sec) as i64;
                        let nsecs = ((value % units_per_sec) * NANOS_PER_UNIT as u64) as u32;
                        DateTime::from_timestamp(secs, nsecs).ok_or_else(|| invalid_ts(value))
                    }
                }
            }

            #[doc = concat!("(De)serialize an `Option<DateTime<Utc>>` as a Unix timestamp in ", $expecting, ", or `None`.")]
            pub mod $unit_option {
                use core::fmt;
                use serde::{de, ser};

                use super::$unit::TsVisitor;
                use crate::{DateTime, Utc};

                /// Serializes into an integer timestamp, or `null`.
                pub fn serialize<S>(
                    opt: &Option<DateTime<Utc>>,
                    serializer: S,
                ) -> Result<S::Ok, S::Error>
                where
                    S: ser::Serializer,
                {
                    let get_value: fn(&DateTime<Utc>) -> Result<i64, &'static str> = $get_value;
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
                    type Value = Option<DateTime<Utc>>;

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
                pub fn deserialize<'de, D>(d: D) -> Result<Option<DateTime<Utc>>, D::Error>
                where
                    D: de::Deserializer<'de>,
                {
                    d.deserialize_option(OptionTsVisitor)
                }
            }
        };
    }

    ts_unit!(ts_seconds, ts_seconds_option, 1, |dt| Ok(dt.timestamp()), "seconds");
    ts_unit!(
        ts_milliseconds,
        ts_milliseconds_option,
        1_000,
        |dt| Ok(dt.timestamp_millis()),
        "milliseconds"
    );
    ts_unit!(
        ts_microseconds,
        ts_microseconds_option,
        1_000_000,
        |dt| Ok(dt.timestamp_micros()),
        "microseconds"
    );
    ts_unit!(
        ts_nanoseconds,
        ts_nanoseconds_option,
        1_000_000_000,
        |dt| dt
            .timestamp_nanos_opt()
            .ok_or("value out of range for a timestamp with nanosecond precision"),
        "nanoseconds"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // `1_700_000_000` is a well-known round Unix timestamp: 2023-11-14
    // 22:13:20 UTC (hand-verified via the epoch-day/calendar arithmetic
    // also used to build `calendar.rs`'s own tests).

    #[test]
    fn from_timestamp_and_timestamp_round_trip() {
        let dt = DateTime::<Utc>::from_timestamp(1_700_000_000, 500_000_000).unwrap();
        assert_eq!(dt.timestamp(), 1_700_000_000);
        assert_eq!(dt.timestamp_subsec_nanos(), 500_000_000);
        assert_eq!((dt.year(), dt.month(), dt.day()), (2023, 11, 14));
        assert_eq!((dt.hour(), dt.minute(), dt.second()), (22, 13, 20));
    }

    #[test]
    fn from_timestamp_secs_matches_from_timestamp_with_zero_nanos() {
        assert_eq!(DateTime::<Utc>::from_timestamp_secs(0), DateTime::<Utc>::from_timestamp(0, 0));
    }

    #[test]
    fn from_timestamp_millis_micros_nanos_agree() {
        let from_secs = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let from_millis = DateTime::<Utc>::from_timestamp_millis(1_700_000_000_000).unwrap();
        let from_micros = DateTime::<Utc>::from_timestamp_micros(1_700_000_000_000_000).unwrap();
        let from_nanos = DateTime::<Utc>::from_timestamp_nanos(1_700_000_000_000_000_000);
        assert_eq!(from_secs, from_millis);
        assert_eq!(from_secs, from_micros);
        assert_eq!(from_secs, from_nanos);
    }

    #[test]
    fn from_timestamp_rejects_out_of_range_seconds() {
        assert!(DateTime::<Utc>::from_timestamp(i64::MAX, 0).is_none());
        assert!(DateTime::<Utc>::from_timestamp(i64::MIN, 0).is_none());
    }

    #[test]
    fn from_timestamp_accepts_leap_second_nanos() {
        // `secs_of_day % 60 == 59` is required for the leap-second nanos
        // range to be valid; `secs = 59` lands exactly on a `:59` mark.
        let dt = DateTime::<Utc>::from_timestamp(59, 1_500_000_000).unwrap();
        assert_eq!(dt.second(), 59); // `second()` never reports 60.
        assert_eq!(dt.nanosecond(), 1_500_000_000);
    }

    #[test]
    fn unix_epoch_constant_is_1970_01_01() {
        let epoch = DateTime::<Utc>::UNIX_EPOCH;
        assert_eq!(epoch.timestamp(), 0);
        assert_eq!((epoch.year(), epoch.month(), epoch.day()), (1970, 1, 1));
    }

    #[test]
    fn timestamp_millis_micros_nanos_opt_with_subsecond() {
        let dt = DateTime::<Utc>::from_timestamp(1_700_000_000, 123_456_789).unwrap();
        assert_eq!(dt.timestamp_millis(), 1_700_000_000_123);
        assert_eq!(dt.timestamp_micros(), 1_700_000_000_123_456);
        assert_eq!(dt.timestamp_nanos_opt(), Some(1_700_000_000_123_456_789));
        assert_eq!(dt.timestamp_subsec_millis(), 123);
        assert_eq!(dt.timestamp_subsec_micros(), 123_456);
        assert_eq!(dt.timestamp_subsec_nanos(), 123_456_789);
    }

    #[test]
    fn timestamp_nanos_opt_is_none_far_from_epoch() {
        // An `i64` of nanoseconds spans only ~584 years; year 9999 is far
        // past that range from the 1970 epoch.
        let far_future = NaiveDate::from_ymd_opt(9999, 1, 1).unwrap().and_time(NaiveTime::MIN).and_utc();
        assert!(far_future.timestamp_nanos_opt().is_none());
    }

    #[test]
    fn with_timezone_preserves_the_instant() {
        let utc = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let offset = FixedOffset::east_opt(3600).unwrap();
        let fixed = utc.with_timezone(&offset);
        assert_eq!(fixed.timestamp(), utc.timestamp());
        assert_eq!(fixed.hour(), (utc.hour() + 1) % 24);
        let back = fixed.with_timezone(&Utc);
        assert_eq!(back, utc);
    }

    #[test]
    fn fixed_offset_and_to_utc_round_trip() {
        let utc = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let fixed = utc.fixed_offset();
        assert_eq!(fixed.offset(), &FixedOffset::east_opt(0).unwrap());
        assert_eq!(fixed.to_utc(), utc);
    }

    #[test]
    fn checked_add_signed_and_sub_signed() {
        let dt = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let later = dt.checked_add_signed(TimeDelta::seconds(3600)).unwrap();
        assert_eq!(later.timestamp(), dt.timestamp() + 3600);
        let earlier = dt.checked_sub_signed(TimeDelta::seconds(3600)).unwrap();
        assert_eq!(earlier.timestamp(), dt.timestamp() - 3600);
    }

    #[test]
    fn checked_add_signed_none_past_max() {
        assert!(DateTime::<Utc>::MAX_UTC.checked_add_signed(TimeDelta::seconds(1)).is_none());
    }

    #[test]
    fn checked_add_months_clamps_to_end_of_month() {
        let dt = Utc.with_ymd_and_hms(2023, 1, 31, 0, 0, 0).unwrap();
        let next = dt.checked_add_months(Months::new(1)).unwrap();
        assert_eq!((next.year(), next.month(), next.day()), (2023, 2, 28));
    }

    #[test]
    fn checked_sub_months_clamps_to_end_of_month() {
        let dt = Utc.with_ymd_and_hms(2023, 3, 31, 0, 0, 0).unwrap();
        let prev = dt.checked_sub_months(Months::new(1)).unwrap();
        assert_eq!((prev.year(), prev.month(), prev.day()), (2023, 2, 28));
    }

    #[test]
    fn checked_add_days_and_sub_days() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        let later = dt.checked_add_days(Days::new(10)).unwrap();
        assert_eq!((later.year(), later.month(), later.day()), (2023, 6, 25));
        let earlier = dt.checked_sub_days(Days::new(10)).unwrap();
        assert_eq!((earlier.year(), earlier.month(), earlier.day()), (2023, 6, 5));
    }

    #[test]
    fn checked_add_days_zero_is_identity() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        assert_eq!(dt.checked_add_days(Days::new(0)).unwrap(), dt);
    }

    #[test]
    fn signed_duration_since_computes_the_difference() {
        let a = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        let b = Utc.with_ymd_and_hms(2023, 6, 14, 12, 0, 0).unwrap();
        assert_eq!(a.signed_duration_since(b), TimeDelta::days(1));
    }

    #[test]
    fn years_since_counts_whole_elapsed_years() {
        let base = Utc.with_ymd_and_hms(2020, 6, 15, 0, 0, 0).unwrap();
        let three_years_later = Utc.with_ymd_and_hms(2023, 6, 15, 0, 0, 0).unwrap();
        assert_eq!(three_years_later.years_since(base), Some(3));

        let day_before_anniversary = Utc.with_ymd_and_hms(2023, 6, 14, 0, 0, 0).unwrap();
        assert_eq!(day_before_anniversary.years_since(base), Some(2));
    }

    #[test]
    fn years_since_is_none_when_base_is_later() {
        let base = Utc.with_ymd_and_hms(2023, 6, 15, 0, 0, 0).unwrap();
        let earlier = Utc.with_ymd_and_hms(2020, 6, 15, 0, 0, 0).unwrap();
        assert_eq!(earlier.years_since(base), None);
    }

    #[test]
    fn age_is_years_since_with_reversed_argument_order() {
        let date_of_birth = Utc.with_ymd_and_hms(1990, 6, 15, 0, 0, 0).unwrap();
        let day_before_birthday = Utc.with_ymd_and_hms(2023, 6, 14, 0, 0, 0).unwrap();
        let birthday = Utc.with_ymd_and_hms(2023, 6, 15, 0, 0, 0).unwrap();
        assert_eq!(date_of_birth.age(day_before_birthday), Some(32));
        assert_eq!(date_of_birth.age(birthday), Some(33));
        assert_eq!(date_of_birth.age(birthday), birthday.years_since(date_of_birth));
    }

    #[test]
    fn age_is_none_when_reference_instant_precedes_date_of_birth() {
        let date_of_birth = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        let earlier = Utc.with_ymd_and_hms(1999, 12, 31, 0, 0, 0).unwrap();
        assert_eq!(date_of_birth.age(earlier), None);
    }

    #[test]
    fn with_time_replaces_time_of_day_keeping_date() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 8, 0, 0).unwrap();
        let new_time = NaiveTime::from_hms_opt(20, 30, 0).unwrap();
        let updated = dt.with_time(new_time).unwrap();
        assert_eq!((updated.year(), updated.month(), updated.day()), (2023, 6, 15));
        assert_eq!((updated.hour(), updated.minute()), (20, 30));
    }

    #[test]
    fn datelike_and_timelike_mutators() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 10, 30, 45).unwrap();
        assert_eq!(dt.with_year(2024).unwrap().year(), 2024);
        assert_eq!(dt.with_month(1).unwrap().month(), 1);
        assert_eq!(dt.with_day(1).unwrap().day(), 1);
        assert_eq!(dt.with_hour(0).unwrap().hour(), 0);
        assert_eq!(dt.with_minute(0).unwrap().minute(), 0);
        assert_eq!(dt.with_second(0).unwrap().second(), 0);
        assert!(dt.with_month(13).is_none());
        assert!(dt.with_day(32).is_none());
    }

    #[test]
    fn to_rfc2822_format() {
        // 2003-07-01 is hand-verified as a Tuesday (see `calendar.rs`'s
        // weekday tests for the same anchoring technique).
        let dt = Utc.with_ymd_and_hms(2003, 7, 1, 10, 52, 37).unwrap();
        assert_eq!(dt.to_rfc2822(), "Tue, 1 Jul 2003 10:52:37 +0000");
    }

    #[test]
    fn to_rfc3339_format_with_fixed_offset() {
        let offset = FixedOffset::west_opt(8 * 3600).unwrap();
        let dt = offset.with_ymd_and_hms(1996, 12, 19, 16, 39, 57).unwrap();
        assert_eq!(dt.to_rfc3339(), "1996-12-19T16:39:57-08:00");
    }

    #[test]
    fn to_rfc3339_for_utc_uses_plus_00_00_not_z() {
        // Distinct from the `serde` impl below, which always uses `Z` for
        // a zero offset: `to_rfc3339` hard-codes `use_z = false`.
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 9, 30, 0).unwrap();
        assert_eq!(dt.to_rfc3339(), "2023-06-15T09:30:00+00:00");
    }

    #[test]
    fn to_rfc3339_opts_uses_z_when_requested_and_utc() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        assert_eq!(dt.to_rfc3339_opts(SecondsFormat::Secs, true), "2023-06-15T12:00:00Z");
        assert_eq!(dt.to_rfc3339_opts(SecondsFormat::Secs, false), "2023-06-15T12:00:00+00:00");
    }

    #[test]
    fn format_with_strftime_pattern() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 9, 5, 3).unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2023-06-15 09:05:03");
    }

    #[test]
    fn parse_from_rfc2822_round_trips() {
        let parsed = DateTime::<FixedOffset>::parse_from_rfc2822("Tue, 1 Jul 2003 10:52:37 +0200").unwrap();
        assert_eq!(parsed.year(), 2003);
        assert_eq!(parsed.offset().local_minus_utc(), 7200);
    }

    #[test]
    fn parse_from_rfc3339_round_trips() {
        let parsed = DateTime::<FixedOffset>::parse_from_rfc3339("1996-12-19T16:39:57-08:00").unwrap();
        assert_eq!((parsed.year(), parsed.month(), parsed.day()), (1996, 12, 19));
        assert_eq!(parsed.offset().local_minus_utc(), -8 * 3600);
    }

    #[test]
    fn parse_from_str_requires_a_timezone() {
        let parsed =
            DateTime::<FixedOffset>::parse_from_str("2023-06-15 12:00:00 +0100", "%Y-%m-%d %H:%M:%S %z")
                .unwrap();
        assert_eq!(parsed.offset().local_minus_utc(), 3600);
    }

    #[test]
    fn parse_and_remainder_returns_leftover_input() {
        let (dt, rest) = DateTime::<FixedOffset>::parse_and_remainder(
            "2023-06-15 12:00:00 +0000 trailing",
            "%Y-%m-%d %H:%M:%S %z",
        )
        .unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(rest, " trailing");
    }

    #[test]
    fn fromstr_for_datetime_utc_parses_rfc3339() {
        let dt: DateTime<Utc> = "2023-06-15T12:00:00+02:00".parse().unwrap();
        assert_eq!(dt.hour(), 10); // shifted from +02:00 wall time to UTC.
        assert_eq!(dt.to_rfc3339(), "2023-06-15T10:00:00+00:00");
    }

    #[test]
    fn default_is_unix_epoch() {
        assert_eq!(DateTime::<Utc>::default(), DateTime::<Utc>::UNIX_EPOCH);
    }

    #[test]
    fn default_local_round_trips_to_unix_epoch_in_utc() {
        // Written to hold regardless of the system's configured time
        // zone: only the *instant*, not the offset, is asserted.
        let default_local = DateTime::<Local>::default();
        assert_eq!(default_local.with_timezone(&Utc), DateTime::<Utc>::UNIX_EPOCH);
    }

    #[test]
    fn from_utc_to_fixed_offset_uses_zero_offset() {
        let utc = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let fixed: DateTime<FixedOffset> = utc.into();
        assert_eq!(fixed.offset(), &FixedOffset::east_opt(0).unwrap());
        assert_eq!(fixed.timestamp(), utc.timestamp());
    }

    #[test]
    fn from_fixed_offset_to_utc_preserves_instant() {
        let offset = FixedOffset::east_opt(3600).unwrap();
        let fixed = offset.with_ymd_and_hms(2023, 6, 15, 13, 0, 0).unwrap();
        let utc: DateTime<Utc> = fixed.into();
        assert_eq!(utc.hour(), 12);
    }

    #[test]
    fn operator_add_sub_time_delta() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        assert_eq!((dt + TimeDelta::hours(1)).hour(), 13);
        assert_eq!((dt - TimeDelta::hours(1)).hour(), 11);
    }

    #[test]
    fn operator_add_sub_std_duration() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        let std_dur = core::time::Duration::from_secs(3600);
        assert_eq!((dt + std_dur).hour(), 13);
        assert_eq!((dt - std_dur).hour(), 11);
    }

    #[test]
    fn operator_add_sub_months_and_days() {
        let dt = Utc.with_ymd_and_hms(2023, 1, 31, 0, 0, 0).unwrap();
        assert_eq!((dt + Months::new(1)).day(), 28);
        let dt2 = Utc.with_ymd_and_hms(2023, 6, 15, 0, 0, 0).unwrap();
        assert_eq!((dt2 + Days::new(5)).day(), 20);
        assert_eq!((dt2 - Days::new(5)).day(), 10);
    }

    #[test]
    fn operator_sub_datetime_gives_duration() {
        let a = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        let b = Utc.with_ymd_and_hms(2023, 6, 15, 10, 0, 0).unwrap();
        assert_eq!(a - b, TimeDelta::hours(2));
    }

    #[test]
    #[should_panic]
    fn operator_add_time_delta_panics_on_overflow() {
        let _ = DateTime::<Utc>::MAX_UTC + TimeDelta::seconds(1);
    }

    #[test]
    fn add_assign_and_sub_assign_time_delta() {
        let mut dt = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        dt += TimeDelta::hours(1);
        assert_eq!(dt.hour(), 13);
        dt -= TimeDelta::hours(2);
        assert_eq!(dt.hour(), 11);
    }

    #[test]
    fn comparisons_are_based_on_the_instant_not_the_timezone() {
        let utc = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        let fixed = utc.with_timezone(&FixedOffset::east_opt(3600).unwrap());
        assert_eq!(utc, fixed);
        assert!(utc <= fixed);
        assert!(fixed <= utc);
    }

    #[test]
    fn debug_and_display_include_offset() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 9, 30, 0).unwrap();
        assert_eq!(format!("{dt:?}"), "2023-06-15T09:30:00Z");
        assert_eq!(dt.to_string(), "2023-06-15 09:30:00 UTC");
    }

    #[test]
    fn min_and_max_utc_are_ordered() {
        assert!(DateTime::<Utc>::MIN_UTC < DateTime::<Utc>::MAX_UTC);
    }

    #[test]
    fn from_system_time_round_trips() {
        let st = std::time::UNIX_EPOCH + std::time::Duration::new(1_700_000_000, 500_000_000);
        let dt: DateTime<Utc> = st.into();
        assert_eq!(dt.timestamp(), 1_700_000_000);
        assert_eq!(dt.timestamp_subsec_nanos(), 500_000_000);
        let back: std::time::SystemTime = dt.into();
        assert_eq!(back, st);
    }

    #[test]
    fn from_system_time_before_epoch() {
        let st = std::time::UNIX_EPOCH - std::time::Duration::new(100, 0);
        let dt: DateTime<Utc> = st.into();
        assert_eq!(dt.timestamp(), -100);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trips_via_rfc3339() {
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 9, 30, 0).unwrap();
        let json = serde_json::to_string(&dt).unwrap();
        assert_eq!(json, "\"2023-06-15T09:30:00Z\"");
        let back: DateTime<Utc> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, dt);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn ts_seconds_serde_helper_round_trips_for_datetime_utc() {
        #[derive(::serde::Serialize, ::serde::Deserialize, PartialEq, Debug)]
        struct Wrapper {
            #[serde(with = "super::serde::ts_seconds")]
            dt: DateTime<Utc>,
        }
        let original = Wrapper { dt: Utc.with_ymd_and_hms(2023, 6, 15, 9, 30, 0).unwrap() };
        let json = serde_json::to_string(&original).unwrap();
        let back: Wrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
    }
}
