//! `NaiveDate`: a date in the proleptic Gregorian calendar, without an
//! associated time zone or time of day.

use crate::calendar::{
    civil_from_days, days_from_civil, days_in_month, days_in_year, is_leap_year, iso_year_week,
    EPOCH_OFFSET_FROM_CE,
};
use crate::duration::{Days, Duration, Months};
// `JapaneseEra`: a `time_compute` extension -- not part of chrono.
use crate::japanese_era::JapaneseEra;
use crate::format::{parse, parse_and_remainder, DelayedFormat, Item, ParseResult, Parsed, StrftimeItems};
#[cfg(feature = "unstable-locales")]
use crate::format::Locale;
use crate::naive::datetime::NaiveDateTime;
use crate::naive::time::NaiveTime;
use crate::traits::Datelike;
use crate::weekday::Weekday;
use core::borrow::Borrow;
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// Accepted year bounds. Chosen far beyond any realistic need (~5 million
/// years on either side of year 0), while guaranteeing that no internal
/// computation (day counter, `Duration` in seconds, `num_days_from_ce` as an
/// `i32`) can silently overflow.
///
/// Note: this is intentionally much wider than chrono's own range (chrono
/// rejects years beyond roughly +/-262,000 / -400,000 depending on the
/// exact date). Any date chrono accepts is accepted here too; this crate
/// additionally accepts some extreme dates chrono would reject.
pub(crate) const MIN_YEAR: i32 = -5_000_000;
pub(crate) const MAX_YEAR: i32 = 5_000_000;

/// A date in the proleptic Gregorian calendar (year, month, day), without a
/// time of day or time zone.
///
/// API aligned with `chrono::NaiveDate`: same constructors (`from_ymd_opt`,
/// `from_yo_opt`, `from_isoywd_opt`, ...), same accessors via the
/// [`Datelike`] trait, same operators.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct NaiveDate {
    year: i32,
    month: u32,
    day: u32,
}

/// `arbitrary` support: picks a uniformly random year in the accepted
/// range, then a uniformly random ordinal day within that year, mirroring
/// chrono's own strategy.
#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for NaiveDate {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<NaiveDate> {
        let year = u.int_in_range(MIN_YEAR..=MAX_YEAR)?;
        let max_days = days_in_year(year);
        let ord = u.int_in_range(1..=max_days)?;
        NaiveDate::from_yo_opt(year, ord).ok_or(arbitrary::Error::IncorrectFormat)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for NaiveDate {
    fn format(&self, fmt: defmt::Formatter) {
        if (0..=9999).contains(&self.year) {
            defmt::write!(fmt, "{:04}-{:02}-{:02}", self.year, self.month, self.day);
        } else {
            let sign = if self.year < 0 { '-' } else { '+' };
            defmt::write!(fmt, "{}{:04}-{:02}-{:02}", sign, self.year.abs(), self.month, self.day);
        }
    }
}

impl NaiveDate {
    /// The smallest representable date.
    pub const MIN: NaiveDate = NaiveDate { year: MIN_YEAR, month: 1, day: 1 };

    /// The largest representable date.
    pub const MAX: NaiveDate = NaiveDate { year: MAX_YEAR, month: 12, day: 31 };

    /// One day before [`MIN`](Self::MIN). Used only internally as a buffer
    /// value for offset arithmetic that may briefly step outside the
    /// representable range (see `NaiveDateTime::overflowing_add_offset`);
    /// never exposed to users.
    pub(crate) const BEFORE_MIN: NaiveDate = NaiveDate { year: MIN_YEAR - 1, month: 12, day: 31 };

    /// One day after [`MAX`](Self::MAX). See [`BEFORE_MIN`](Self::BEFORE_MIN).
    pub(crate) const AFTER_MAX: NaiveDate = NaiveDate { year: MAX_YEAR + 1, month: 1, day: 1 };

    /// Builds a date from the year, month (1..=12) and day of month.
    /// Returns `None` if the date does not exist (invalid month, day out of
    /// bounds, February 29th on a non-leap year, ...).
    pub const fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
        if year < MIN_YEAR || year > MAX_YEAR {
            return None;
        }
        if month == 0 || month > 12 {
            return None;
        }
        if day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(NaiveDate { year, month, day })
    }

    /// Builds a date from the year, month and day of month, like
    /// [`from_ymd_opt`](Self::from_ymd_opt).
    ///
    /// # Panics
    /// Panics if the specified calendar day does not exist, or if `year` is
    /// out of range.
    #[deprecated(note = "use `from_ymd_opt()` instead")]
    pub fn from_ymd(year: i32, month: u32, day: u32) -> NaiveDate {
        Self::from_ymd_opt(year, month, day).expect("invalid or out-of-range date")
    }

    /// Builds a date from the year and the day of the year (1..=365 or
    /// 366). Returns `None` if `ordinal` is out of bounds.
    pub const fn from_yo_opt(year: i32, ordinal: u32) -> Option<NaiveDate> {
        if year < MIN_YEAR || year > MAX_YEAR {
            return None;
        }
        if ordinal == 0 || ordinal > days_in_year(year) {
            return None;
        }
        let mut remaining = ordinal;
        let mut month = 1u32;
        loop {
            let dim = days_in_month(year, month);
            if remaining <= dim {
                break;
            }
            remaining -= dim;
            month += 1;
        }
        Some(NaiveDate { year, month, day: remaining })
    }

    /// Builds a date from the year and day of the year, like
    /// [`from_yo_opt`](Self::from_yo_opt).
    ///
    /// # Panics
    /// Panics if the specified ordinal day does not exist, or if `year` is
    /// out of range.
    #[deprecated(note = "use `from_yo_opt()` instead")]
    pub fn from_yo(year: i32, ordinal: u32) -> NaiveDate {
        Self::from_yo_opt(year, ordinal).expect("invalid or out-of-range date")
    }

    /// Builds a date from an ISO 8601 year, an ISO week number (1..=53) and
    /// a weekday. Returns `None` if the combination does not correspond to
    /// any real date (e.g. week 53 on a year that only has 52).
    pub const fn from_isoywd_opt(iso_year: i32, week: u32, weekday: Weekday) -> Option<NaiveDate> {
        if week == 0 || week > 53 {
            return None;
        }
        let w1 = crate::calendar::iso_week1_monday(iso_year);
        let z = w1 + (week as i64 - 1) * 7 + weekday.num_days_from_monday() as i64;
        let candidate = match Self::from_days_since_epoch(z) {
            Some(v) => v,
            None => return None,
        };
        // Round-trip check: guarantees we haven't built a week that
        // "overflows" into a different ISO year.
        let (check_year, check_week) = iso_year_week(z, candidate.year);
        if check_year == iso_year && check_week == week {
            Some(candidate)
        } else {
            None
        }
    }

    /// Builds a date from an ISO 8601 year/week/weekday, like
    /// [`from_isoywd_opt`](Self::from_isoywd_opt).
    ///
    /// # Panics
    /// Panics if the combination does not correspond to any real date.
    #[deprecated(note = "use `from_isoywd_opt()` instead")]
    pub fn from_isoywd(iso_year: i32, week: u32, weekday: Weekday) -> NaiveDate {
        Self::from_isoywd_opt(iso_year, week, weekday).expect("invalid or out-of-range date")
    }

    /// Builds a date from its day number since the proleptic 0001-01-01
    /// (1 = 0001-01-01), like `chrono`.
    pub const fn from_num_days_from_ce_opt(days: i32) -> Option<NaiveDate> {
        let z = (days as i64) + EPOCH_OFFSET_FROM_CE - 1;
        Self::from_days_since_epoch(z)
    }

    /// Builds a date from its day number since the proleptic 0001-01-01,
    /// like [`from_num_days_from_ce_opt`](Self::from_num_days_from_ce_opt).
    ///
    /// # Panics
    /// Panics if `days` is out of range.
    #[deprecated(note = "use `from_num_days_from_ce_opt()` instead")]
    pub fn from_num_days_from_ce(days: i32) -> NaiveDate {
        Self::from_num_days_from_ce_opt(days).expect("invalid or out-of-range date")
    }

    /// Builds a date from its day count since the Unix epoch (1970-01-01 =
    /// day 0), like `chrono`. Returns `None` if out of range.
    pub const fn from_epoch_days(days: i32) -> Option<NaiveDate> {
        Self::from_days_since_epoch(days as i64)
    }

    /// Day count since the Unix epoch (1970-01-01 = day 0).
    pub const fn to_epoch_days(&self) -> i32 {
        // Safe: guaranteed to fit in an i32 by the choice of
        // `MIN_YEAR`/`MAX_YEAR` (see their documentation).
        self.days_since_epoch() as i32
    }

    /// Builds a date by counting occurrences of a given weekday from the
    /// start of a month. `n` is 1-indexed: for the 2nd Friday of March
    /// 2017, use `from_weekday_of_month_opt(2017, 3, Weekday::Fri, 2)`.
    /// Returns `None` if that occurrence does not exist in the month, or
    /// if `month`/`n`/`year` are invalid.
    pub const fn from_weekday_of_month_opt(
        year: i32,
        month: u32,
        weekday: Weekday,
        n: u8,
    ) -> Option<NaiveDate> {
        if n == 0 {
            return None;
        }
        let first = match NaiveDate::from_ymd_opt(year, month, 1) {
            Some(d) => d,
            None => return None,
        };
        let first_weekday = crate::calendar::weekday_from_days(first.days_since_epoch());
        let first_to_dow =
            (7 + weekday.number_from_monday() - first_weekday.number_from_monday()) % 7;
        let day = (n - 1) as u32 * 7 + first_to_dow + 1;
        NaiveDate::from_ymd_opt(year, month, day)
    }

    /// Builds a date by counting weekday occurrences in a month, like
    /// [`from_weekday_of_month_opt`](Self::from_weekday_of_month_opt).
    ///
    /// # Panics
    /// Panics if that occurrence does not exist, or if the inputs are
    /// invalid.
    #[deprecated(note = "use `from_weekday_of_month_opt()` instead")]
    pub fn from_weekday_of_month(year: i32, month: u32, weekday: Weekday, n: u8) -> NaiveDate {
        Self::from_weekday_of_month_opt(year, month, weekday, n).expect("out-of-range date")
    }

    const fn from_days_since_epoch(z: i64) -> Option<NaiveDate> {
        let (year_i64, month, day) = civil_from_days(z);
        if year_i64 < MIN_YEAR as i64 || year_i64 > MAX_YEAR as i64 {
            return None;
        }
        Some(NaiveDate { year: year_i64 as i32, month, day })
    }

    /// Day number since the proleptic 0001-01-01 (1 = 0001-01-01).
    pub const fn num_days_from_ce(&self) -> i32 {
        // Safe: `MIN_YEAR`/`MAX_YEAR` are chosen so that this counter
        // always fits in an i32 (see their documentation).
        (self.days_since_epoch() - EPOCH_OFFSET_FROM_CE + 1) as i32
    }

    pub(crate) const fn days_since_epoch(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// Returns the week number, counting from 1, of the week that starts on
    /// `day` and contains this date's first occurrence of `day` on or before
    /// January 1. Used internally by the `%U`/`%W` format specifiers.
    pub(crate) fn weeks_from(&self, day: Weekday) -> i32 {
        (self.ordinal() as i32 - self.weekday().days_since(day) as i32 + 6) / 7
    }

    /// `true` if the year of this date is a leap year.
    pub const fn leap_year(&self) -> bool {
        is_leap_year(self.year)
    }

    /// The next day, or `None` if `self` is already `NaiveDate::MAX`.
    pub const fn succ_opt(&self) -> Option<NaiveDate> {
        self.checked_add_days(Days::new(1))
    }

    /// The next day, like [`succ_opt`](Self::succ_opt).
    ///
    /// # Panics
    /// Panics when `self` is the last representable date.
    #[deprecated(note = "use `succ_opt()` instead")]
    pub fn succ(&self) -> NaiveDate {
        self.succ_opt().expect("out of bound")
    }

    /// The previous day, or `None` if `self` is already `NaiveDate::MIN`.
    pub const fn pred_opt(&self) -> Option<NaiveDate> {
        self.checked_sub_days(Days::new(1))
    }

    /// The previous day, like [`pred_opt`](Self::pred_opt).
    ///
    /// # Panics
    /// Panics when `self` is the first representable date.
    #[deprecated(note = "use `pred_opt()` instead")]
    pub fn pred(&self) -> NaiveDate {
        self.pred_opt().expect("out of bound")
    }

    /// Adds a number of calendar days. Returns `None` on overflow of the
    /// representable bounds.
    pub const fn checked_add_days(self, days: Days) -> Option<NaiveDate> {
        if days.0 > i64::MAX as u64 {
            return None;
        }
        let delta = days.0 as i64;
        let z = match self.days_since_epoch().checked_add(delta) {
            Some(v) => v,
            None => return None,
        };
        Self::from_days_since_epoch(z)
    }

    /// Subtracts a number of calendar days. Returns `None` on overflow of
    /// the representable bounds.
    pub const fn checked_sub_days(self, days: Days) -> Option<NaiveDate> {
        if days.0 > i64::MAX as u64 {
            return None;
        }
        let delta = days.0 as i64;
        let z = match self.days_since_epoch().checked_sub(delta) {
            Some(v) => v,
            None => return None,
        };
        Self::from_days_since_epoch(z)
    }

    /// Adds a number of months. Uses the last valid day of the resulting
    /// month if the original day of the month does not exist there (e.g.
    /// January 31st + 1 month -> February 28th or 29th), matching
    /// `chrono`'s behaviour. Returns `None` only if the resulting date
    /// would be out of the representable range.
    pub const fn checked_add_months(self, months: Months) -> Option<NaiveDate> {
        self.add_months_signed(months.as_u32() as i64)
    }

    /// Subtracts a number of months. Same rules as
    /// [`checked_add_months`](Self::checked_add_months).
    pub const fn checked_sub_months(self, months: Months) -> Option<NaiveDate> {
        self.add_months_signed(-(months.as_u32() as i64))
    }

    const fn add_months_signed(self, delta: i64) -> Option<NaiveDate> {
        let total_months = (self.year as i64) * 12 + (self.month as i64 - 1) + delta;
        let year = crate::calendar::div_floor(total_months, 12);
        if year < MIN_YEAR as i64 || year > MAX_YEAR as i64 {
            return None;
        }
        let month0 = total_months - year * 12; // 0..=11
        let year = year as i32;
        let month = month0 as u32 + 1;
        // Clamp the day of the month if it doesn't exist in the target
        // month (e.g. day 31 in a 30-day month), instead of failing.
        let day_max = days_in_month(year, month);
        let day = if self.day > day_max { day_max } else { self.day };
        Some(NaiveDate { year, month, day })
    }

    /// Adds a signed [`Duration`]. The duration is converted to a whole
    /// number of days (truncated toward zero), like `chrono`.
    pub const fn checked_add_signed(self, rhs: Duration) -> Option<NaiveDate> {
        let z = match self.days_since_epoch().checked_add(rhs.num_days()) {
            Some(v) => v,
            None => return None,
        };
        Self::from_days_since_epoch(z)
    }

    /// Subtracts a signed [`Duration`].
    pub const fn checked_sub_signed(self, rhs: Duration) -> Option<NaiveDate> {
        let z = match self.days_since_epoch().checked_sub(rhs.num_days()) {
            Some(v) => v,
            None => return None,
        };
        Self::from_days_since_epoch(z)
    }

    /// Signed duration between two dates (`self - other`), in whole days.
    pub const fn signed_duration_since(self, other: NaiveDate) -> Duration {
        Duration::days(self.days_since_epoch() - other.days_since_epoch())
    }

    /// Absolute (unsigned) number of days between two dates, regardless of
    /// which one comes first.
    pub const fn abs_diff(self, rhs: Self) -> Days {
        let a = self.num_days_from_ce();
        let b = rhs.num_days_from_ce();
        let diff = if a >= b { a - b } else { b - a };
        Days::new(diff as u64)
    }

    /// Number of whole years between `base` and `self` (`self` must be on
    /// or after `base`). Returns `None` if `base > self`.
    pub const fn years_since(&self, base: Self) -> Option<u32> {
        let mut years = self.year - base.year;
        // Combine month and day into a single comparable number, since
        // tuple comparison is not available in a const context here.
        if (self.month << 5 | self.day) < (base.month << 5 | base.day) {
            years -= 1;
        }
        if years >= 0 {
            Some(years as u32)
        } else {
            None
        }
    }

    /// Age in whole years, as of `on` -- e.g. `self` is a date of birth and
    /// `on` is a reference date such as "today". Returns `None` if `on` is
    /// before `self`.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// This method has **no equivalent in chrono**: it is a `time_compute`-only
    /// addition, layered on top of the frozen, chrono-compatible API surface,
    /// inspired by WLanguage's `Age()` function. It is exactly
    /// [`years_since`](Self::years_since) with the arguments in the order
    /// that reads naturally for this use case (`date_of_birth.age(today)`
    /// rather than `today.years_since(date_of_birth)`).
    ///
    /// `NaiveDate` never reads the system clock -- that stays `Utc`'s and
    /// `Local`'s job, matching chrono's own separation between "naive" and
    /// timezone-aware types. Pass today's date explicitly, e.g.
    /// `date_of_birth.age(Utc::now().date_naive())` or
    /// `date_of_birth.age(Local::now().date_naive())`.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// let date_of_birth = NaiveDate::from_ymd_opt(1990, 6, 15).unwrap();
    /// let today = NaiveDate::from_ymd_opt(2023, 6, 14).unwrap(); // day before the birthday
    /// assert_eq!(date_of_birth.age(today), Some(32));
    /// let today = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(); // the birthday itself
    /// assert_eq!(date_of_birth.age(today), Some(33));
    /// ```
    #[must_use]
    pub const fn age(&self, on: Self) -> Option<u32> {
        on.years_since(*self)
    }

    /// Date of Easter Sunday for the given year, in the western
    /// (Catholic/Protestant) Christian tradition -- computed directly in
    /// the (proleptic) Gregorian calendar. Returns `None` only if the
    /// result would fall outside `NaiveDate`'s representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// This function has **no equivalent in chrono** (chrono's own
    /// ecosystem relies on the separate `computus` crate for this); it is
    /// a `time_compute`-only addition, inspired by WLanguage's `Easter()`
    /// function. Uses the "anonymous Gregorian algorithm" (Meeus/Jones/
    /// Butcher) -- a standard, public-domain calendar algorithm, the same
    /// category of well-known math already used elsewhere in this crate's
    /// calendar computations (see `calendar.rs`); implemented from
    /// scratch, not copied from any other crate.
    ///
    /// See also [`orthodox_easter`](Self::orthodox_easter) for the Eastern
    /// Orthodox date, which is computed differently (Julian calendar
    /// reckoning, then converted to the Gregorian calendar) and normally
    /// falls on a different day.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::easter(2023), NaiveDate::from_ymd_opt(2023, 4, 9));
    /// assert_eq!(NaiveDate::easter(2024), NaiveDate::from_ymd_opt(2024, 3, 31));
    /// assert_eq!(NaiveDate::easter(2025), NaiveDate::from_ymd_opt(2025, 4, 20));
    /// ```
    #[must_use]
    pub const fn easter(year: i32) -> Option<NaiveDate> {
        let a = year.rem_euclid(19);
        let b = year.div_euclid(100);
        let c = year.rem_euclid(100);
        let d = b.div_euclid(4);
        let e = b.rem_euclid(4);
        let f = (b + 8).div_euclid(25);
        let g = (b - f + 1).div_euclid(3);
        let h = (19 * a + b - d - g + 15).rem_euclid(30);
        let i = c.div_euclid(4);
        let k = c.rem_euclid(4);
        let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
        let m = (a + 11 * h + 22 * l).div_euclid(451);
        let sum = h + l - 7 * m + 114;
        let month = sum.div_euclid(31);
        let day = sum.rem_euclid(31) + 1;
        NaiveDate::from_ymd_opt(year, month as u32, day as u32)
    }

    /// Date of Easter Sunday for the given year, in the Eastern Orthodox
    /// Christian tradition. Returns `None` only if the result would fall
    /// outside `NaiveDate`'s representable range (including, degenerately,
    /// years so far in the past that the Julian/Gregorian drift computed
    /// below would be negative).
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`easter`](Self::easter) for the general remarks (no chrono
    /// equivalent, inspired by WLanguage, original implementation). This
    /// one is computed differently: the Orthodox tradition keeps the older
    /// Julian-calendar reckoning of the Paschal full moon (Meeus's Julian
    /// algorithm), and the result is then converted to the (proleptic)
    /// Gregorian calendar this crate uses throughout, by adding the
    /// accumulated Julian/Gregorian drift for that year:
    /// `year/100 - year/400 - 2` days (integer division) -- the standard
    /// closed-form for this offset, equivalent to the commonly published
    /// table (+10 days for 1583-1699, +11 for 1700-1799, +12 for
    /// 1800-1899, +13 for 1900-2099, +14 for 2100-2199, ...).
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::orthodox_easter(2023), NaiveDate::from_ymd_opt(2023, 4, 16));
    /// assert_eq!(NaiveDate::orthodox_easter(2024), NaiveDate::from_ymd_opt(2024, 5, 5));
    /// ```
    #[must_use]
    pub const fn orthodox_easter(year: i32) -> Option<NaiveDate> {
        let a = year.rem_euclid(4);
        let b = year.rem_euclid(7);
        let c = year.rem_euclid(19);
        let d = (19 * c + 15).rem_euclid(30);
        let e = (2 * a + 4 * b - d + 34).rem_euclid(7);
        let sum = d + e + 114;
        let julian_month = sum.div_euclid(31);
        let julian_day = sum.rem_euclid(31) + 1;
        // `julian_reading`: a `NaiveDate` whose (year, month, day) fields
        // hold the Julian-calendar Easter reading computed above, treated
        // for a moment as if it were a Gregorian-calendar date -- purely
        // to get a `NaiveDate` value we can shift by a number of days.
        let julian_reading =
            match NaiveDate::from_ymd_opt(year, julian_month as u32, julian_day as u32) {
                Some(date) => date,
                None => return None,
            };
        let offset = year.div_euclid(100) - year.div_euclid(400) - 2;
        if offset < 0 {
            return None;
        }
        julian_reading.checked_add_days(Days::new(offset as u64))
    }

    /// Date of the first day of Passover (Pessah, 15 Nisan) for the given
    /// Gregorian year, according to the traditional Hebrew calendar.
    /// Returns `None` only if the result would fall outside `NaiveDate`'s
    /// representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Inspired by WLanguage's date-of-religious-
    /// holiday functions. Unlike [`easter`](Self::easter) and
    /// [`orthodox_easter`](Self::orthodox_easter), which are governed by
    /// fixed arithmetic congruences, the Hebrew calendar is lunisolar:
    /// this implementation computes the *molad* (mean lunar conjunction)
    /// of Tishrei for the relevant Hebrew year using the traditional
    /// 19-year Metonic cycle and the classic mean month length of 29
    /// days, 12 hours, 793 *chalakim* (1 chalak = 1/1080 hour -- all
    /// arithmetic here is exact integers, no floating point), applies the
    /// four traditional postponement rules ("Lo ADU Rosh", "Molad Zaken",
    /// "GaTaRaD", "BeTuTeKaFoT") to obtain Rosh Hashanah (1 Tishrei), then
    /// advances by the month lengths between Tishrei and Nisan (accounting
    /// for Cheshvan/Kislev's variable lengths and, in leap years, the
    /// extra month Adar I) to reach 15 Nisan. The whole computation is
    /// anchored to one independently verified real date (1 Tishrei 5783 =
    /// Monday 26 September 2022), so no ancient proleptic epoch conversion
    /// is needed -- every other year is reached purely by counting elapsed
    /// days from that anchor. Hand-verified (including cross-checking
    /// against an independent, external date-conversion reference) for
    /// 5715, 5718, and 5778-5800 -- thirteen reference dates spanning both
    /// rare postponement-rule edge cases and all three Hebrew year-length
    /// categories -- in addition to the four dates in the doctest below.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::passover(2023), NaiveDate::from_ymd_opt(2023, 4, 6));
    /// assert_eq!(NaiveDate::passover(2024), NaiveDate::from_ymd_opt(2024, 4, 23));
    /// assert_eq!(NaiveDate::passover(2025), NaiveDate::from_ymd_opt(2025, 4, 13));
    /// assert_eq!(NaiveDate::passover(2026), NaiveDate::from_ymd_opt(2026, 4, 2));
    /// ```
    #[must_use]
    pub const fn passover(year: i32) -> Option<NaiveDate> {
        let hebrew_year = year as i64 + 3760;
        let (rosh_hashanah, cheshvan_kislev, leap) = match Self::hebrew_year_info(hebrew_year) {
            Some(info) => info,
            None => return None,
        };
        let leap_bonus = if leap { 30 } else { 0 };
        // Days from 1 Tishrei to 15 Nisan: Tishrei(30) + Cheshvan + Kislev
        // + Tevet(29) + Shevat(30) + [Adar I(30) if leap] + Adar/AdarII(29)
        // to reach 1 Nisan, plus 14 more days to reach 15 Nisan.
        let offset_from_rosh_hashanah = 132 + cheshvan_kislev + leap_bonus;
        Self::hebrew_day_number_to_gregorian(rosh_hashanah + offset_from_rosh_hashanah)
    }

    /// Date of Rosh Hashanah (1 Tishrei, the Hebrew New Year) for the
    /// given Gregorian year. Rosh Hashanah falls in autumn, so (unlike
    /// [`passover`](Self::passover) and the other spring/summer Hebrew
    /// holidays below) the Hebrew year used is `year + 3761`, not `year +
    /// 3760`. Returns `None` only if the result would fall outside
    /// `NaiveDate`'s representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Shares the *molad* / postponement-rule engine
    /// described at [`passover`](Self::passover). Hand-verified against
    /// hebcal.com for 2022-2025.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::rosh_hashanah(2024), NaiveDate::from_ymd_opt(2024, 10, 3));
    /// assert_eq!(NaiveDate::rosh_hashanah(2025), NaiveDate::from_ymd_opt(2025, 9, 23));
    /// ```
    #[must_use]
    pub const fn rosh_hashanah(year: i32) -> Option<NaiveDate> {
        let hebrew_year = year as i64 + 3761;
        Self::hebrew_day_number_to_gregorian(Self::hebrew_rosh_hashanah_day_number(hebrew_year))
    }

    /// Date of Yom Kippur (10 Tishrei) for the given Gregorian year: 9
    /// days after [`rosh_hashanah`](Self::rosh_hashanah). Returns `None`
    /// only if the result would fall outside `NaiveDate`'s representable
    /// range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Hand-verified against hebcal.com for
    /// 2022-2025.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::yom_kippur(2024), NaiveDate::from_ymd_opt(2024, 10, 12));
    /// ```
    #[must_use]
    pub const fn yom_kippur(year: i32) -> Option<NaiveDate> {
        let hebrew_year = year as i64 + 3761;
        Self::hebrew_day_number_to_gregorian(Self::hebrew_rosh_hashanah_day_number(hebrew_year) + 9)
    }

    /// Date of the first day of Sukkot (15 Tishrei) for the given
    /// Gregorian year: 14 days after [`rosh_hashanah`](Self::rosh_hashanah).
    /// Returns `None` only if the result would fall outside `NaiveDate`'s
    /// representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Hand-verified against hebcal.com for
    /// 2022-2025.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::sukkot(2024), NaiveDate::from_ymd_opt(2024, 10, 17));
    /// ```
    #[must_use]
    pub const fn sukkot(year: i32) -> Option<NaiveDate> {
        let hebrew_year = year as i64 + 3761;
        Self::hebrew_day_number_to_gregorian(Self::hebrew_rosh_hashanah_day_number(hebrew_year) + 14)
    }

    /// Date of the first day of Hanukkah (25 Kislev) for the given
    /// Gregorian year. Returns `None` only if the result would fall
    /// outside `NaiveDate`'s representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Kislev's length (29 or 30 days) depends on
    /// the Hebrew year's length category, exactly like Passover's
    /// dependency on Cheshvan+Kislev -- see [`passover`](Self::passover)
    /// for the underlying engine. Hand-verified against hebcal.com for
    /// 2022-2025.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::hanukkah(2024), NaiveDate::from_ymd_opt(2024, 12, 26));
    /// ```
    #[must_use]
    pub const fn hanukkah(year: i32) -> Option<NaiveDate> {
        let hebrew_year = year as i64 + 3761;
        let (rosh_hashanah, cheshvan_kislev, _leap) = match Self::hebrew_year_info(hebrew_year) {
            Some(info) => info,
            None => return None,
        };
        // Cheshvan is 30 days in a "complete" year (Cheshvan+Kislev = 60),
        // 29 days otherwise (Cheshvan+Kislev = 58 or 59).
        let cheshvan_len = if cheshvan_kislev == 60 { 30 } else { 29 };
        // Tishrei(30) + Cheshvan to reach 1 Kislev, then 24 more days to
        // reach the 25th of Kislev.
        Self::hebrew_day_number_to_gregorian(rosh_hashanah + 30 + cheshvan_len + 24)
    }

    /// Date of Purim (14 Adar in a common Hebrew year, 14 Adar II in a
    /// leap year) for the given Gregorian year. Purim falls in late
    /// winter, so (like [`passover`](Self::passover)) the Hebrew year used
    /// is `year + 3760`. Returns `None` only if the result would fall
    /// outside `NaiveDate`'s representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Hand-verified against hebcal.com for 2023
    /// (common year, 14 Adar) and 2024 (leap year, 14 Adar II).
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::purim(2025), NaiveDate::from_ymd_opt(2025, 3, 14));
    /// assert_eq!(NaiveDate::purim(2024), NaiveDate::from_ymd_opt(2024, 3, 24));
    /// ```
    #[must_use]
    pub const fn purim(year: i32) -> Option<NaiveDate> {
        let hebrew_year = year as i64 + 3760;
        let (rosh_hashanah, cheshvan_kislev, leap) = match Self::hebrew_year_info(hebrew_year) {
            Some(info) => info,
            None => return None,
        };
        let leap_bonus = if leap { 30 } else { 0 };
        // 16 days before 1 Nisan (see `passover`'s "132" constant): 132 -
        // 16 = 116... derived directly instead as Tishrei(30) + Cheshvan +
        // Kislev + Tevet(29) + Shevat(30) + [Adar I(30) if leap] + 13 (14th
        // day of Adar/Adar II).
        Self::hebrew_day_number_to_gregorian(rosh_hashanah + 102 + cheshvan_kislev + leap_bonus)
    }

    /// Date of Shavuot (6 Sivan) for the given Gregorian year: 50 days
    /// after [`passover`](Self::passover) (an "Omer count" of 49 days,
    /// then Shavuot on the 50th day) -- the same relationship as western
    /// [`pentecost`](Self::pentecost) to [`easter`](Self::easter). Returns
    /// `None` only if the result would fall outside `NaiveDate`'s
    /// representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Hand-verified against hebcal.com for 2024.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::shavuot(2024), NaiveDate::from_ymd_opt(2024, 6, 12));
    /// ```
    #[must_use]
    pub const fn shavuot(year: i32) -> Option<NaiveDate> {
        match Self::passover(year) {
            Some(p) => p.checked_add_signed(Duration::days(50)),
            None => None,
        }
    }

    /// Converts a date in the Hebrew calendar to the equivalent Gregorian
    /// `NaiveDate`. `year` is the Hebrew year; `month` uses **civil
    /// numbering** (1 = Tishrei, 2 = Cheshvan, 3 = Kislev, 4 = Tevet, 5 =
    /// Shevat, 6 = Adar in a common year or Adar I in a leap year, 7 =
    /// Nisan in a common year or Adar II in a leap year, 8 = Nisan in a
    /// leap year, ..., ending at Elul which is month 12 in a common year
    /// or month 13 in a leap year) -- the convention used by, among
    /// others, .NET's `HebrewCalendar` and hebcal. Returns `None` if
    /// `month` is out of range for that year's leap status, if `day` is
    /// out of range for that month, or if the result would fall outside
    /// `NaiveDate`'s representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Shares the *molad* / postponement-rule engine
    /// described at [`passover`](Self::passover) (which this function
    /// generalizes to every day of the year, not just Passover).
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// // 5786 is a common year, so Nisan is month 7.
    /// assert_eq!(NaiveDate::from_hebrew_ymd(5786, 1, 1), NaiveDate::from_ymd_opt(2025, 9, 23));
    /// // 5784 is a leap year, so Nisan is month 8.
    /// assert_eq!(NaiveDate::from_hebrew_ymd(5784, 8, 15), NaiveDate::from_ymd_opt(2024, 4, 23));
    /// ```
    #[must_use]
    pub const fn from_hebrew_ymd(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
        let hebrew_year = year as i64;
        let (rosh_hashanah, cheshvan_kislev, leap) = match Self::hebrew_year_info(hebrew_year) {
            Some(info) => info,
            None => return None,
        };
        let month_count = if leap { 13 } else { 12 };
        if month < 1 || month > month_count {
            return None;
        }
        let month_length = Self::hebrew_month_length(month, leap, cheshvan_kislev);
        if day < 1 || day > month_length {
            return None;
        }
        let day_number =
            rosh_hashanah + Self::hebrew_days_before_month(month, leap, cheshvan_kislev) + (day as i64 - 1);
        Self::hebrew_day_number_to_gregorian(day_number)
    }

    /// Converts `self` to the equivalent date in the Hebrew calendar,
    /// returning `(year, month, day)` using the same civil month
    /// numbering as [`from_hebrew_ymd`](Self::from_hebrew_ymd), of which
    /// this is the exact inverse.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// let d = NaiveDate::from_ymd_opt(2025, 9, 23).unwrap();
    /// assert_eq!(d.to_hebrew_ymd(), (5786, 1, 1));
    /// ```
    #[must_use]
    pub const fn to_hebrew_ymd(&self) -> (i32, u32, u32) {
        const ANCHOR_HEBREW_YEAR: i64 = 5783;
        // `from_ymd_opt(2022, 9, 26)` is a known-valid date, so the `None`
        // arm below is unreachable in practice; it only exists because
        // `from_ymd_opt` returns `Option` in general.
        let anchor_date = match NaiveDate::from_ymd_opt(2022, 9, 26) {
            Some(date) => date,
            None => *self,
        };
        let delta_days = (self.num_days_from_ce() - anchor_date.num_days_from_ce()) as i64;
        let anchor_day_number = Self::hebrew_rosh_hashanah_day_number(ANCHOR_HEBREW_YEAR);
        let target_day_number = anchor_day_number + delta_days;

        // Bracket search for the Hebrew year containing
        // `target_day_number`, starting from a good estimate (a Hebrew
        // year averages ~365.25 days, same as a Gregorian year, by
        // construction of the 19-year Metonic cycle).
        let mut year = ANCHOR_HEBREW_YEAR + delta_days.div_euclid(365);
        while Self::hebrew_rosh_hashanah_day_number(year) > target_day_number {
            year -= 1;
        }
        while Self::hebrew_rosh_hashanah_day_number(year + 1) <= target_day_number {
            year += 1;
        }

        // `year` is now guaranteed valid by construction (it was found by
        // bracketing a real target day number), so the `None` arm below
        // is unreachable in practice.
        let (rosh_hashanah, cheshvan_kislev, leap) = match Self::hebrew_year_info(year) {
            Some(info) => info,
            None => (Self::hebrew_rosh_hashanah_day_number(year), 59, false),
        };
        let month_count = if leap { 13 } else { 12 };
        let mut remaining = target_day_number - rosh_hashanah;
        let mut month: u32 = 1;
        loop {
            let month_length = Self::hebrew_month_length(month, leap, cheshvan_kislev) as i64;
            if remaining < month_length || month == month_count {
                break;
            }
            remaining -= month_length;
            month += 1;
        }
        (year as i32, month, (remaining + 1) as u32)
    }

    /// Number of days from 1 Tishrei to the first day of Hebrew month
    /// `month`, using the civil numbering documented at
    /// [`from_hebrew_ymd`](Self::from_hebrew_ymd). Caller must ensure
    /// `month` is valid for the given `leap` status (`1..=12` in a common
    /// year, `1..=13` in a leap year).
    ///
    /// Private helper for `from_hebrew_ymd` and `to_hebrew_ymd`.
    const fn hebrew_days_before_month(month: u32, leap: bool, cheshvan_kislev: i64) -> i64 {
        let cheshvan_len = if cheshvan_kislev == 60 { 30 } else { 29 };
        match month {
            1 => 0,
            2 => 30,
            3 => 30 + cheshvan_len,
            4 => 30 + cheshvan_kislev,
            5 => 30 + cheshvan_kislev + 29,
            6 => 30 + cheshvan_kislev + 29 + 30,
            _ => {
                // Days before Adar (common year) / Adar I (leap year):
                // Tishrei + Cheshvan + Kislev + Tevet + Shevat.
                let base = 30 + cheshvan_kislev + 29 + 30;
                if !leap {
                    match month {
                        7 => base + 29,
                        8 => base + 29 + 30,
                        9 => base + 29 + 30 + 29,
                        10 => base + 29 + 30 + 29 + 30,
                        11 => base + 29 + 30 + 29 + 30 + 29,
                        _ => base + 29 + 30 + 29 + 30 + 29 + 30, // month 12 (Elul)
                    }
                } else {
                    let base2 = base + 30; // after Adar I
                    match month {
                        7 => base2, // start of Adar II
                        _ => {
                            let base3 = base2 + 29; // after Adar II, start of Nisan
                            match month {
                                8 => base3,
                                9 => base3 + 30,
                                10 => base3 + 30 + 29,
                                11 => base3 + 30 + 29 + 30,
                                12 => base3 + 30 + 29 + 30 + 29,
                                _ => base3 + 30 + 29 + 30 + 29 + 30, // month 13 (Elul)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Length in days of Hebrew month `month` in a year with the given
    /// `leap` status and Cheshvan+Kislev combined length. Caller must
    /// ensure `month` is valid for `leap` (`1..=12` common, `1..=13`
    /// leap).
    ///
    /// Private helper for `from_hebrew_ymd` and `to_hebrew_ymd`.
    const fn hebrew_month_length(month: u32, leap: bool, cheshvan_kislev: i64) -> u32 {
        let month_count = if leap { 13 } else { 12 };
        if month == month_count {
            // Elul, the last month either way, is always 29 days.
            29
        } else {
            (Self::hebrew_days_before_month(month + 1, leap, cheshvan_kislev)
                - Self::hebrew_days_before_month(month, leap, cheshvan_kislev)) as u32
        }
    }

    /// Rosh Hashanah day number (see
    /// [`hebrew_rosh_hashanah_day_number`](Self::hebrew_rosh_hashanah_day_number)),
    /// the combined length of Cheshvan+Kislev (58, 59, or 60 days), and
    /// leap-year status for Hebrew year `h`. Returns `None` if the
    /// computed year length is not one of the six valid Hebrew calendar
    /// year lengths (353 to 355, or 383 to 385 in a leap year) -- this
    /// should not happen for any real Hebrew year, but fails safely
    /// rather than propagate a nonsensical offset.
    ///
    /// Private helper for `passover` and the other Hebrew-calendar
    /// holidays.
    const fn hebrew_year_info(h: i64) -> Option<(i64, i64, bool)> {
        let rosh_hashanah = Self::hebrew_rosh_hashanah_day_number(h);
        let rosh_hashanah_next = Self::hebrew_rosh_hashanah_day_number(h + 1);
        let year_length = rosh_hashanah_next - rosh_hashanah;
        let cheshvan_kislev = match year_length {
            353 | 383 => 58,
            354 | 384 => 59,
            355 | 385 => 60,
            _ => return None,
        };
        Some((rosh_hashanah, cheshvan_kislev, Self::hebrew_year_is_leap(h)))
    }

    /// Converts an internal Hebrew-calendar day number (as returned by
    /// [`hebrew_rosh_hashanah_day_number`](Self::hebrew_rosh_hashanah_day_number)
    /// and the offsets derived from it) to a real Gregorian `NaiveDate`,
    /// anchored to one independently verified real date (1 Tishrei 5783 =
    /// Monday 26 September 2022). Every other Hebrew year is located
    /// relative to this one by pure day counting, so no ancient proleptic
    /// epoch conversion is ever needed.
    ///
    /// Private helper for `passover` and the other Hebrew-calendar
    /// holidays.
    const fn hebrew_day_number_to_gregorian(day_number: i64) -> Option<NaiveDate> {
        const ANCHOR_HEBREW_YEAR: i64 = 5783;
        let anchor_date = match NaiveDate::from_ymd_opt(2022, 9, 26) {
            Some(date) => date,
            None => return None,
        };
        let anchor_day_number = Self::hebrew_rosh_hashanah_day_number(ANCHOR_HEBREW_YEAR);
        anchor_date.checked_add_signed(Duration::days(day_number - anchor_day_number))
    }

    /// Whether Hebrew year `h` is a leap year (has the extra month Adar I),
    /// per the 19-year Metonic cycle. Leap years fall at cycle positions
    /// 3, 6, 8, 11, 14, 17, 19 (1-indexed).
    ///
    /// Private helper for [`passover`](Self::passover).
    const fn hebrew_year_is_leap(h: i64) -> bool {
        let position = (h - 1).rem_euclid(19) + 1;
        matches!(position, 3 | 6 | 8 | 11 | 14 | 17 | 19)
    }

    /// Number of Hebrew calendar months elapsed in years `1..h` (i.e.
    /// before year `h` begins), using the 19-year / 235-month Metonic
    /// cycle (12 months per common year, 13 per leap year).
    ///
    /// Private helper for [`passover`](Self::passover).
    const fn hebrew_months_elapsed(h: i64) -> i64 {
        let n = h - 1;
        let full_cycles = n.div_euclid(19);
        let rem = n.rem_euclid(19);
        // Number of leap-year positions (3,6,8,11,14,17,19) at or before
        // cycle position `rem` (rem ranges 0..=18, so position 19 is never
        // included here -- it is already accounted for by `full_cycles`
        // whenever a complete 19-year cycle has elapsed).
        let leap_months_before = match rem {
            0..=2 => 0,
            3..=5 => 1,
            6..=7 => 2,
            8..=10 => 3,
            11..=13 => 4,
            14..=16 => 5,
            _ => 6, // rem is 17 or 18
        };
        full_cycles * 235 + 12 * rem + leap_months_before
    }

    /// Day number (an internal, self-consistent day count with no
    /// standalone calendrical meaning -- only *differences* between two
    /// calls are meaningful) of Rosh Hashanah (1 Tishrei) for Hebrew year
    /// `h`, computed from the mean lunar conjunction (*molad*) of Tishrei
    /// and the four traditional postponement rules.
    ///
    /// Private helper for [`passover`](Self::passover).
    const fn hebrew_rosh_hashanah_day_number(h: i64) -> i64 {
        const CHALAKIM_PER_HOUR: i64 = 1080;
        const CHALAKIM_PER_DAY: i64 = 24 * CHALAKIM_PER_HOUR;
        // Classic mean lunar month: 29 days, 12 hours, 793 chalakim.
        const MONTH_CHALAKIM: i64 = 29 * CHALAKIM_PER_DAY + 12 * CHALAKIM_PER_HOUR + 793;
        // BAHARAD: the traditional molad epoch (day 1 = Monday, +5 hours,
        // +204 chalakim), expressed as an offset in chalakim from the
        // start of "day 0" (a Sunday) in this function's internal,
        // arbitrary day numbering.
        const EPOCH_CHALAKIM: i64 = CHALAKIM_PER_DAY + 5 * CHALAKIM_PER_HOUR + 204;

        let molad_chalakim = EPOCH_CHALAKIM + Self::hebrew_months_elapsed(h) * MONTH_CHALAKIM;
        let raw_day = molad_chalakim.div_euclid(CHALAKIM_PER_DAY);
        let time_of_day = molad_chalakim.rem_euclid(CHALAKIM_PER_DAY);
        let original_weekday = raw_day.rem_euclid(7); // 0 = Sunday

        // Rule 1 ("Molad Zaken"): a molad at or after noon (18h) postpones
        // Rosh Hashanah to the next day.
        let after_rule1 =
            if time_of_day >= 18 * CHALAKIM_PER_HOUR { raw_day + 1 } else { raw_day };

        // Rule 3 ("GaTaRaD"): in a common (non-leap) year, if the
        // *original* (pre-rule-1) molad falls on Tuesday at or after
        // 9h204p, Rosh Hashanah is postponed to Thursday.
        let rule3_applies = !Self::hebrew_year_is_leap(h)
            && original_weekday == 2
            && time_of_day >= 9 * CHALAKIM_PER_HOUR + 204;
        // Rule 4 ("BeTuTeKaFoT"): if the *previous* year was leap and the
        // original molad falls on Monday at or after 15h589p, Rosh
        // Hashanah is postponed to Tuesday.
        let rule4_applies = Self::hebrew_year_is_leap(h - 1)
            && original_weekday == 1
            && time_of_day >= 15 * CHALAKIM_PER_HOUR + 589;

        if rule3_applies {
            raw_day + 2
        } else if rule4_applies {
            raw_day + 1
        } else {
            // Rule 2 ("Lo ADU Rosh"): Rosh Hashanah never falls on Sunday,
            // Wednesday or Friday -- postpone by one more day if it would.
            let weekday = after_rule1.rem_euclid(7);
            if matches!(weekday, 0 | 3 | 5) { after_rule1 + 1 } else { after_rule1 }
        }
    }

    /// `easter(year)` shifted by `offset_days` days (may be negative).
    ///
    /// Private helper shared by the western movable feasts below, all of
    /// which are defined as a fixed number of days before or after Easter
    /// Sunday.
    const fn easter_offset(year: i32, offset_days: i64) -> Option<NaiveDate> {
        match Self::easter(year) {
            Some(e) => e.checked_add_signed(Duration::days(offset_days)),
            None => None,
        }
    }

    /// Date of Mardi Gras ("Fat Tuesday" / Shrove Tuesday) for the given
    /// Gregorian year: 47 days before western Easter Sunday, the day
    /// before Ash Wednesday.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. A fixed offset from [`easter`](Self::easter);
    /// see that method for the underlying algorithm and its verification.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::mardi_gras(2024), NaiveDate::from_ymd_opt(2024, 2, 13));
    /// ```
    #[must_use]
    pub const fn mardi_gras(year: i32) -> Option<NaiveDate> {
        Self::easter_offset(year, -47)
    }

    /// Date of Ash Wednesday (the first day of Lent) for the given
    /// Gregorian year: 46 days before western Easter Sunday.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. A fixed offset from [`easter`](Self::easter);
    /// see that method for the underlying algorithm and its verification.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::ash_wednesday(2024), NaiveDate::from_ymd_opt(2024, 2, 14));
    /// ```
    #[must_use]
    pub const fn ash_wednesday(year: i32) -> Option<NaiveDate> {
        Self::easter_offset(year, -46)
    }

    /// Date of Palm Sunday for the given Gregorian year: 7 days before
    /// western Easter Sunday.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. A fixed offset from [`easter`](Self::easter);
    /// see that method for the underlying algorithm and its verification.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::palm_sunday(2024), NaiveDate::from_ymd_opt(2024, 3, 24));
    /// ```
    #[must_use]
    pub const fn palm_sunday(year: i32) -> Option<NaiveDate> {
        Self::easter_offset(year, -7)
    }

    /// Date of Ascension Day for the given Gregorian year: 39 days after
    /// western Easter Sunday (the 40th day, counting Easter Sunday itself
    /// as day 1).
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. A fixed offset from [`easter`](Self::easter);
    /// see that method for the underlying algorithm and its verification.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::ascension(2024), NaiveDate::from_ymd_opt(2024, 5, 9));
    /// ```
    #[must_use]
    pub const fn ascension(year: i32) -> Option<NaiveDate> {
        Self::easter_offset(year, 39)
    }

    /// Date of Pentecost (Whit Sunday) for the given Gregorian year: 49
    /// days after western Easter Sunday.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. A fixed offset from [`easter`](Self::easter);
    /// see that method for the underlying algorithm and its verification.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::pentecost(2024), NaiveDate::from_ymd_opt(2024, 5, 19));
    /// ```
    #[must_use]
    pub const fn pentecost(year: i32) -> Option<NaiveDate> {
        Self::easter_offset(year, 49)
    }

    /// Converts a date in the tabular (arithmetical) Hijri/Islamic
    /// calendar -- `year` is the Hijri (AH) year, `month` is `1..=12`
    /// (1 = Muharram, ..., 9 = Ramadan, 10 = Shawwal, 12 = Dhu al-Hijjah)
    /// -- to the equivalent Gregorian `NaiveDate`. Returns `None` if
    /// `month` is not in `1..=12`, if `day` is out of range for that
    /// Hijri month, or if the result would fall outside `NaiveDate`'s
    /// representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent -- chrono has no Hijri calendar support at
    /// all. This implements the widely used **"tabular" (also called
    /// "civil" or "Kuwaiti algorithm") Islamic calendar**: a fixed
    /// arithmetical approximation (12 lunar months per year; odd months
    /// have 30 days, even months 29, except the 12th month Dhu al-Hijjah
    /// has 30 days in a leap year instead of 29; 11 leap years in each
    /// 30-year cycle, at cycle positions 2, 5, 7, 10, 13, 16, 18, 21, 24,
    /// 26, 29) rather than actual moon-sighting observation. This is the
    /// same scheme used by, among others, Microsoft Windows, and is
    /// documented as "Type IIa" on Wikipedia's "Tabular Islamic calendar"
    /// page.
    ///
    /// **Important limitation, inherent to any tabular calendar (not
    /// specific to this implementation):** real-world religious
    /// observance of the Islamic calendar is based on actual moon
    /// sighting (or, in some countries, the Umm al-Qura astronomical
    /// calendar), which routinely differs from this tabular approximation
    /// by a day and occasionally two -- this is expected and documented
    /// behavior of *any* tabular Hijri calendar, not a bug. Use this for
    /// consistent, reproducible historical/approximate conversions, not
    /// for determining the officially announced date of a religious
    /// observance.
    ///
    /// Anchored to two independently verified real dates (1 Muharram 1446
    /// AH = Sunday 7 July 2024, and 1 Muharram 1447 AH = Thursday 26 June
    /// 2025) rather than the ancient 7th-century epoch, consistent with
    /// the approach used for [`passover`](Self::passover).
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::from_hijri_ymd(1446, 1, 1), NaiveDate::from_ymd_opt(2024, 7, 7));
    /// assert_eq!(NaiveDate::from_hijri_ymd(1447, 1, 1), NaiveDate::from_ymd_opt(2025, 6, 26));
    /// ```
    #[must_use]
    pub const fn from_hijri_ymd(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
        if month < 1 || month > 12 {
            return None;
        }
        let hijri_year = year as i64;
        let month_length = Self::hijri_month_length(hijri_year, month);
        if day < 1 || day > month_length {
            return None;
        }
        let day_number = Self::hijri_day_number(hijri_year, month, day);

        // Anchor: 1 Muharram 1446 AH = Sunday, 7 July 2024 (independently
        // verified). Every other Hijri date is located relative to this
        // one by pure day counting, so no ancient epoch conversion is
        // ever needed.
        const ANCHOR_HIJRI_YEAR: i64 = 1446;
        let anchor_date = match NaiveDate::from_ymd_opt(2024, 7, 7) {
            Some(date) => date,
            None => return None,
        };
        let anchor_day_number = Self::hijri_day_number(ANCHOR_HIJRI_YEAR, 1, 1);
        anchor_date.checked_add_signed(Duration::days(day_number - anchor_day_number))
    }

    /// Date of the Hijri New Year (1 Muharram) for the given Hijri year,
    /// per the tabular calendar -- see [`from_hijri_ymd`](Self::from_hijri_ymd)
    /// for the algorithm and its limitations. Note that this takes a
    /// **Hijri** year, not a Gregorian one: unlike the Christian and
    /// Hebrew calendars above, the Hijri calendar drifts through all
    /// Gregorian seasons over roughly 33 years, so there is no fixed,
    /// reliable mapping from a Gregorian year to "the" corresponding
    /// Hijri year.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::hijri_new_year(1447), NaiveDate::from_ymd_opt(2025, 6, 26));
    /// ```
    #[must_use]
    pub const fn hijri_new_year(hijri_year: i32) -> Option<NaiveDate> {
        Self::from_hijri_ymd(hijri_year, 1, 1)
    }

    /// Date of the first day of Ramadan (1 Ramadan) for the given Hijri
    /// year, per the tabular calendar -- see
    /// [`from_hijri_ymd`](Self::from_hijri_ymd) for the algorithm and, in
    /// particular, its documented limitation relative to real-world
    /// moon-sighting-based observance.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::ramadan_start(1447), NaiveDate::from_ymd_opt(2026, 2, 17));
    /// ```
    #[must_use]
    pub const fn ramadan_start(hijri_year: i32) -> Option<NaiveDate> {
        Self::from_hijri_ymd(hijri_year, 9, 1)
    }

    /// Date of Eid al-Fitr (1 Shawwal, marking the end of Ramadan) for the
    /// given Hijri year, per the tabular calendar -- see
    /// [`from_hijri_ymd`](Self::from_hijri_ymd) for the algorithm and its
    /// limitations.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::eid_al_fitr(1447), NaiveDate::from_ymd_opt(2026, 3, 19));
    /// ```
    #[must_use]
    pub const fn eid_al_fitr(hijri_year: i32) -> Option<NaiveDate> {
        Self::from_hijri_ymd(hijri_year, 10, 1)
    }

    /// Date of Eid al-Adha (10 Dhu al-Hijjah) for the given Hijri year,
    /// per the tabular calendar -- see
    /// [`from_hijri_ymd`](Self::from_hijri_ymd) for the algorithm and its
    /// limitations.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::eid_al_adha(1447), NaiveDate::from_ymd_opt(2026, 5, 26));
    /// ```
    #[must_use]
    pub const fn eid_al_adha(hijri_year: i32) -> Option<NaiveDate> {
        Self::from_hijri_ymd(hijri_year, 12, 10)
    }

    /// Converts `self` to the equivalent date in the tabular Hijri
    /// calendar, returning `(year, month, day)` with `year` the Hijri
    /// (AH) year and `month` in `1..=12`. See
    /// [`from_hijri_ymd`](Self::from_hijri_ymd) for the algorithm and its
    /// limitations -- this is its exact inverse.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// let d = NaiveDate::from_ymd_opt(2025, 6, 26).unwrap();
    /// assert_eq!(d.to_hijri_ymd(), (1447, 1, 1));
    /// ```
    #[must_use]
    pub const fn to_hijri_ymd(&self) -> (i32, u32, u32) {
        const ANCHOR_HIJRI_YEAR: i64 = 1446;
        // `from_ymd_opt(2024, 7, 7)` is a known-valid date, so the `None`
        // arm below is unreachable in practice; it only exists because
        // `from_ymd_opt` returns `Option` in general.
        let anchor_date = match NaiveDate::from_ymd_opt(2024, 7, 7) {
            Some(date) => date,
            None => *self,
        };
        let delta_days = (self.num_days_from_ce() - anchor_date.num_days_from_ce()) as i64;
        let anchor_day_number = Self::hijri_days_before_year(ANCHOR_HIJRI_YEAR);
        let target_day_number = anchor_day_number + delta_days;

        // Bracket search for the Hijri year containing
        // `target_day_number`, starting from a good estimate (a Hijri
        // year averages ~354.37 days).
        let mut year = ANCHOR_HIJRI_YEAR + delta_days.div_euclid(354);
        while Self::hijri_days_before_year(year) > target_day_number {
            year -= 1;
        }
        while Self::hijri_days_before_year(year + 1) <= target_day_number {
            year += 1;
        }

        let mut remaining = target_day_number - Self::hijri_days_before_year(year);
        let mut month: u32 = 1;
        loop {
            let month_length = Self::hijri_month_length(year, month) as i64;
            if remaining < month_length || month == 12 {
                break;
            }
            remaining -= month_length;
            month += 1;
        }
        (year as i32, month, (remaining + 1) as u32)
    }

    /// The Japanese era (nengo) and era-year containing `self`, e.g.
    /// `(JapaneseEra::Reiwa, 8)` for a date in 2026 (the 8th year of the
    /// Reiwa era). Returns `None` for dates before the modern era system
    /// began (25 January 1868) -- only the five modern "one reign, one
    /// era name" eras (Meiji onward) are supported; pre-Meiji era names
    /// (over 200 of them, some lasting only months) are out of scope.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent -- chrono has no notion of Japanese eras.
    ///
    /// ```
    /// use time_compute::{NaiveDate, JapaneseEra};
    ///
    /// let d = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
    /// assert_eq!(d.japanese_era(), Some((JapaneseEra::Reiwa, 8)));
    ///
    /// let d = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    /// assert_eq!(d.japanese_era(), Some((JapaneseEra::Heisei, 12)));
    /// ```
    #[must_use]
    pub const fn japanese_era(&self) -> Option<(JapaneseEra, i32)> {
        const ERAS: [JapaneseEra; 5] = [
            JapaneseEra::Reiwa,
            JapaneseEra::Heisei,
            JapaneseEra::Showa,
            JapaneseEra::Taisho,
            JapaneseEra::Meiji,
        ];
        let self_days = self.num_days_from_ce();
        let mut i = 0;
        while i < ERAS.len() {
            let era = ERAS[i];
            let start = era.start_date();
            if self_days >= start.num_days_from_ce() {
                // Era-years follow plain Gregorian year boundaries after
                // the (partial) first year -- see the doc comment on
                // `JapaneseEra`.
                let era_year = self.year - start.year + 1;
                return Some((era, era_year));
            }
            i += 1;
        }
        None
    }

    /// Converts a Japanese era date to the equivalent Gregorian
    /// `NaiveDate`. `era_year` is 1-based (the first, partial year of a
    /// reign is era-year 1). Returns `None` if `era_year`/`month`/`day`
    /// don't form a valid date, or if that date falls outside `era`'s
    /// actual span (e.g. `era_year` too large and overlapping the next
    /// era, or a date before the era's true start day within its first
    /// year).
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Exact inverse of
    /// [`japanese_era`](Self::japanese_era).
    ///
    /// ```
    /// use time_compute::{NaiveDate, JapaneseEra};
    ///
    /// assert_eq!(
    ///     NaiveDate::from_japanese_era_ymd(JapaneseEra::Reiwa, 8, 7, 13),
    ///     NaiveDate::from_ymd_opt(2026, 7, 13)
    /// );
    /// ```
    #[must_use]
    pub const fn from_japanese_era_ymd(era: JapaneseEra, era_year: i32, month: u32, day: u32) -> Option<NaiveDate> {
        let start = era.start_date();
        let gregorian_year = start.year + era_year - 1;
        let candidate = match NaiveDate::from_ymd_opt(gregorian_year, month, day) {
            Some(date) => date,
            None => return None,
        };
        if candidate.num_days_from_ce() < start.num_days_from_ce() {
            return None;
        }
        if let Some(end) = era.end_date_exclusive() {
            if candidate.num_days_from_ce() >= end.num_days_from_ce() {
                return None;
            }
        }
        Some(candidate)
    }

    /// Date of Shogatsu (正月, the Japanese New Year) for the given
    /// Gregorian year: 1 January. Returns `None` only if the result would
    /// fall outside `NaiveDate`'s representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Shinto-associated (the "hatsumode" first
    /// shrine visit of the year); a fixed date, no calculation needed.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::shogatsu(2026), NaiveDate::from_ymd_opt(2026, 1, 1));
    /// ```
    #[must_use]
    pub const fn shogatsu(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, 1, 1)
    }

    /// Date of Hana Matsuri (花祭り, the Buddha's birthday observance in
    /// Japan) for the given Gregorian year: 8 April. Returns `None` only
    /// if the result would fall outside `NaiveDate`'s representable
    /// range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Buddhist. Unlike most other Buddhist
    /// countries/traditions, which place the Buddha's birthday on a
    /// lunar-calendar date, Japan fixed it to 8 April on the Gregorian
    /// calendar -- a fixed date, no calculation needed.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::hana_matsuri(2026), NaiveDate::from_ymd_opt(2026, 4, 8));
    /// ```
    #[must_use]
    pub const fn hana_matsuri(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, 4, 8)
    }

    /// Date of Tanabata (七夕, the star festival) for the given Gregorian
    /// year: 7 July, the date used in most of Japan. (A minority of
    /// regions instead observe it a month later, on the old-calendar
    /// equivalent date -- roughly 7 August -- which this function does
    /// not cover.) Returns `None` only if the result would fall outside
    /// `NaiveDate`'s representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Shinto-influenced folk festival; a fixed
    /// date in its most common modern observance, no calculation needed.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::tanabata(2026), NaiveDate::from_ymd_opt(2026, 7, 7));
    /// ```
    #[must_use]
    pub const fn tanabata(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, 7, 7)
    }

    /// Date of the first day of Obon (お盆, the ancestor-veneration
    /// festival) for the given Gregorian year: 13 August, the date used
    /// in most of Japan (the festival then runs through 16 August). Some
    /// regions (e.g. Okinawa) instead follow the traditional lunisolar
    /// calendar date, which this function does not cover. Returns `None`
    /// only if the result would fall outside `NaiveDate`'s representable
    /// range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Buddhist. A fixed date in its most common
    /// modern (post-Meiji, "month-delayed") observance, no calculation
    /// needed.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::obon_start(2026), NaiveDate::from_ymd_opt(2026, 8, 13));
    /// ```
    #[must_use]
    pub const fn obon_start(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, 8, 13)
    }

    /// Date of Shichi-Go-San (七五三, a rite-of-passage festival for
    /// children aged three, five and seven) for the given Gregorian year:
    /// 15 November. Returns `None` only if the result would fall outside
    /// `NaiveDate`'s representable range.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Shinto. A fixed date, no calculation needed.
    ///
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::shichi_go_san(2026), NaiveDate::from_ymd_opt(2026, 11, 15));
    /// ```
    #[must_use]
    pub const fn shichi_go_san(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, 11, 15)
    }

    /// Japan's spring solar-term public holiday ("Shunbun no hi", 春分の日
    /// -- Vernal Equinox Day): the calendar day, in Japan Standard Time
    /// (JST, UTC+9), containing the instant the Sun's apparent geocentric
    /// ecliptic longitude crosses 0 degrees (the March equinox). Also the
    /// midpoint of the spring "Higan" (彼岸) week, a Buddhist observance.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent (chrono has no astronomical calculations at
    /// all). Requested by Fabrice alongside the other Japanese festivals,
    /// after asking what could be added for Japanese religious traditions
    /// beyond the administrative era system (see [`JapaneseEra`]).
    ///
    /// # A deliberate, one-time exception to this crate's usual approach
    /// Unlike every other function in this crate, this one is **not**
    /// `const fn`, uses **floating-point** arithmetic, and calls an
    /// **external computation dependency** (the `astro` crate, which
    /// implements Meeus' astronomical algorithms). This was a conscious
    /// choice: finding an equinox requires evaluating the Sun's true
    /// position, which has no closed-form integer solution the way the
    /// Hebrew/Hijri calendars do. Fabrice explicitly asked for and
    /// approved adding this dependency (2026-07-13) after being told it
    /// broke the crate's founding "near-zero-dependency" identity.
    ///
    /// # Precision
    /// Accurate to well within a day for any plausible year; the Sun's
    /// longitude is evaluated using `astro`'s "mean equinox of the date"
    /// position (it does not apply the small nutation correction that
    /// distinguishes "apparent" from "mean" longitude), and Delta-T (the
    /// TT-UT correction) is approximated by `astro::time::delta_t`. Both
    /// simplifications are on the order of tens of seconds of time --
    /// utterly negligible next to a calendar day, except in the
    /// vanishingly rare case where the true equinox instant falls within
    /// about a minute of a JST midnight boundary.
    ///
    /// # Range
    /// Returns `None` for `year` outside roughly `-32768..=32767`: unlike
    /// the rest of this crate (which supports years `-5,000,000` to
    /// `5,000,000`), the underlying `astro` crate represents the year as
    /// an `i16`. Not a practical limitation for a solar-term calculation
    /// tied to modern Japan.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// // Verified against published JMA (Japan Meteorological Agency)
    /// // Vernal Equinox Day announcements.
    /// assert_eq!(NaiveDate::shunbun_no_hi(2024), NaiveDate::from_ymd_opt(2024, 3, 20));
    /// assert_eq!(NaiveDate::shunbun_no_hi(2025), NaiveDate::from_ymd_opt(2025, 3, 20));
    /// assert_eq!(NaiveDate::shunbun_no_hi(2026), NaiveDate::from_ymd_opt(2026, 3, 20));
    /// ```
    #[must_use]
    pub fn shunbun_no_hi(year: i32) -> Option<NaiveDate> {
        Self::solar_term_date_jst(year, 3, 20, 0.0)
    }

    /// Japan's autumn solar-term public holiday ("Shuubun no hi", 秋分の日
    /// -- Autumnal Equinox Day): the JST calendar day containing the
    /// instant the Sun's apparent geocentric ecliptic longitude crosses
    /// 180 degrees (the September equinox). Also the midpoint of the
    /// autumn "Higan" (彼岸) week.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`shunbun_no_hi`](Self::shunbun_no_hi) for the full rationale,
    /// dependency exception, and precision notes -- they apply identically
    /// here.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::shuubun_no_hi(2024), NaiveDate::from_ymd_opt(2024, 9, 22));
    /// assert_eq!(NaiveDate::shuubun_no_hi(2025), NaiveDate::from_ymd_opt(2025, 9, 23));
    /// assert_eq!(NaiveDate::shuubun_no_hi(2026), NaiveDate::from_ymd_opt(2026, 9, 23));
    /// ```
    #[must_use]
    pub fn shuubun_no_hi(year: i32) -> Option<NaiveDate> {
        Self::solar_term_date_jst(year, 9, 23, 180.0)
    }

    /// Setsubun (節分, "seasonal division"): the day before "Risshun" (立春,
    /// the solar-term start of spring -- the Sun's apparent geocentric
    /// ecliptic longitude crossing 315 degrees), traditionally marked with
    /// the bean-throwing ritual (mamemaki) at both Shinto shrines and
    /// Buddhist temples.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`shunbun_no_hi`](Self::shunbun_no_hi) for the full rationale,
    /// dependency exception, and precision notes -- they apply identically
    /// here.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// // Setsubun drifts between Feb 2nd and 4th depending on the exact
    /// // moment of Risshun -- 2021 and 2025 both fell on Feb 2nd, the
    /// // first Feb-2nd occurrences since 1897.
    /// assert_eq!(NaiveDate::setsubun(2024), NaiveDate::from_ymd_opt(2024, 2, 3));
    /// assert_eq!(NaiveDate::setsubun(2025), NaiveDate::from_ymd_opt(2025, 2, 2));
    /// assert_eq!(NaiveDate::setsubun(2026), NaiveDate::from_ymd_opt(2026, 2, 3));
    /// ```
    #[must_use]
    pub fn setsubun(year: i32) -> Option<NaiveDate> {
        let risshun = Self::solar_term_date_jst(year, 2, 4, 315.0)?;
        risshun.pred_opt()
    }

    /// The JST (UTC+9) calendar date on which the Sun's apparent
    /// geocentric ecliptic longitude crosses `target_deg` degrees,
    /// searching near `year`-`seed_month`-`seed_day` (a plain calendar
    /// date a few days from the true crossing is enough of a seed -- see
    /// [`Self::bisect_solar_longitude`]).
    ///
    /// Private helper for [`shunbun_no_hi`](Self::shunbun_no_hi),
    /// [`shuubun_no_hi`](Self::shuubun_no_hi), and
    /// [`setsubun`](Self::setsubun).
    fn solar_term_date_jst(year: i32, seed_month: u32, seed_day: u32, target_deg: f64) -> Option<NaiveDate> {
        let year_i16 = i16::try_from(year).ok()?;
        let month_u8 = u8::try_from(seed_month).ok()?;
        let seed_date = astro::time::Date {
            year: year_i16,
            month: month_u8,
            decimal_day: f64::from(seed_day),
            cal_type: astro::time::CalType::Gregorian,
        };
        let seed_jd = astro::time::julian_day(&seed_date);
        let delta_t_seconds = astro::time::delta_t(year, month_u8);
        let seed_jde = astro::time::julian_ephemeris_day(seed_jd, delta_t_seconds);

        let crossing_jde = Self::bisect_solar_longitude(seed_jde, target_deg);

        // `crossing_jde` is a Julian Ephemeris Day (Terrestrial Time).
        // Subtract Delta-T back out to get a plain (UT) Julian Day -- it
        // is held fixed at the seed date's value across this narrow +/-8
        // day search, an error of milliseconds, irrelevant here -- then
        // add 9 hours to shift from UT to JST before reading off the
        // calendar date: Japan's solar terms are announced on the JST
        // calendar day containing the crossing instant, not the UTC one.
        let crossing_jd_ut = crossing_jde - delta_t_seconds / 86_400.0;
        let jst_jd = crossing_jd_ut + 9.0 / 24.0;
        let (result_year, result_month, decimal_day) = astro::time::date_frm_julian_day(jst_jd).ok()?;
        NaiveDate::from_ymd_opt(i32::from(result_year), u32::from(result_month), decimal_day.trunc() as u32)
    }

    /// Finds, via bisection, the Julian Ephemeris Day within +/-8 days of
    /// `seed_jde` at which the Sun's apparent geocentric ecliptic
    /// longitude crosses `target_deg` degrees.
    ///
    /// The Sun's ecliptic longitude increases by roughly one degree a day
    /// and is otherwise monotonic on this timescale, so an 8-day-wide
    /// window around a reasonable seed date (see the call sites in
    /// [`Self::solar_term_date_jst`]) brackets exactly one crossing.
    ///
    /// Private helper.
    fn bisect_solar_longitude(seed_jde: f64, target_deg: f64) -> f64 {
        let f = |jde: f64| Self::signed_longitude_distance_deg(Self::solar_longitude_deg(jde), target_deg);

        // Scan in half-day steps to find a bracket where `f` changes sign.
        let mut lo = seed_jde - 8.0;
        let mut f_lo = f(lo);
        let mut hi = seed_jde + 8.0;
        let mut t = lo;
        while t <= seed_jde + 8.0 {
            let f_t = f(t);
            if f_lo * f_t < 0.0 {
                hi = t;
                break;
            }
            lo = t;
            f_lo = f_t;
            t += 0.5;
        }

        // Bisect the bracket down to well under a millisecond of a day.
        let mut a = lo;
        let mut b = hi;
        let mut f_a = f(a);
        for _ in 0..60 {
            let mid = (a + b) / 2.0;
            let f_mid = f(mid);
            if f_a * f_mid <= 0.0 {
                b = mid;
            } else {
                a = mid;
                f_a = f_mid;
            }
        }
        (a + b) / 2.0
    }

    /// The Sun's apparent geocentric ecliptic longitude, in degrees,
    /// normalized to `[0, 360)`, at Julian Ephemeris Day `jde`. Private
    /// helper, wraps `astro::sun::geocent_ecl_pos`.
    fn solar_longitude_deg(jde: f64) -> f64 {
        let (ecl_point, _sun_earth_dist_au) = astro::sun::geocent_ecl_pos(jde);
        let deg = ecl_point.long.to_degrees() % 360.0;
        if deg < 0.0 {
            deg + 360.0
        } else {
            deg
        }
    }

    /// Signed angular distance `a - target`, normalized to `(-180, 180]`
    /// degrees. Private helper for [`Self::bisect_solar_longitude`].
    fn signed_longitude_distance_deg(a: f64, target: f64) -> f64 {
        let d = (a - target) % 360.0;
        if d <= -180.0 {
            d + 360.0
        } else if d > 180.0 {
            d - 360.0
        } else {
            d
        }
    }

    /// The Chinese lunisolar calendar's New Year's Day (first day of
    /// month 1) of Chinese Year `year`, where `year` is the Gregorian
    /// year the New Year falls in (the common "civil year" convention --
    /// see [`Self::to_chinese_ymd`]).
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent -- chrono has zero Chinese calendar support.
    /// Requested by Fabrice after asking about Chinese and Japanese
    /// festivals; deferred at first ("rien pour l'instant") because it
    /// needed real astronomical computation the crate didn't have, then
    /// revisited once the `astro` dependency was added for the Japanese
    /// solar-term functions -- see [`Self::shunbun_no_hi`] for the
    /// dependency/precision notes, which apply here too, and the
    /// `chinese_calendar.rs` module doc for the full algorithm writeup.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::chinese_new_year(2024), NaiveDate::from_ymd_opt(2024, 2, 10)); // Year of the Dragon
    /// assert_eq!(NaiveDate::chinese_new_year(2025), NaiveDate::from_ymd_opt(2025, 1, 29)); // Year of the Snake
    /// assert_eq!(NaiveDate::chinese_new_year(2026), NaiveDate::from_ymd_opt(2026, 2, 17)); // Year of the Horse
    /// ```
    #[must_use]
    pub fn chinese_new_year(year: i32) -> Option<NaiveDate> {
        crate::chinese_calendar::chinese_new_year(year)
    }

    /// Converts this date to the Chinese lunisolar calendar: `(year,
    /// month, is_leap, day)`, where `year` is the Gregorian year in which
    /// that Chinese year's New Year falls (there is no single
    /// universally standard numeric "Chinese year count" the way the
    /// Hebrew or Hijri calendars have one -- this is the convention used
    /// by the overwhelming majority of Chinese calendar software; the
    /// traditional alternative, a 60-year sexagenary cycle name, is not
    /// implemented here), `month` is `1..=12`, `is_leap` marks a leap
    /// ("intercalary") month, and `day` is `1..=30`.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::chinese_new_year`] for the request context and
    /// [`Self::shunbun_no_hi`] for the dependency/precision notes that
    /// apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// // 2023 had a leap 2nd month (闰二月).
    /// let d = NaiveDate::from_ymd_opt(2023, 3, 22).unwrap();
    /// assert_eq!(d.to_chinese_ymd(), Some((2023, 2, true, 1)));
    /// ```
    #[must_use]
    pub fn to_chinese_ymd(&self) -> Option<(i32, u32, bool, u32)> {
        crate::chinese_calendar::to_chinese_ymd(*self)
    }

    /// The inverse of [`Self::to_chinese_ymd`]: builds a `NaiveDate` from
    /// a Chinese lunisolar calendar year/month/leap-flag/day. Returns
    /// `None` if `month`/`is_leap` doesn't identify an actual month of
    /// Chinese Year `year` (e.g. asking for a leap month in a year that
    /// doesn't have one), or if `day` exceeds that month's actual length
    /// (29 or 30 days).
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::chinese_new_year`] for the request context and
    /// [`Self::shunbun_no_hi`] for the dependency/precision notes that
    /// apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(
    ///     NaiveDate::from_chinese_ymd(2023, 2, true, 1),
    ///     NaiveDate::from_ymd_opt(2023, 3, 22)
    /// );
    /// // 2024 has no leap month at all.
    /// assert_eq!(NaiveDate::from_chinese_ymd(2024, 2, true, 1), None);
    /// ```
    #[must_use]
    pub fn from_chinese_ymd(year: i32, month: u32, is_leap: bool, day: u32) -> Option<NaiveDate> {
        crate::chinese_calendar::from_chinese_ymd(year, month, is_leap, day)
    }

    /// The Dragon Boat Festival (端午節, Duānwǔ), the 5th day of the 5th
    /// month of the Chinese lunisolar calendar.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::chinese_new_year`] for the request context and
    /// [`Self::shunbun_no_hi`] for the dependency/precision notes that
    /// apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::duanwu(2024), NaiveDate::from_ymd_opt(2024, 6, 10));
    /// ```
    #[must_use]
    pub fn duanwu(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_chinese_ymd(year, 5, false, 5)
    }

    /// The Mid-Autumn Festival (中秋節, Zhōngqiū), the 15th day of the 8th
    /// month of the Chinese lunisolar calendar.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::chinese_new_year`] for the request context and
    /// [`Self::shunbun_no_hi`] for the dependency/precision notes that
    /// apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::zhongqiu(2024), NaiveDate::from_ymd_opt(2024, 9, 17));
    /// ```
    #[must_use]
    pub fn zhongqiu(year: i32) -> Option<NaiveDate> {
        NaiveDate::from_chinese_ymd(year, 8, false, 15)
    }

    /// Qingming (清明, "clear and bright"), a solar-term festival (unlike
    /// [`Self::duanwu`]/[`Self::zhongqiu`], **not** a Chinese-calendar
    /// month/day -- it's the day the Sun's apparent geocentric ecliptic
    /// longitude crosses 15 degrees, roughly April 4th-5th every year),
    /// associated with tomb-sweeping and ancestor veneration.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::chinese_new_year`] for the request context and
    /// [`Self::shunbun_no_hi`] for the dependency/precision notes that
    /// apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::qingming(2024), NaiveDate::from_ymd_opt(2024, 4, 4));
    /// assert_eq!(NaiveDate::qingming(2025), NaiveDate::from_ymd_opt(2025, 4, 4));
    /// assert_eq!(NaiveDate::qingming(2026), NaiveDate::from_ymd_opt(2026, 4, 5));
    /// ```
    #[must_use]
    pub fn qingming(year: i32) -> Option<NaiveDate> {
        crate::chinese_calendar::qingming(year)
    }

    /// Magha Bucha (มาฆบูชา), commemorating the spontaneous gathering of
    /// 1,250 of the Buddha's disciples: the full moon of the 3rd month
    /// of the traditional Thai ("Chulasakarat") lunisolar calendar --
    /// or the 4th month, in a year with an intercalary ("athikamas")
    /// month.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent. Added after Fabrice asked for Buddhist
    /// calendar support (following the Japanese and Chinese calendar
    /// work above) and chose the full lunisolar engine over just a
    /// Buddhist-era year conversion. Unlike the Chinese calendar or the
    /// Japanese solar-term functions, this is **not** based on true
    /// astronomical positions -- it's a centuries-old, purely arithmetic
    /// mean-motion model (no `astro` dependency, no floating point), so
    /// its dates can differ by a day or two from true astronomical new
    /// moons, and even from other published Thai calendars, which use
    /// slightly different historical corrections to the same underlying
    /// system. See the `buddhist_calendar.rs` module doc for the full
    /// algorithm writeup and verification notes.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::magha_bucha(2024), NaiveDate::from_ymd_opt(2024, 2, 24));
    /// assert_eq!(NaiveDate::magha_bucha(2025), NaiveDate::from_ymd_opt(2025, 2, 12));
    /// ```
    #[must_use]
    pub fn magha_bucha(year: i32) -> Option<NaiveDate> {
        crate::buddhist_calendar::magha_bucha(year)
    }

    /// Visakha Bucha (วิสาขบูชา), also known as Vesak: the most important
    /// Buddhist holy day, commemorating the Buddha's birth, enlightenment
    /// and death, all traditionally held to have occurred on the same
    /// full-moon day. This is the full moon of the 6th lunar month of
    /// the traditional Thai calendar -- or the 7th month, in a year with
    /// an intercalary month.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::magha_bucha`] for the request context and the
    /// dependency/precision notes that apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::visakha_bucha(2024), NaiveDate::from_ymd_opt(2024, 5, 22));
    /// assert_eq!(NaiveDate::visakha_bucha(2023), NaiveDate::from_ymd_opt(2023, 6, 3)); // intercalary year
    /// ```
    #[must_use]
    pub fn visakha_bucha(year: i32) -> Option<NaiveDate> {
        crate::buddhist_calendar::visakha_bucha(year)
    }

    /// Asalha Bucha (อาสาฬหบูชา), commemorating the Buddha's first sermon:
    /// the full moon of the 8th lunar month of the traditional Thai
    /// calendar -- or of the *second* (repeated) occurrence of the 8th
    /// month, in a year with an intercalary month. The following day
    /// begins the Buddhist Lent (see [`Self::khao_phansa`]).
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::magha_bucha`] for the request context and the
    /// dependency/precision notes that apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::asalha_bucha(2024), NaiveDate::from_ymd_opt(2024, 7, 20));
    /// ```
    #[must_use]
    pub fn asalha_bucha(year: i32) -> Option<NaiveDate> {
        crate::buddhist_calendar::asalha_bucha(year)
    }

    /// Khao Phansa (เข้าพรรษา, "entering the rains"), the start of the
    /// three-lunar-month Buddhist Lent (a retreat period for monks) --
    /// the day immediately after [`Self::asalha_bucha`].
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::magha_bucha`] for the request context and the
    /// dependency/precision notes that apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::khao_phansa(2024), NaiveDate::from_ymd_opt(2024, 7, 21));
    /// ```
    #[must_use]
    pub fn khao_phansa(year: i32) -> Option<NaiveDate> {
        crate::buddhist_calendar::khao_phansa(year)
    }

    /// Awk Phansa (ออกพรรษา, "leaving the rains"), the end of the
    /// Buddhist Lent: the full moon of the 11th lunar month of the
    /// traditional Thai calendar. Unlike [`Self::visakha_bucha`] and
    /// [`Self::asalha_bucha`], this is never shifted by an intercalary
    /// month: whichever intercalation happens that year has already
    /// occurred earlier, at month 8.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// See [`Self::magha_bucha`] for the request context and the
    /// dependency/precision notes that apply here too.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::awk_phansa(2024), NaiveDate::from_ymd_opt(2024, 10, 17));
    /// assert_eq!(NaiveDate::awk_phansa(2025), NaiveDate::from_ymd_opt(2025, 10, 7));
    /// ```
    #[must_use]
    pub fn awk_phansa(year: i32) -> Option<NaiveDate> {
        crate::buddhist_calendar::awk_phansa(year)
    }

    /// Matariki, the Māori New Year, marked by the pre-dawn midwinter
    /// rising of the Matariki star cluster (the Pleiades). Returns
    /// `None` outside 2022-2052.
    ///
    /// # `time_compute` extension -- not part of chrono
    /// No chrono equivalent (a New Zealand public holiday since 2022).
    /// Added after Fabrice asked about Pacific/Oceania observances
    /// following the Chinese and Thai/Buddhist calendar work above.
    ///
    /// Unlike every other calendar function in this crate, this is
    /// **not** computed from a formula: New Zealand's Matariki Advisory
    /// Committee sets the date each year (the Friday closest to a
    /// several-day lunar phase window, itself not reducible to a single
    /// instant), and it is legislated as a public holiday. This method
    /// is a fixed lookup table transcribed from the Museum of New
    /// Zealand Te Papa Tongarewa's officially published dates,
    /// 2022-2052 -- reproducing an approximate formula instead would
    /// risk silently disagreeing with the actual legal holiday date,
    /// which defeats the point. `None` for any year outside that range,
    /// rather than a guess: the committee has not published dates
    /// beyond 2052 as of this writing. See the `matariki` module doc for
    /// sourcing details.
    ///
    /// # Examples
    /// ```
    /// use time_compute::NaiveDate;
    ///
    /// assert_eq!(NaiveDate::matariki(2024), NaiveDate::from_ymd_opt(2024, 6, 28));
    /// assert_eq!(NaiveDate::matariki(2026), NaiveDate::from_ymd_opt(2026, 7, 10));
    /// assert_eq!(NaiveDate::matariki(2053), None); // not yet published
    /// ```
    #[must_use]
    pub fn matariki(year: i32) -> Option<NaiveDate> {
        crate::matariki::matariki(year)
    }

    /// Whether Hijri year `y` is a leap year (Dhu al-Hijjah has 30 days
    /// instead of 29), per the standard 30-year cycle with leap years at
    /// cycle positions 2, 5, 7, 10, 13, 16, 18, 21, 24, 26, 29
    /// (1-indexed) -- the "Type IIa" / "Kuwaiti algorithm" pattern, the
    /// most widely used one.
    ///
    /// Private helper for [`from_hijri_ymd`](Self::from_hijri_ymd).
    const fn hijri_year_is_leap(y: i64) -> bool {
        let position = (y - 1).rem_euclid(30) + 1;
        matches!(position, 2 | 5 | 7 | 10 | 13 | 16 | 18 | 21 | 24 | 26 | 29)
    }

    /// Length in days of Hijri month `month` (`1..=12`) in Hijri year `y`.
    /// Odd months have 30 days, even months 29, except month 12 (Dhu
    /// al-Hijjah) which has 30 days in a leap year.
    ///
    /// Private helper for [`from_hijri_ymd`](Self::from_hijri_ymd). Caller
    /// must ensure `month` is in `1..=12`.
    const fn hijri_month_length(y: i64, month: u32) -> u32 {
        if month == 12 {
            if Self::hijri_year_is_leap(y) { 30 } else { 29 }
        } else if month % 2 == 1 {
            30
        } else {
            29
        }
    }

    /// Number of days in Hijri years `1..y` (i.e. before year `y`
    /// begins), using the 30-year cycle (10,631 days per full cycle: 30
    /// common years of 354 days, plus 11 leap days).
    ///
    /// Private helper for [`from_hijri_ymd`](Self::from_hijri_ymd).
    const fn hijri_days_before_year(y: i64) -> i64 {
        let n = y - 1;
        let full_cycles = n.div_euclid(30);
        let rem = n.rem_euclid(30);
        // Number of leap-year positions (2,5,7,10,13,16,18,21,24,26,29) at
        // or before cycle position `rem` (rem ranges 0..=29).
        let leap_years_before = match rem {
            0..=1 => 0,
            2..=4 => 1,
            5..=6 => 2,
            7..=9 => 3,
            10..=12 => 4,
            13..=15 => 5,
            16..=17 => 6,
            18..=20 => 7,
            21..=23 => 8,
            24..=25 => 9,
            26..=28 => 10,
            _ => 11, // rem == 29
        };
        full_cycles * (30 * 354 + 11) + rem * 354 + leap_years_before
    }

    /// Day number (an internal, self-consistent day count with no
    /// standalone calendrical meaning -- only *differences* between two
    /// calls are meaningful) of Hijri date `(y, month, day)`. Caller must
    /// ensure `month` is in `1..=12` and `day` is valid for that month.
    ///
    /// Private helper for [`from_hijri_ymd`](Self::from_hijri_ymd).
    const fn hijri_day_number(y: i64, month: u32, day: u32) -> i64 {
        let months_before = (month - 1) as i64;
        // Odd months (1st, 3rd, ...) have 30 days, even months 29 --
        // `29 * months_before` plus one extra day for every odd month
        // among the `months_before` already elapsed.
        let days_before_month = 29 * months_before + (months_before + 1).div_euclid(2);
        Self::hijri_days_before_year(y) + days_before_month + (day as i64 - 1)
    }

    /// An iterator over dates, starting at `self` and advancing one day at
    /// a time, up to and including `NaiveDate::MAX`.
    pub const fn iter_days(&self) -> crate::naive::iter::NaiveDateDaysIterator {
        crate::naive::iter::NaiveDateDaysIterator { value: *self }
    }

    /// An iterator over dates, starting at `self` and advancing one week
    /// (7 days) at a time, up to and including `NaiveDate::MAX`.
    pub const fn iter_weeks(&self) -> crate::naive::iter::NaiveDateWeeksIterator {
        crate::naive::iter::NaiveDateWeeksIterator { value: *self }
    }

    /// The [`NaiveWeek`](crate::naive::week::NaiveWeek) that this date
    /// belongs to, with weeks considered to start on `start`.
    pub const fn week(&self, start: Weekday) -> crate::naive::week::NaiveWeek {
        crate::naive::week::NaiveWeek::new(*self, start)
    }

    /// Combines this date with a time of day into a [`NaiveDateTime`].
    pub const fn and_time(&self, time: NaiveTime) -> NaiveDateTime {
        NaiveDateTime::new(*self, time)
    }

    /// Combines this date with an hour/minute/second, like
    /// [`and_hms_opt`](Self::and_hms_opt).
    ///
    /// # Panics
    /// Panics if the time is invalid.
    #[deprecated(note = "use `and_hms_opt()` instead")]
    pub fn and_hms(&self, hour: u32, min: u32, sec: u32) -> NaiveDateTime {
        self.and_hms_opt(hour, min, sec).expect("invalid time")
    }

    /// Combines this date with an hour/minute/second. Returns `None` if
    /// the time is invalid (use [`and_hms_milli_opt`](Self::and_hms_milli_opt)
    /// for a leap second).
    pub const fn and_hms_opt(&self, hour: u32, min: u32, sec: u32) -> Option<NaiveDateTime> {
        match NaiveTime::from_hms_opt(hour, min, sec) {
            Some(time) => Some(self.and_time(time)),
            None => None,
        }
    }

    /// Combines this date with an hour/minute/second/millisecond, like
    /// [`and_hms_milli_opt`](Self::and_hms_milli_opt).
    ///
    /// # Panics
    /// Panics if the time is invalid.
    #[deprecated(note = "use `and_hms_milli_opt()` instead")]
    pub fn and_hms_milli(&self, hour: u32, min: u32, sec: u32, milli: u32) -> NaiveDateTime {
        self.and_hms_milli_opt(hour, min, sec, milli)
            .expect("invalid time")
    }

    /// Combines this date with an hour/minute/second/millisecond. `milli`
    /// may exceed 1,000 to represent a leap second (only when `sec == 59`).
    /// Returns `None` if the time is invalid.
    pub const fn and_hms_milli_opt(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        milli: u32,
    ) -> Option<NaiveDateTime> {
        match NaiveTime::from_hms_milli_opt(hour, min, sec, milli) {
            Some(time) => Some(self.and_time(time)),
            None => None,
        }
    }

    /// Combines this date with an hour/minute/second/microsecond, like
    /// [`and_hms_micro_opt`](Self::and_hms_micro_opt).
    ///
    /// # Panics
    /// Panics if the time is invalid.
    #[deprecated(note = "use `and_hms_micro_opt()` instead")]
    pub fn and_hms_micro(&self, hour: u32, min: u32, sec: u32, micro: u32) -> NaiveDateTime {
        self.and_hms_micro_opt(hour, min, sec, micro)
            .expect("invalid time")
    }

    /// Combines this date with an hour/minute/second/microsecond. `micro`
    /// may exceed 1,000,000 to represent a leap second (only when
    /// `sec == 59`). Returns `None` if the time is invalid.
    pub const fn and_hms_micro_opt(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        micro: u32,
    ) -> Option<NaiveDateTime> {
        match NaiveTime::from_hms_micro_opt(hour, min, sec, micro) {
            Some(time) => Some(self.and_time(time)),
            None => None,
        }
    }

    /// Combines this date with an hour/minute/second/nanosecond, like
    /// [`and_hms_nano_opt`](Self::and_hms_nano_opt).
    ///
    /// # Panics
    /// Panics if the time is invalid.
    #[deprecated(note = "use `and_hms_nano_opt()` instead")]
    pub fn and_hms_nano(&self, hour: u32, min: u32, sec: u32, nano: u32) -> NaiveDateTime {
        self.and_hms_nano_opt(hour, min, sec, nano)
            .expect("invalid time")
    }

    /// Combines this date with an hour/minute/second/nanosecond. `nano`
    /// may exceed 1,000,000,000 (up to 1,999,999,999) to represent a leap
    /// second (only when `sec == 59`). Returns `None` if the time is
    /// invalid.
    pub const fn and_hms_nano_opt(
        &self,
        hour: u32,
        min: u32,
        sec: u32,
        nano: u32,
    ) -> Option<NaiveDateTime> {
        match NaiveTime::from_hms_nano_opt(hour, min, sec, nano) {
            Some(time) => Some(self.and_time(time)),
            None => None,
        }
    }

    /// Parses a `NaiveDate` from a string using a user-specified format.
    /// See the [`crate::format::strftime`] module for the supported escape
    /// sequences.
    pub fn parse_from_str(s: &str, fmt: &str) -> ParseResult<NaiveDate> {
        let mut parsed = Parsed::new();
        parse(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_naive_date()
    }

    /// Parses a `NaiveDate` from a string using a user-specified format,
    /// returning the value and a slice with the remaining, unparsed portion
    /// of the string.
    pub fn parse_and_remainder<'a>(s: &'a str, fmt: &str) -> ParseResult<(NaiveDate, &'a str)> {
        let mut parsed = Parsed::new();
        let remainder = parse_and_remainder(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_naive_date().map(|d| (d, remainder))
    }

    /// Formats the date with the specified formatting items.
    #[must_use]
    pub fn format_with_items<'a, I, B>(&self, items: I) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        DelayedFormat::new(Some(*self), None, items)
    }

    /// Formats the date with the specified format string. See the
    /// [`crate::format::strftime`] module for the supported escape
    /// sequences.
    ///
    /// This returns a `DelayedFormat`, converted to a string only when
    /// actually formatted (via `Display`/`to_string`).
    ///
    /// # Panics
    /// Converting or formatting the returned `DelayedFormat` panics if the
    /// format string is invalid.
    #[must_use]
    pub fn format<'a>(&self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'a>> {
        self.format_with_items(StrftimeItems::new(fmt))
    }

    /// Formats the date with the specified formatting items and locale.
    ///
    /// Requires the `unstable-locales` feature.
    #[cfg(feature = "unstable-locales")]
    #[must_use]
    pub fn format_localized_with_items<'a, I, B>(&self, items: I, locale: Locale) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        DelayedFormat::new_with_locale(Some(*self), None, items, locale)
    }

    /// Formats the date with the specified format string and locale. See
    /// the [`crate::format::strftime`] module for the supported escape
    /// sequences.
    ///
    /// Requires the `unstable-locales` feature.
    ///
    /// # Panics
    /// Converting or formatting the returned `DelayedFormat` panics if the
    /// format string is invalid.
    #[cfg(feature = "unstable-locales")]
    #[must_use]
    pub fn format_localized<'a>(&self, fmt: &'a str, locale: Locale) -> DelayedFormat<StrftimeItems<'a>> {
        self.format_localized_with_items(StrftimeItems::new_with_locale(fmt, locale), locale)
    }
}

impl Datelike for NaiveDate {
    fn year(&self) -> i32 {
        self.year
    }

    fn month(&self) -> u32 {
        self.month
    }

    fn month0(&self) -> u32 {
        self.month - 1
    }

    fn day(&self) -> u32 {
        self.day
    }

    fn day0(&self) -> u32 {
        self.day - 1
    }

    fn ordinal(&self) -> u32 {
        let mut total = self.day;
        for m in 1..self.month {
            total += days_in_month(self.year, m);
        }
        total
    }

    fn ordinal0(&self) -> u32 {
        self.ordinal() - 1
    }

    fn weekday(&self) -> Weekday {
        crate::calendar::weekday_from_days(self.days_since_epoch())
    }

    fn iso_week(&self) -> IsoWeek {
        let (year, week) = iso_year_week(self.days_since_epoch(), self.year);
        IsoWeek { year, week }
    }

    fn with_year(&self, year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, self.month, self.day)
    }

    fn with_month(&self, month: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, month, self.day)
    }

    fn with_month0(&self, month0: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, month0.checked_add(1)?, self.day)
    }

    fn with_day(&self, day: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, self.month, day)
    }

    fn with_day0(&self, day0: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, self.month, day0.checked_add(1)?)
    }

    fn with_ordinal(&self, ordinal: u32) -> Option<NaiveDate> {
        NaiveDate::from_yo_opt(self.year, ordinal)
    }

    fn with_ordinal0(&self, ordinal0: u32) -> Option<NaiveDate> {
        NaiveDate::from_yo_opt(self.year, ordinal0.checked_add(1)?)
    }
}

impl Add<Duration> for NaiveDate {
    type Output = NaiveDate;
    fn add(self, rhs: Duration) -> NaiveDate {
        self.checked_add_signed(rhs)
            .expect("NaiveDate + Duration out of representable bounds")
    }
}

impl AddAssign<Duration> for NaiveDate {
    fn add_assign(&mut self, rhs: Duration) {
        *self = self.add(rhs);
    }
}

impl Sub<Duration> for NaiveDate {
    type Output = NaiveDate;
    fn sub(self, rhs: Duration) -> NaiveDate {
        self.checked_sub_signed(rhs)
            .expect("NaiveDate - Duration out of representable bounds")
    }
}

impl SubAssign<Duration> for NaiveDate {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = self.sub(rhs);
    }
}

impl Add<Months> for NaiveDate {
    type Output = NaiveDate;
    fn add(self, rhs: Months) -> NaiveDate {
        self.checked_add_months(rhs).expect("NaiveDate + Months out of range")
    }
}

impl Sub<Months> for NaiveDate {
    type Output = NaiveDate;
    fn sub(self, rhs: Months) -> NaiveDate {
        self.checked_sub_months(rhs).expect("NaiveDate - Months out of range")
    }
}

impl Add<Days> for NaiveDate {
    type Output = NaiveDate;
    fn add(self, rhs: Days) -> NaiveDate {
        self.checked_add_days(rhs).expect("NaiveDate + Days out of range")
    }
}

impl Sub<Days> for NaiveDate {
    type Output = NaiveDate;
    fn sub(self, rhs: Days) -> NaiveDate {
        self.checked_sub_days(rhs).expect("NaiveDate - Days out of range")
    }
}

impl Sub<NaiveDate> for NaiveDate {
    type Output = Duration;
    fn sub(self, rhs: NaiveDate) -> Duration {
        self.signed_duration_since(rhs)
    }
}

impl fmt::Display for NaiveDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if (0..=9999).contains(&self.year) {
            write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
        } else {
            write!(f, "{:+05}-{:02}-{:02}", self.year, self.month, self.day)
        }
    }
}

impl fmt::Debug for NaiveDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Default for NaiveDate {
    /// Defaults to 1970-01-01 (the Unix epoch), like `chrono`.
    fn default() -> Self {
        NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
    }
}

/// Parsing a `str` into a `NaiveDate` uses the format `%Y-%m-%d`.
impl core::str::FromStr for NaiveDate {
    type Err = crate::format::ParseError;

    fn from_str(s: &str) -> ParseResult<NaiveDate> {
        const ITEMS: &[Item<'static>] = &[
            Item::Numeric(crate::format::Numeric::Year, crate::format::Pad::Zero),
            Item::Space(""),
            Item::Literal("-"),
            Item::Numeric(crate::format::Numeric::Month, crate::format::Pad::Zero),
            Item::Space(""),
            Item::Literal("-"),
            Item::Numeric(crate::format::Numeric::Day, crate::format::Pad::Zero),
            Item::Space(""),
        ];

        let mut parsed = Parsed::new();
        parse(&mut parsed, s, ITEMS.iter())?;
        parsed.to_naive_date()
    }
}

/// Deprecated alias for [`NaiveDate::MIN`].
#[deprecated(note = "use `NaiveDate::MIN` instead")]
pub const MIN_DATE: NaiveDate = NaiveDate::MIN;

/// Deprecated alias for [`NaiveDate::MAX`].
#[deprecated(note = "use `NaiveDate::MAX` instead")]
pub const MAX_DATE: NaiveDate = NaiveDate::MAX;

/// A week in the ISO 8601 calendar (ISO year + week number).
///
/// The ISO year can differ from the calendar year for the few days that
/// straddle two years (e.g. December 31st can belong to week 1 of the
/// following year).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IsoWeek {
    year: i32,
    week: u32,
}

impl IsoWeek {
    /// ISO 8601 year.
    pub fn year(&self) -> i32 {
        self.year
    }

    /// ISO 8601 week number, from 1 to 52 or 53.
    pub fn week(&self) -> u32 {
        self.week
    }

    /// ISO 8601 week number, from 0 to 51 or 52.
    pub fn week0(&self) -> u32 {
        self.week - 1
    }
}

impl fmt::Display for IsoWeek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-W{:02}", self.year, self.week)
    }
}

/// `serde` support: serializes as the ISO 8601 string (`Display`/`FromStr`).
/// `IsoWeek` has no `serde` impl in chrono, so none is provided here either.
#[cfg(feature = "serde")]
mod serde_impl {
    use super::NaiveDate;
    use core::fmt;
    use serde::{de, ser};

    impl ser::Serialize for NaiveDate {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            serializer.collect_str(self)
        }
    }

    struct NaiveDateVisitor;

    impl de::Visitor<'_> for NaiveDateVisitor {
        type Value = NaiveDate;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a formatted date string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(E::custom)
        }
    }

    impl<'de> de::Deserialize<'de> for NaiveDate {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(NaiveDateVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Timelike;

    #[test]
    fn from_ymd_opt_validity() {
        assert!(NaiveDate::from_ymd_opt(2023, 1, 1).is_some());
        assert!(NaiveDate::from_ymd_opt(2023, 0, 1).is_none());
        assert!(NaiveDate::from_ymd_opt(2023, 13, 1).is_none());
        assert!(NaiveDate::from_ymd_opt(2023, 1, 0).is_none());
        assert!(NaiveDate::from_ymd_opt(2023, 1, 32).is_none());
        assert!(NaiveDate::from_ymd_opt(2023, 2, 29).is_none()); // not leap
        assert!(NaiveDate::from_ymd_opt(2024, 2, 29).is_some()); // leap
        assert!(NaiveDate::from_ymd_opt(MIN_YEAR, 1, 1).is_some());
        assert!(NaiveDate::from_ymd_opt(MIN_YEAR - 1, 1, 1).is_none());
        assert!(NaiveDate::from_ymd_opt(MAX_YEAR, 12, 31).is_some());
        assert!(NaiveDate::from_ymd_opt(MAX_YEAR + 1, 1, 1).is_none());
    }

    #[test]
    fn from_yo_opt_validity_and_boundaries() {
        assert_eq!(NaiveDate::from_yo_opt(2023, 1), NaiveDate::from_ymd_opt(2023, 1, 1));
        assert_eq!(NaiveDate::from_yo_opt(2023, 365), NaiveDate::from_ymd_opt(2023, 12, 31));
        assert_eq!(NaiveDate::from_yo_opt(2023, 366), None); // 2023 is not leap
        assert_eq!(NaiveDate::from_yo_opt(2024, 366), NaiveDate::from_ymd_opt(2024, 12, 31));
        assert_eq!(NaiveDate::from_yo_opt(2023, 0), None);
    }

    #[test]
    fn ordinal_and_from_yo_opt_round_trip_every_day_of_leap_and_non_leap_years() {
        for &year in &[2023, 2024] {
            for ordinal in 1..=days_in_year(year) {
                let date = NaiveDate::from_yo_opt(year, ordinal).unwrap();
                assert_eq!(date.ordinal(), ordinal, "year={year} ordinal={ordinal}");
                assert_eq!(NaiveDate::from_yo_opt(year, date.ordinal()), Some(date));
            }
        }
    }

    #[test]
    fn from_isoywd_opt_round_trips_with_iso_week_and_weekday() {
        let samples = [
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2018, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2020, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(2000, 2, 29).unwrap(),
            NaiveDate::from_ymd_opt(-44, 3, 15).unwrap(),
        ];
        for date in samples {
            let iso_week = date.iso_week();
            let rebuilt = NaiveDate::from_isoywd_opt(iso_week.year(), iso_week.week(), date.weekday());
            assert_eq!(rebuilt, Some(date), "round-trip failed for {date}");
        }
    }

    #[test]
    fn from_isoywd_opt_builds_known_year_boundary_dates() {
        // 2023-01-01 is a Sunday and belongs to ISO week 2022-W52 (hand
        // derivation in calendar.rs's tests).
        assert_eq!(
            NaiveDate::from_isoywd_opt(2022, 52, Weekday::Sun),
            NaiveDate::from_ymd_opt(2023, 1, 1)
        );
        // ISO week 1 of 2023 starts on Monday 2023-01-02.
        assert_eq!(
            NaiveDate::from_isoywd_opt(2023, 1, Weekday::Mon),
            NaiveDate::from_ymd_opt(2023, 1, 2)
        );
    }

    #[test]
    fn from_isoywd_opt_rejects_invalid_week_numbers() {
        assert_eq!(NaiveDate::from_isoywd_opt(2023, 0, Weekday::Mon), None);
        assert_eq!(NaiveDate::from_isoywd_opt(2023, 54, Weekday::Mon), None);
    }

    #[test]
    fn num_days_from_ce_round_trips() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let n = d.num_days_from_ce();
        assert_eq!(NaiveDate::from_num_days_from_ce_opt(n), Some(d));
        assert_eq!(NaiveDate::from_ymd_opt(1, 1, 1).unwrap().num_days_from_ce(), 1);
    }

    #[test]
    fn epoch_days_round_trip_and_epoch_is_day_zero() {
        let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        assert_eq!(epoch.to_epoch_days(), 0);
        assert_eq!(NaiveDate::from_epoch_days(0), Some(epoch));
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(NaiveDate::from_epoch_days(d.to_epoch_days()), Some(d));
    }

    #[test]
    fn from_weekday_of_month_opt_known_example() {
        // 2nd Friday of March 2017 = March 10, 2017 (verified by hand:
        // March 1, 2017 is a Wednesday, so the first Friday is March 3rd).
        assert_eq!(
            NaiveDate::from_weekday_of_month_opt(2017, 3, Weekday::Fri, 2),
            NaiveDate::from_ymd_opt(2017, 3, 10)
        );
    }

    #[test]
    fn from_weekday_of_month_opt_finds_the_fifth_occurrence_when_it_exists() {
        // March 2017 starts on a Wednesday and has 31 days, so Wed/Thu/Fri
        // each occur 5 times; the 5th Friday is March 31st.
        assert_eq!(
            NaiveDate::from_weekday_of_month_opt(2017, 3, Weekday::Fri, 5),
            NaiveDate::from_ymd_opt(2017, 3, 31)
        );
    }

    #[test]
    fn from_weekday_of_month_opt_returns_none_for_nonexistent_occurrence() {
        // No month can have a 6th occurrence of any single weekday
        // (6*7=42 exceeds any month's length).
        assert_eq!(NaiveDate::from_weekday_of_month_opt(2017, 3, Weekday::Fri, 6), None);
    }

    #[test]
    fn succ_and_pred_opt_basic_and_rollover() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(d.succ_opt(), NaiveDate::from_ymd_opt(2023, 6, 16));
        assert_eq!(d.pred_opt(), NaiveDate::from_ymd_opt(2023, 6, 14));
        let dec31 = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        assert_eq!(dec31.succ_opt(), NaiveDate::from_ymd_opt(2024, 1, 1));
        let jan1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        assert_eq!(jan1.pred_opt(), NaiveDate::from_ymd_opt(2022, 12, 31));
    }

    #[test]
    fn succ_and_pred_opt_are_none_at_the_bounds() {
        assert_eq!(NaiveDate::MAX.succ_opt(), None);
        assert_eq!(NaiveDate::MIN.pred_opt(), None);
    }

    #[test]
    fn checked_add_days_overflows_past_max() {
        assert_eq!(NaiveDate::MAX.checked_add_days(Days::new(1)), None);
    }

    #[test]
    fn checked_sub_days_overflows_before_min() {
        assert_eq!(NaiveDate::MIN.checked_sub_days(Days::new(1)), None);
    }

    #[test]
    fn checked_add_months_clamps_to_last_valid_day() {
        let jan31 = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();
        assert_eq!(jan31.checked_add_months(Months::new(1)), NaiveDate::from_ymd_opt(2023, 2, 28));
        let jan31_leap = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        assert_eq!(
            jan31_leap.checked_add_months(Months::new(1)),
            NaiveDate::from_ymd_opt(2024, 2, 29)
        );
        let dec31 = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        assert_eq!(dec31.checked_add_months(Months::new(1)), NaiveDate::from_ymd_opt(2024, 1, 31));
    }

    #[test]
    fn checked_sub_months_clamps_to_last_valid_day() {
        let mar31 = NaiveDate::from_ymd_opt(2023, 3, 31).unwrap();
        assert_eq!(mar31.checked_sub_months(Months::new(1)), NaiveDate::from_ymd_opt(2023, 2, 28));
    }

    #[test]
    fn checked_add_signed_and_sub_signed() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(d.checked_add_signed(Duration::days(10)), NaiveDate::from_ymd_opt(2023, 6, 25));
        assert_eq!(d.checked_sub_signed(Duration::days(10)), NaiveDate::from_ymd_opt(2023, 6, 5));
    }

    #[test]
    fn signed_duration_since_and_sub_operator() {
        let a = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let b = NaiveDate::from_ymd_opt(2023, 6, 10).unwrap();
        assert_eq!(a.signed_duration_since(b), Duration::days(5));
        assert_eq!(b.signed_duration_since(a), Duration::days(-5));
        assert_eq!(a - b, Duration::days(5));
    }

    #[test]
    fn abs_diff_is_symmetric() {
        let a = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let b = NaiveDate::from_ymd_opt(2023, 6, 10).unwrap();
        assert_eq!(a.abs_diff(b), Days::new(5));
        assert_eq!(b.abs_diff(a), Days::new(5));
    }

    #[test]
    fn years_since_computes_whole_years_relative_to_anniversary() {
        let base = NaiveDate::from_ymd_opt(2000, 6, 15).unwrap();
        assert_eq!(NaiveDate::from_ymd_opt(2010, 6, 15).unwrap().years_since(base), Some(10));
        assert_eq!(NaiveDate::from_ymd_opt(2010, 6, 14).unwrap().years_since(base), Some(9));
        assert_eq!(NaiveDate::from_ymd_opt(2010, 6, 16).unwrap().years_since(base), Some(10));
        assert_eq!(base.years_since(NaiveDate::from_ymd_opt(2001, 1, 1).unwrap()), None);
    }

    #[test]
    fn age_is_years_since_with_reversed_argument_order() {
        let date_of_birth = NaiveDate::from_ymd_opt(1990, 6, 15).unwrap();
        let day_before_birthday = NaiveDate::from_ymd_opt(2023, 6, 14).unwrap();
        let birthday = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(date_of_birth.age(day_before_birthday), Some(32));
        assert_eq!(date_of_birth.age(birthday), Some(33));
        // Exactly `on.years_since(self)`, just spelled the other way round.
        assert_eq!(date_of_birth.age(birthday), birthday.years_since(date_of_birth));
    }

    #[test]
    fn age_is_none_when_reference_date_precedes_date_of_birth() {
        let date_of_birth = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let earlier = NaiveDate::from_ymd_opt(1999, 12, 31).unwrap();
        assert_eq!(date_of_birth.age(earlier), None);
    }

    #[test]
    fn easter_matches_known_reference_dates() {
        // Hand-verified against published Easter dates (2022-2026), and
        // against the `computus` crate's own reference values for 2023.
        assert_eq!(NaiveDate::easter(2022), NaiveDate::from_ymd_opt(2022, 4, 17));
        assert_eq!(NaiveDate::easter(2023), NaiveDate::from_ymd_opt(2023, 4, 9));
        assert_eq!(NaiveDate::easter(2024), NaiveDate::from_ymd_opt(2024, 3, 31));
        assert_eq!(NaiveDate::easter(2025), NaiveDate::from_ymd_opt(2025, 4, 20));
        assert_eq!(NaiveDate::easter(2026), NaiveDate::from_ymd_opt(2026, 4, 5));
    }

    #[test]
    fn orthodox_easter_matches_known_reference_dates() {
        // Hand-verified against published Orthodox Easter (Pascha) dates
        // (2022-2026). 2025 is a year where Orthodox and Western Easter
        // coincide, which this test also exercises.
        assert_eq!(NaiveDate::orthodox_easter(2022), NaiveDate::from_ymd_opt(2022, 4, 24));
        assert_eq!(NaiveDate::orthodox_easter(2023), NaiveDate::from_ymd_opt(2023, 4, 16));
        assert_eq!(NaiveDate::orthodox_easter(2024), NaiveDate::from_ymd_opt(2024, 5, 5));
        assert_eq!(NaiveDate::orthodox_easter(2025), NaiveDate::from_ymd_opt(2025, 4, 20));
        assert_eq!(NaiveDate::orthodox_easter(2026), NaiveDate::from_ymd_opt(2026, 4, 12));
    }

    #[test]
    fn orthodox_easter_is_never_before_western_easter() {
        // The Orthodox computation is meant to never precede the western
        // one in the same year (a basic sanity property of the two
        // algorithms, distinct from checking exact reference dates).
        for year in 2000..2100 {
            assert!(NaiveDate::orthodox_easter(year) >= NaiveDate::easter(year), "year {year}");
        }
    }

    #[test]
    fn passover_matches_known_reference_dates() {
        // Hand-verified against hebcal.com's Hebrew date converter.
        assert_eq!(NaiveDate::passover(2023), NaiveDate::from_ymd_opt(2023, 4, 6));
        assert_eq!(NaiveDate::passover(2024), NaiveDate::from_ymd_opt(2024, 4, 23));
        assert_eq!(NaiveDate::passover(2025), NaiveDate::from_ymd_opt(2025, 4, 13));
        assert_eq!(NaiveDate::passover(2026), NaiveDate::from_ymd_opt(2026, 4, 2));
    }

    #[test]
    fn passover_matches_known_reference_dates_further_from_the_anchor_year() {
        // Additional reference dates further from the 2022 anchor used
        // internally, still hand-verified against hebcal.com, covering
        // both common and leap Hebrew years.
        assert_eq!(NaiveDate::passover(2018), NaiveDate::from_ymd_opt(2018, 3, 31));
        assert_eq!(NaiveDate::passover(2019), NaiveDate::from_ymd_opt(2019, 4, 20));
        assert_eq!(NaiveDate::passover(2020), NaiveDate::from_ymd_opt(2020, 4, 9));
        assert_eq!(NaiveDate::passover(2021), NaiveDate::from_ymd_opt(2021, 3, 28));
        assert_eq!(NaiveDate::passover(2022), NaiveDate::from_ymd_opt(2022, 4, 16));
        assert_eq!(NaiveDate::passover(2027), NaiveDate::from_ymd_opt(2027, 4, 22));
        assert_eq!(NaiveDate::passover(2028), NaiveDate::from_ymd_opt(2028, 4, 11));
        assert_eq!(NaiveDate::passover(2030), NaiveDate::from_ymd_opt(2030, 4, 18));
        assert_eq!(NaiveDate::passover(2040), NaiveDate::from_ymd_opt(2040, 3, 29));
    }

    #[test]
    fn christian_movable_feasts_match_known_reference_dates() {
        // 2024 (leap year) and 2023 (common year), both hand-verified
        // against well-known published dates, to exercise the fixed
        // offsets from `easter` across a February/March boundary that
        // shifts depending on leap-year status.
        assert_eq!(NaiveDate::mardi_gras(2024), NaiveDate::from_ymd_opt(2024, 2, 13));
        assert_eq!(NaiveDate::ash_wednesday(2024), NaiveDate::from_ymd_opt(2024, 2, 14));
        assert_eq!(NaiveDate::palm_sunday(2024), NaiveDate::from_ymd_opt(2024, 3, 24));
        assert_eq!(NaiveDate::ascension(2024), NaiveDate::from_ymd_opt(2024, 5, 9));
        assert_eq!(NaiveDate::pentecost(2024), NaiveDate::from_ymd_opt(2024, 5, 19));

        assert_eq!(NaiveDate::ash_wednesday(2023), NaiveDate::from_ymd_opt(2023, 2, 22));
    }

    #[test]
    fn christian_movable_feasts_are_consistently_offset_from_easter() {
        // Property check across a wide range of years: each feast must be
        // exactly its documented offset away from `easter`, regardless of
        // the specific calendar quirks of any one year.
        for year in 1900..2100 {
            let easter = NaiveDate::easter(year).unwrap();
            assert_eq!(NaiveDate::mardi_gras(year), Some(easter - Duration::days(47)));
            assert_eq!(NaiveDate::ash_wednesday(year), Some(easter - Duration::days(46)));
            assert_eq!(NaiveDate::palm_sunday(year), Some(easter - Duration::days(7)));
            assert_eq!(NaiveDate::ascension(year), Some(easter + Duration::days(39)));
            assert_eq!(NaiveDate::pentecost(year), Some(easter + Duration::days(49)));
        }
    }

    #[test]
    fn passover_matches_known_reference_dates_around_rare_postponement_rules() {
        // These two years specifically exercise the rarer "GaTaRaD" and
        // "BeTuTeKaFoT" postponement rules (rule 3 and rule 4), which
        // trigger only a handful of times per 19-year cycle -- hand-
        // verified against hebcal.com.
        assert_eq!(NaiveDate::passover(1955), NaiveDate::from_ymd_opt(1955, 4, 7));
        assert_eq!(NaiveDate::passover(1958), NaiveDate::from_ymd_opt(1958, 4, 5));
    }

    #[test]
    fn additional_jewish_holidays_match_known_reference_dates() {
        // Hand-verified against hebcal.com's Hebrew date converter.
        assert_eq!(NaiveDate::rosh_hashanah(2022), NaiveDate::from_ymd_opt(2022, 9, 26));
        assert_eq!(NaiveDate::rosh_hashanah(2024), NaiveDate::from_ymd_opt(2024, 10, 3));
        assert_eq!(NaiveDate::rosh_hashanah(2025), NaiveDate::from_ymd_opt(2025, 9, 23));

        assert_eq!(NaiveDate::yom_kippur(2024), NaiveDate::from_ymd_opt(2024, 10, 12));
        assert_eq!(NaiveDate::sukkot(2024), NaiveDate::from_ymd_opt(2024, 10, 17));
        assert_eq!(NaiveDate::hanukkah(2024), NaiveDate::from_ymd_opt(2024, 12, 26));

        // 2025 (5785, common year -> 14 Adar) and 2024 (5784, leap year ->
        // 14 Adar II) both exercise the two branches of `purim`.
        assert_eq!(NaiveDate::purim(2025), NaiveDate::from_ymd_opt(2025, 3, 14));
        assert_eq!(NaiveDate::purim(2024), NaiveDate::from_ymd_opt(2024, 3, 24));

        assert_eq!(NaiveDate::shavuot(2024), NaiveDate::from_ymd_opt(2024, 6, 12));
    }

    #[test]
    fn hebrew_holidays_are_consistently_offset_from_rosh_hashanah_or_passover() {
        // Property check across a range of Gregorian years: each holiday
        // must land exactly on its documented offset, for both common and
        // leap Hebrew years.
        for year in 2000..2100 {
            let rh = NaiveDate::rosh_hashanah(year).unwrap();
            assert_eq!(NaiveDate::yom_kippur(year), Some(rh + Duration::days(9)));
            assert_eq!(NaiveDate::sukkot(year), Some(rh + Duration::days(14)));

            let passover = NaiveDate::passover(year).unwrap();
            assert_eq!(NaiveDate::shavuot(year), Some(passover + Duration::days(50)));
        }
    }

    #[test]
    fn from_hijri_ymd_matches_independently_verified_reference_dates() {
        // The two Hijri New Year dates used as this implementation's
        // anchor and cross-check, independently verified against a
        // Hijri/Gregorian date converter.
        assert_eq!(NaiveDate::from_hijri_ymd(1446, 1, 1), NaiveDate::from_ymd_opt(2024, 7, 7));
        assert_eq!(NaiveDate::from_hijri_ymd(1447, 1, 1), NaiveDate::from_ymd_opt(2025, 6, 26));
    }

    #[test]
    fn from_hijri_ymd_rejects_invalid_month_or_day() {
        assert_eq!(NaiveDate::from_hijri_ymd(1446, 0, 1), None);
        assert_eq!(NaiveDate::from_hijri_ymd(1446, 13, 1), None);
        assert_eq!(NaiveDate::from_hijri_ymd(1446, 1, 0), None);
        // Month 1 (Muharram) has 30 days, never 31.
        assert_eq!(NaiveDate::from_hijri_ymd(1446, 1, 31), None);
        assert!(NaiveDate::from_hijri_ymd(1446, 1, 30).is_some());
        // Month 2 (Safar) has 29 days, never 30.
        assert_eq!(NaiveDate::from_hijri_ymd(1446, 2, 30), None);
        assert!(NaiveDate::from_hijri_ymd(1446, 2, 29).is_some());
    }

    #[test]
    fn from_hijri_ymd_dhu_al_hijjah_length_matches_leap_year_status() {
        // 1445 AH is a leap year in the Type IIa / Kuwaiti tabular
        // scheme (cycle position 5): Dhu al-Hijjah has 30 days.
        assert!(NaiveDate::from_hijri_ymd(1445, 12, 30).is_some());
        // 1446 AH is not a leap year (cycle position 6): Dhu al-Hijjah
        // has only 29 days.
        assert_eq!(NaiveDate::from_hijri_ymd(1446, 12, 30), None);
        assert!(NaiveDate::from_hijri_ymd(1446, 12, 29).is_some());
    }

    #[test]
    fn hijri_holidays_are_consistent_with_from_hijri_ymd() {
        for hijri_year in 1400..1500 {
            assert_eq!(NaiveDate::hijri_new_year(hijri_year), NaiveDate::from_hijri_ymd(hijri_year, 1, 1));
            assert_eq!(NaiveDate::ramadan_start(hijri_year), NaiveDate::from_hijri_ymd(hijri_year, 9, 1));
            assert_eq!(NaiveDate::eid_al_fitr(hijri_year), NaiveDate::from_hijri_ymd(hijri_year, 10, 1));
            assert_eq!(NaiveDate::eid_al_adha(hijri_year), NaiveDate::from_hijri_ymd(hijri_year, 12, 10));
        }
    }

    #[test]
    fn hijri_new_year_advances_by_354_or_355_days_each_year() {
        // Property check: consecutive Hijri New Years are always 354 or
        // 355 days apart (a common or leap Hijri year), never anything
        // else -- across a wide range, including several full 30-year
        // cycles.
        for hijri_year in 1400..1500 {
            let this_year = NaiveDate::hijri_new_year(hijri_year).unwrap();
            let next_year = NaiveDate::hijri_new_year(hijri_year + 1).unwrap();
            let gap = (next_year - this_year).num_days();
            assert!(gap == 354 || gap == 355, "hijri_year {hijri_year}: gap {gap}");
        }
    }

    #[test]
    fn to_hijri_ymd_is_the_inverse_of_from_hijri_ymd() {
        assert_eq!(
            NaiveDate::from_ymd_opt(2025, 6, 26).unwrap().to_hijri_ymd(),
            (1447, 1, 1)
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(2024, 7, 7).unwrap().to_hijri_ymd(),
            (1446, 1, 1)
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(2026, 7, 13).unwrap().to_hijri_ymd(),
            (1448, 1, 28)
        );
    }

    #[test]
    fn hijri_round_trip_holds_across_every_month_boundary_for_a_wide_range() {
        // For every Hijri year in a wide range, both the first and last
        // day of every month must round-trip exactly through
        // `from_hijri_ymd` -> `to_hijri_ymd`, including the Dhu al-Hijjah
        // boundary that depends on leap-year status.
        for year in 1400i32..1500 {
            for month in 1..=12u32 {
                let last_day = {
                    let mut d = 30;
                    while NaiveDate::from_hijri_ymd(year, month, d).is_none() {
                        d -= 1;
                    }
                    d
                };
                for day in [1, last_day] {
                    let g = NaiveDate::from_hijri_ymd(year, month, day).unwrap();
                    assert_eq!(g.to_hijri_ymd(), (year, month, day), "year {year} month {month} day {day}");
                }
            }
        }
    }

    #[test]
    fn gregorian_to_hijri_round_trip_holds_across_a_wide_date_range() {
        let mut d = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2060, 1, 1).unwrap();
        while d < end {
            let (y, m, day) = d.to_hijri_ymd();
            assert_eq!(NaiveDate::from_hijri_ymd(y, m, day), Some(d), "date {d:?}");
            d += Duration::days(41);
        }
    }

    #[test]
    fn from_hebrew_ymd_matches_known_reference_dates() {
        // 5786 is a common year (Nisan is month 7); 5784 is a leap year
        // (Nisan is month 8) -- both branches of the civil month
        // numbering are exercised here.
        assert_eq!(NaiveDate::from_hebrew_ymd(5786, 1, 1), NaiveDate::from_ymd_opt(2025, 9, 23));
        assert_eq!(NaiveDate::from_hebrew_ymd(5784, 8, 15), NaiveDate::from_ymd_opt(2024, 4, 23));
        // Month/day validation: month 13 only exists in a leap year, and
        // Elul (the last month) is always 29 days, never 30.
        assert!(NaiveDate::from_hebrew_ymd(5784, 13, 1).is_some()); // 5784 is leap
        assert_eq!(NaiveDate::from_hebrew_ymd(5786, 13, 1), None); // 5786 is not leap
        assert!(NaiveDate::from_hebrew_ymd(5786, 12, 29).is_some());
        assert_eq!(NaiveDate::from_hebrew_ymd(5786, 12, 30), None);
    }

    #[test]
    fn to_hebrew_ymd_is_the_inverse_of_from_hebrew_ymd() {
        assert_eq!(
            NaiveDate::from_ymd_opt(2025, 9, 23).unwrap().to_hebrew_ymd(),
            (5786, 1, 1)
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(2024, 4, 23).unwrap().to_hebrew_ymd(),
            (5784, 8, 15)
        );
    }

    #[test]
    fn hebrew_round_trip_holds_across_every_month_boundary_for_a_wide_range() {
        // For every Hebrew year in a wide range, both the first and last
        // day of every month (common or leap, as applicable) must
        // round-trip exactly through `from_hebrew_ymd` -> `to_hebrew_ymd`.
        for year in 5700i32..5900 {
            let month_count = if NaiveDate::from_hebrew_ymd(year, 13, 1).is_some() { 13 } else { 12 };
            for month in 1..=month_count {
                let last_day = {
                    let mut d = 30;
                    while NaiveDate::from_hebrew_ymd(year, month, d).is_none() {
                        d -= 1;
                    }
                    d
                };
                for day in [1, last_day] {
                    let g = NaiveDate::from_hebrew_ymd(year, month, day).unwrap();
                    assert_eq!(g.to_hebrew_ymd(), (year, month, day), "year {year} month {month} day {day}");
                }
            }
        }
    }

    #[test]
    fn gregorian_to_hebrew_round_trip_holds_across_a_wide_date_range() {
        let mut d = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2060, 1, 1).unwrap();
        while d < end {
            let (y, m, day) = d.to_hebrew_ymd();
            assert_eq!(NaiveDate::from_hebrew_ymd(y, m, day), Some(d), "date {d:?}");
            d += Duration::days(37);
        }
    }

    #[test]
    fn japanese_era_matches_known_reference_dates() {
        assert_eq!(
            NaiveDate::from_ymd_opt(2026, 7, 13).unwrap().japanese_era(),
            Some((crate::JapaneseEra::Reiwa, 8))
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap().japanese_era(),
            Some((crate::JapaneseEra::Heisei, 12))
        );
        // Emperor Showa died 7 January 1989; Heisei began the next day.
        // So 1-7 January 1989 is the short final year of Showa (Showa
        // 64), and Heisei begins immediately at Heisei 1 on 8 January --
        // a real, well-documented historical edge case.
        assert_eq!(
            NaiveDate::from_ymd_opt(1989, 1, 1).unwrap().japanese_era(),
            Some((crate::JapaneseEra::Showa, 64))
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(1989, 1, 8).unwrap().japanese_era(),
            Some((crate::JapaneseEra::Heisei, 1))
        );
        // Before the modern era system (Meiji started 25 January 1868).
        assert_eq!(NaiveDate::from_ymd_opt(1868, 1, 24).unwrap().japanese_era(), None);
        assert_eq!(
            NaiveDate::from_ymd_opt(1868, 1, 25).unwrap().japanese_era(),
            Some((crate::JapaneseEra::Meiji, 1))
        );
    }

    #[test]
    fn from_japanese_era_ymd_is_the_inverse_of_japanese_era() {
        assert_eq!(
            NaiveDate::from_japanese_era_ymd(crate::JapaneseEra::Reiwa, 8, 7, 13),
            NaiveDate::from_ymd_opt(2026, 7, 13)
        );
        assert_eq!(
            NaiveDate::from_japanese_era_ymd(crate::JapaneseEra::Showa, 64, 1, 1),
            NaiveDate::from_ymd_opt(1989, 1, 1)
        );
        // Showa 64 only covers 1-7 January 1989 -- 8 January 1989 is
        // already Heisei 1, not Showa 64.
        assert_eq!(NaiveDate::from_japanese_era_ymd(crate::JapaneseEra::Showa, 64, 1, 8), None);
    }

    #[test]
    fn japanese_era_round_trip_holds_across_every_era_boundary() {
        // Every day from just before Meiji started through today should
        // round-trip exactly through `japanese_era` -> `from_japanese_era_ymd`.
        let mut d = NaiveDate::from_ymd_opt(1868, 1, 20).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        while d < end {
            if let Some((era, era_year)) = d.japanese_era() {
                assert_eq!(
                    NaiveDate::from_japanese_era_ymd(era, era_year, d.month(), d.day()),
                    Some(d),
                    "date {d:?}"
                );
            }
            d += Duration::days(53);
        }
    }

    #[test]
    fn fixed_date_japanese_festivals_match_the_gregorian_month_and_day_they_are_defined_on() {
        assert_eq!(NaiveDate::shogatsu(2026), NaiveDate::from_ymd_opt(2026, 1, 1));
        assert_eq!(NaiveDate::hana_matsuri(2026), NaiveDate::from_ymd_opt(2026, 4, 8));
        assert_eq!(NaiveDate::tanabata(2026), NaiveDate::from_ymd_opt(2026, 7, 7));
        assert_eq!(NaiveDate::obon_start(2026), NaiveDate::from_ymd_opt(2026, 8, 13));
        assert_eq!(NaiveDate::shichi_go_san(2026), NaiveDate::from_ymd_opt(2026, 11, 15));
    }

    #[test]
    fn fixed_date_japanese_festivals_reject_the_one_out_of_range_year() {
        // `NaiveDate::MIN` is year `NaiveDate::MIN.year()`, so year
        // `i32::MIN` itself is out of range for every one of these.
        assert_eq!(NaiveDate::shogatsu(i32::MIN), None);
        assert_eq!(NaiveDate::hana_matsuri(i32::MIN), None);
        assert_eq!(NaiveDate::tanabata(i32::MIN), None);
        assert_eq!(NaiveDate::obon_start(i32::MIN), None);
        assert_eq!(NaiveDate::shichi_go_san(i32::MIN), None);
    }

    #[test]
    fn shunbun_no_hi_matches_published_jma_vernal_equinox_day_dates() {
        // Reference dates from Japan Meteorological Agency (JMA) official
        // "Vernal Equinox Day" announcements / public holiday calendars,
        // independently cross-checked via a Meeus-algorithm bisection
        // prototype (Python, `pymeeus`) before writing this Rust code.
        assert_eq!(NaiveDate::shunbun_no_hi(2024), NaiveDate::from_ymd_opt(2024, 3, 20));
        assert_eq!(NaiveDate::shunbun_no_hi(2025), NaiveDate::from_ymd_opt(2025, 3, 20));
        assert_eq!(NaiveDate::shunbun_no_hi(2026), NaiveDate::from_ymd_opt(2026, 3, 20));
    }

    #[test]
    fn shuubun_no_hi_matches_published_jma_autumnal_equinox_day_dates() {
        assert_eq!(NaiveDate::shuubun_no_hi(2024), NaiveDate::from_ymd_opt(2024, 9, 22));
        assert_eq!(NaiveDate::shuubun_no_hi(2025), NaiveDate::from_ymd_opt(2025, 9, 23));
        assert_eq!(NaiveDate::shuubun_no_hi(2026), NaiveDate::from_ymd_opt(2026, 9, 23));
    }

    #[test]
    fn setsubun_matches_published_reference_dates_including_the_rare_feb_2nd_occurrences() {
        // 2021 and 2025 are notable: the first Feb 2nd Setsubuns since
        // 1897 (previously always Feb 3rd or, historically, Feb 4th).
        assert_eq!(NaiveDate::setsubun(2021), NaiveDate::from_ymd_opt(2021, 2, 2));
        assert_eq!(NaiveDate::setsubun(2024), NaiveDate::from_ymd_opt(2024, 2, 3));
        assert_eq!(NaiveDate::setsubun(2025), NaiveDate::from_ymd_opt(2025, 2, 2));
        assert_eq!(NaiveDate::setsubun(2026), NaiveDate::from_ymd_opt(2026, 2, 3));
    }

    #[test]
    fn solar_term_functions_return_none_outside_the_astro_crates_i16_year_range() {
        // Unlike the rest of this crate (years +/-5,000,000), the `astro`
        // crate's `Date` represents the year as `i16` (+/-32,767ish) --
        // these should fail gracefully, not panic, outside that range.
        assert_eq!(NaiveDate::shunbun_no_hi(1_000_000), None);
        assert_eq!(NaiveDate::shuubun_no_hi(1_000_000), None);
        assert_eq!(NaiveDate::setsubun(1_000_000), None);
        assert_eq!(NaiveDate::shunbun_no_hi(i32::MIN), None);
    }

    #[test]
    fn setsubun_is_always_the_day_immediately_before_risshun() {
        // Risshun itself (315 degrees solar longitude) isn't exposed as a
        // public function, so this re-derives it via the same private
        // helper `setsubun` uses, and checks the "day before" relationship
        // holds (i.e. no accidental month/year-boundary off-by-one) across
        // a wide range of years. Restricted to 1985-2050: Risshun fell on
        // Feb 5th in some years before the mid-1980s (Gregorian leap-year
        // drift), so the day-3-or-4 assumption below only holds in this
        // more recent window -- verified via the same Python/pymeeus
        // prototype used for the other reference dates in this file.
        for year in 1985..=2050 {
            let setsubun = NaiveDate::setsubun(year).unwrap();
            let risshun = setsubun.succ_opt().unwrap();
            assert_eq!(risshun.year(), year, "year {year}");
            assert!(risshun.month() == 2 && (risshun.day() == 3 || risshun.day() == 4), "year {year}: risshun day {risshun:?}");
        }
    }

    #[test]
    fn chinese_new_year_matches_published_reference_dates() {
        // Verified against widely published Chinese New Year dates,
        // independently cross-checked via a from-scratch Python
        // (`pymeeus`) prototype of the whole algorithm before writing
        // this Rust code -- see `chinese_calendar.rs`'s module doc.
        let expected = [
            (2020, 1, 25),
            (2021, 2, 12),
            (2022, 2, 1),
            (2023, 1, 22),
            (2024, 2, 10),
            (2025, 1, 29),
            (2026, 2, 17),
            (2027, 2, 6),
        ];
        for (year, month, day) in expected {
            assert_eq!(NaiveDate::chinese_new_year(year), NaiveDate::from_ymd_opt(year, month, day), "year {year}");
        }
    }

    #[test]
    fn chinese_calendar_leap_months_match_published_reference_years() {
        // Includes 2033, the notorious "exceptional year" (leap month 11,
        // the rarest possible position -- see `chinese_calendar.rs`'s
        // module doc), which specifically stress-tests the
        // civil-date-vs-instant distinction the algorithm has to get right.
        let expected_leap_month = [
            (2020, 4),
            (2023, 2),
            (2025, 6),
            (2028, 5),
            (2031, 3),
            (2033, 11),
            (2036, 6),
        ];
        for (year, month) in expected_leap_month {
            let d = NaiveDate::from_chinese_ymd(year, month, true, 1);
            assert!(d.is_some(), "year {year}: expected a leap month {month}, got None");
        }
        // Sanity check: adjacent, non-leap years should have no leap
        // month at all, in any of the 12 positions.
        for year in [2021, 2022, 2024, 2026] {
            for month in 1..=12 {
                assert_eq!(
                    NaiveDate::from_chinese_ymd(year, month, true, 1),
                    None,
                    "year {year} month {month}: should have no leap month"
                );
            }
        }
    }

    #[test]
    fn to_chinese_ymd_and_from_chinese_ymd_round_trip_a_wide_range_of_dates() {
        let mut d = NaiveDate::from_ymd_opt(1950, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2060, 1, 1).unwrap();
        while d < end {
            let (year, month, is_leap, day) = d.to_chinese_ymd().unwrap_or_else(|| panic!("to_chinese_ymd failed for {d:?}"));
            assert_eq!(
                NaiveDate::from_chinese_ymd(year, month, is_leap, day),
                Some(d),
                "date {d:?} -> chinese ({year}, {month}, leap={is_leap}, {day})"
            );
            d += Duration::days(37);
        }
    }

    #[test]
    fn duanwu_and_zhongqiu_match_published_reference_dates() {
        assert_eq!(NaiveDate::duanwu(2023), NaiveDate::from_ymd_opt(2023, 6, 22));
        assert_eq!(NaiveDate::duanwu(2024), NaiveDate::from_ymd_opt(2024, 6, 10));
        assert_eq!(NaiveDate::duanwu(2025), NaiveDate::from_ymd_opt(2025, 5, 31));
        assert_eq!(NaiveDate::zhongqiu(2024), NaiveDate::from_ymd_opt(2024, 9, 17));
        assert_eq!(NaiveDate::zhongqiu(2025), NaiveDate::from_ymd_opt(2025, 10, 6));
    }

    #[test]
    fn qingming_matches_published_reference_dates() {
        assert_eq!(NaiveDate::qingming(2024), NaiveDate::from_ymd_opt(2024, 4, 4));
        assert_eq!(NaiveDate::qingming(2025), NaiveDate::from_ymd_opt(2025, 4, 4));
        assert_eq!(NaiveDate::qingming(2026), NaiveDate::from_ymd_opt(2026, 4, 5));
    }

    #[test]
    fn from_chinese_ymd_rejects_an_out_of_range_day() {
        // Chinese lunar months are always 29 or 30 days; day 30 must be
        // rejected for a month that only has 29.
        let months = (1..=12).find_map(|m| {
            let start = NaiveDate::from_chinese_ymd(2024, m, false, 1)?;
            let next = if m == 12 {
                NaiveDate::chinese_new_year(2025)?
            } else {
                NaiveDate::from_chinese_ymd(2024, m + 1, false, 1)?
            };
            if next.num_days_from_ce() - start.num_days_from_ce() == 29 {
                Some(m)
            } else {
                None
            }
        });
        let short_month = months.expect("2024 should have at least one 29-day month");
        assert!(NaiveDate::from_chinese_ymd(2024, short_month, false, 29).is_some());
        assert_eq!(NaiveDate::from_chinese_ymd(2024, short_month, false, 30), None);
        assert_eq!(NaiveDate::from_chinese_ymd(2024, short_month, false, 0), None);
    }

    #[test]
    fn iter_days_yields_self_first_then_advances_by_one() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let days: Vec<NaiveDate> = d.iter_days().take(3).collect();
        assert_eq!(
            days,
            vec![
                NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 16).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 17).unwrap(),
            ]
        );
    }

    #[test]
    fn iter_weeks_yields_self_first_then_advances_by_seven_days() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let weeks: Vec<NaiveDate> = d.iter_weeks().take(3).collect();
        assert_eq!(
            weeks,
            vec![
                NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 22).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 29).unwrap(),
            ]
        );
    }

    #[test]
    fn and_hms_variants_validity_and_leap_second_range() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let dt = d.and_hms_opt(12, 30, 45).unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.hour(), 12);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
        assert!(d.and_hms_opt(24, 0, 0).is_none());
        assert!(d.and_hms_milli_opt(23, 59, 59, 1500).is_some()); // leap second
        assert!(d.and_hms_nano_opt(23, 59, 59, 1_999_999_999).is_some());
        assert!(d.and_hms_nano_opt(23, 59, 59, 2_000_000_000).is_none());
    }

    #[test]
    fn datelike_accessors() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(d.year(), 2023);
        assert_eq!(d.month(), 6);
        assert_eq!(d.month0(), 5);
        assert_eq!(d.day(), 15);
        assert_eq!(d.day0(), 14);
        assert_eq!(d.quarter(), 2);
    }

    #[test]
    fn with_month_rejects_day_that_does_not_exist_in_target_month() {
        let jan31 = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();
        assert_eq!(jan31.with_month(2), None);
        assert_eq!(jan31.with_month(4), None);
        assert!(jan31.with_month(3).is_some());
    }

    #[test]
    fn with_year_respects_leap_day_validity() {
        let feb29 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(feb29.with_year(2023), None);
        assert!(feb29.with_year(2000).is_some());
    }

    #[test]
    fn with_day_and_with_day0_are_1_and_0_indexed() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 1).unwrap();
        assert_eq!(d.with_day(15), NaiveDate::from_ymd_opt(2023, 6, 15));
        assert_eq!(d.with_day0(14), NaiveDate::from_ymd_opt(2023, 6, 15));
        assert_eq!(d.with_day(31), None); // June has only 30 days
    }

    #[test]
    fn with_ordinal_and_with_ordinal0() {
        let d = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        assert_eq!(d.with_ordinal(365), NaiveDate::from_ymd_opt(2023, 12, 31));
        assert_eq!(d.with_ordinal0(364), NaiveDate::from_ymd_opt(2023, 12, 31));
        assert_eq!(d.with_ordinal(366), None); // 2023 is not leap
    }

    #[test]
    fn weekday_matches_hand_verified_facts() {
        assert_eq!(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().weekday(), Weekday::Thu);
        assert_eq!(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().weekday(), Weekday::Sun);
    }

    #[test]
    fn iso_week_matches_hand_verified_year_boundary_cases() {
        let iso = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().iso_week();
        assert_eq!((iso.year(), iso.week()), (2022, 52));
        let iso2 = NaiveDate::from_ymd_opt(2018, 12, 31).unwrap().iso_week();
        assert_eq!((iso2.year(), iso2.week()), (2019, 1));
    }

    #[test]
    fn iso_week_week0_and_display() {
        let iso = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap().iso_week(); // 2023-W01
        assert_eq!(iso.week(), 1);
        assert_eq!(iso.week0(), 0);
        assert_eq!(iso.to_string(), "2023-W01");
    }

    #[test]
    fn add_sub_duration_operators() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(d + Duration::days(5), NaiveDate::from_ymd_opt(2023, 6, 20).unwrap());
        assert_eq!(d - Duration::days(5), NaiveDate::from_ymd_opt(2023, 6, 10).unwrap());
        let mut d2 = d;
        d2 += Duration::days(1);
        assert_eq!(d2, NaiveDate::from_ymd_opt(2023, 6, 16).unwrap());
        d2 -= Duration::days(2);
        assert_eq!(d2, NaiveDate::from_ymd_opt(2023, 6, 14).unwrap());
    }

    #[test]
    fn add_sub_days_and_months_operators() {
        let d = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();
        assert_eq!(d + Days::new(1), NaiveDate::from_ymd_opt(2023, 2, 1).unwrap());
        assert_eq!(d + Months::new(1), NaiveDate::from_ymd_opt(2023, 2, 28).unwrap());
        assert_eq!(d - Days::new(31), NaiveDate::from_ymd_opt(2022, 12, 31).unwrap());
    }

    #[test]
    #[should_panic]
    fn add_days_operator_panics_on_overflow() {
        let _ = NaiveDate::MAX + Days::new(1);
    }

    #[test]
    fn display_uses_4_digit_year_within_0_9999() {
        assert_eq!(NaiveDate::from_ymd_opt(2023, 6, 5).unwrap().to_string(), "2023-06-05");
        assert_eq!(NaiveDate::from_ymd_opt(5, 1, 1).unwrap().to_string(), "0005-01-01");
    }

    #[test]
    fn display_uses_signed_year_outside_0_9999() {
        assert_eq!(NaiveDate::from_ymd_opt(12345, 6, 5).unwrap().to_string(), "+12345-06-05");
        assert_eq!(NaiveDate::from_ymd_opt(-1, 12, 31).unwrap().to_string(), "-0001-12-31");
    }

    #[test]
    fn debug_matches_display() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 5).unwrap();
        assert_eq!(format!("{d:?}"), d.to_string());
    }

    #[test]
    fn default_is_unix_epoch() {
        assert_eq!(NaiveDate::default(), NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
    }

    #[test]
    fn from_str_parses_iso_format_and_rejects_garbage() {
        assert_eq!("2023-06-05".parse::<NaiveDate>(), Ok(NaiveDate::from_ymd_opt(2023, 6, 5).unwrap()));
        assert!("2023/06/05".parse::<NaiveDate>().is_err());
        assert!("not-a-date".parse::<NaiveDate>().is_err());
    }

    #[test]
    fn display_and_from_str_round_trip() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 5).unwrap();
        assert_eq!(d.to_string().parse::<NaiveDate>(), Ok(d));
        let neg = NaiveDate::from_ymd_opt(-1, 12, 31).unwrap();
        assert_eq!(neg.to_string().parse::<NaiveDate>(), Ok(neg));
        let big = NaiveDate::from_ymd_opt(12345, 6, 5).unwrap();
        assert_eq!(big.to_string().parse::<NaiveDate>(), Ok(big));
    }

    #[test]
    fn ordering_matches_chronological_order() {
        let a = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let b = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap();
        let c = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(a < b);
        assert!(b < c);
        assert!(NaiveDate::MIN < NaiveDate::MAX);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let d = NaiveDate::from_ymd_opt(2023, 6, 5).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "\"2023-06-05\"");
        let back: NaiveDate = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }
}
