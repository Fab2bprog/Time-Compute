//! Date and time formatting routines.

use core::borrow::Borrow;
use core::fmt::{self, Display, Write};

use crate::offset::Offset;
use crate::{Datelike, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};

use super::locales::{self, Locale};
use super::{Colons, OffsetFormat, OffsetPrecision, Pad};
use super::{Fixed, InternalFixed, InternalInternal, Item, Numeric};

/// A *temporary* object which can be used as an argument to `format!` or
/// others. This is normally constructed via `format` methods of each date
/// and time type.
#[derive(Debug)]
pub struct DelayedFormat<I> {
    /// The date view, if any.
    date: Option<NaiveDate>,
    /// The time view, if any.
    time: Option<NaiveTime>,
    /// The name and local-to-UTC difference for the offset (timezone), if any.
    off: Option<(String, FixedOffset)>,
    /// An iterator returning formatting items.
    items: I,
    /// Locale used for text. A zero-sized placeholder unless the
    /// `unstable-locales` feature is enabled.
    locale: Locale,
}

impl<'a, I: Iterator<Item = B> + Clone, B: Borrow<Item<'a>>> DelayedFormat<I> {
    /// Makes a new `DelayedFormat` value out of local date and time.
    #[must_use]
    pub fn new(date: Option<NaiveDate>, time: Option<NaiveTime>, items: I) -> DelayedFormat<I> {
        DelayedFormat { date, time, off: None, items, locale: locales::default_locale() }
    }

    /// Makes a new `DelayedFormat` value out of local date and time and UTC offset.
    #[must_use]
    pub fn new_with_offset<Off>(
        date: Option<NaiveDate>,
        time: Option<NaiveTime>,
        offset: &Off,
        items: I,
    ) -> DelayedFormat<I>
    where
        Off: Offset + Display,
    {
        let name_and_diff = (offset.to_string(), offset.fix());
        DelayedFormat { date, time, off: Some(name_and_diff), items, locale: locales::default_locale() }
    }

    /// Makes a new `DelayedFormat` value out of local date and time and locale.
    #[cfg(feature = "unstable-locales")]
    #[must_use]
    pub fn new_with_locale(
        date: Option<NaiveDate>,
        time: Option<NaiveTime>,
        items: I,
        locale: Locale,
    ) -> DelayedFormat<I> {
        DelayedFormat { date, time, off: None, items, locale }
    }

    /// Makes a new `DelayedFormat` value out of local date and time, UTC offset and locale.
    #[cfg(feature = "unstable-locales")]
    #[must_use]
    pub fn new_with_offset_and_locale<Off>(
        date: Option<NaiveDate>,
        time: Option<NaiveTime>,
        offset: &Off,
        items: I,
        locale: Locale,
    ) -> DelayedFormat<I>
    where
        Off: Offset + Display,
    {
        let name_and_diff = (offset.to_string(), offset.fix());
        DelayedFormat { date, time, off: Some(name_and_diff), items, locale }
    }

    /// Formats `DelayedFormat` into a `core::fmt::Write` instance.
    ///
    /// # Errors
    /// Returns a `core::fmt::Error` if formatting into the `core::fmt::Write`
    /// instance fails.
    pub fn write_to(&self, w: &mut (impl Write + ?Sized)) -> fmt::Result {
        for item in self.items.clone() {
            match *item.borrow() {
                Item::Literal(s) | Item::Space(s) => w.write_str(s),
                Item::OwnedLiteral(ref s) | Item::OwnedSpace(ref s) => w.write_str(s),
                Item::Numeric(ref spec, pad) => self.format_numeric(w, spec, pad),
                Item::Fixed(ref spec) => self.format_fixed(w, spec),
                Item::Error => Err(fmt::Error),
            }?;
        }
        Ok(())
    }

    fn format_numeric(
        &self,
        w: &mut (impl Write + ?Sized),
        spec: &Numeric,
        pad: Pad,
    ) -> fmt::Result {
        use self::Numeric::*;

        fn write_one(w: &mut (impl Write + ?Sized), v: u8) -> fmt::Result {
            w.write_char((b'0' + v) as char)
        }

        fn write_two(w: &mut (impl Write + ?Sized), v: u8, pad: Pad) -> fmt::Result {
            let ones = b'0' + v % 10;
            match (v / 10, pad) {
                (0, Pad::None) => {}
                (0, Pad::Space) => w.write_char(' ')?,
                (tens, _) => w.write_char((b'0' + tens) as char)?,
            }
            w.write_char(ones as char)
        }

        #[inline]
        fn write_year(w: &mut (impl Write + ?Sized), year: i32, pad: Pad) -> fmt::Result {
            if (1000..=9999).contains(&year) {
                // fast path
                write_hundreds(w, (year / 100) as u8)?;
                write_hundreds(w, (year % 100) as u8)
            } else {
                write_n(w, 4, year as i64, pad, !(0..10_000).contains(&year))
            }
        }

        fn write_n(
            w: &mut (impl Write + ?Sized),
            n: usize,
            v: i64,
            pad: Pad,
            always_sign: bool,
        ) -> fmt::Result {
            if always_sign {
                match pad {
                    Pad::None => write!(w, "{v:+}"),
                    Pad::Zero => write!(w, "{:+01$}", v, n + 1),
                    Pad::Space => write!(w, "{:+1$}", v, n + 1),
                }
            } else {
                match pad {
                    Pad::None => write!(w, "{v}"),
                    Pad::Zero => write!(w, "{v:0n$}"),
                    Pad::Space => write!(w, "{v:n$}"),
                }
            }
        }

        match (spec, self.date, self.time) {
            (Year, Some(d), _) => write_year(w, d.year(), pad),
            (YearDiv100, Some(d), _) => write_two(w, d.year().div_euclid(100) as u8, pad),
            (YearMod100, Some(d), _) => write_two(w, d.year().rem_euclid(100) as u8, pad),
            (IsoYear, Some(d), _) => write_year(w, d.iso_week().year(), pad),
            (IsoYearDiv100, Some(d), _) => {
                write_two(w, d.iso_week().year().div_euclid(100) as u8, pad)
            }
            (IsoYearMod100, Some(d), _) => {
                write_two(w, d.iso_week().year().rem_euclid(100) as u8, pad)
            }
            (Quarter, Some(d), _) => write_one(w, d.quarter() as u8),
            (Month, Some(d), _) => write_two(w, d.month() as u8, pad),
            (Day, Some(d), _) => write_two(w, d.day() as u8, pad),
            (WeekFromSun, Some(d), _) => write_two(w, d.weeks_from(Weekday::Sun) as u8, pad),
            (WeekFromMon, Some(d), _) => write_two(w, d.weeks_from(Weekday::Mon) as u8, pad),
            (IsoWeek, Some(d), _) => write_two(w, d.iso_week().week() as u8, pad),
            (NumDaysFromSun, Some(d), _) => write_one(w, d.weekday().num_days_from_sunday() as u8),
            (WeekdayFromMon, Some(d), _) => write_one(w, d.weekday().number_from_monday() as u8),
            (Ordinal, Some(d), _) => write_n(w, 3, d.ordinal() as i64, pad, false),
            (Hour, _, Some(t)) => write_two(w, t.hour() as u8, pad),
            (Hour12, _, Some(t)) => write_two(w, t.hour12().1 as u8, pad),
            (Minute, _, Some(t)) => write_two(w, t.minute() as u8, pad),
            (Second, _, Some(t)) => {
                write_two(w, (t.second() + t.nanosecond() / 1_000_000_000) as u8, pad)
            }
            (Nanosecond, _, Some(t)) => {
                write_n(w, 9, (t.nanosecond() % 1_000_000_000) as i64, pad, false)
            }
            (Timestamp, Some(d), Some(t)) => {
                let offset = self.off.as_ref().map(|(_, o)| i64::from(o.local_minus_utc()));
                let timestamp = d.and_time(t).and_utc().timestamp() - offset.unwrap_or(0);
                write_n(w, 9, timestamp, pad, false)
            }
            (Internal(_), _, _) => Ok(()), // for future expansion
            _ => Err(fmt::Error),          // insufficient arguments for given format
        }
    }

    fn format_fixed(&self, w: &mut (impl Write + ?Sized), spec: &Fixed) -> fmt::Result {
        use Fixed::*;
        use InternalInternal::*;

        match (spec, self.date, self.time, self.off.as_ref()) {
            (ShortMonthName, Some(d), _, _) => {
                w.write_str(locales::short_months(self.locale)[d.month0() as usize])
            }
            (LongMonthName, Some(d), _, _) => {
                w.write_str(locales::long_months(self.locale)[d.month0() as usize])
            }
            (ShortWeekdayName, Some(d), _, _) => w.write_str(
                locales::short_weekdays(self.locale)[d.weekday().num_days_from_sunday() as usize],
            ),
            (LongWeekdayName, Some(d), _, _) => w.write_str(
                locales::long_weekdays(self.locale)[d.weekday().num_days_from_sunday() as usize],
            ),
            (LowerAmPm, _, Some(t), _) => {
                let am_pm = locales::am_pm(self.locale);
                let ampm = if t.hour12().0 { am_pm[1] } else { am_pm[0] };
                for c in ampm.chars().flat_map(|c| c.to_lowercase()) {
                    w.write_char(c)?
                }
                Ok(())
            }
            (UpperAmPm, _, Some(t), _) => {
                let am_pm = locales::am_pm(self.locale);
                let ampm = if t.hour12().0 { am_pm[1] } else { am_pm[0] };
                w.write_str(ampm)
            }
            (Nanosecond, _, Some(t), _) => {
                let nano = t.nanosecond() % 1_000_000_000;
                if nano == 0 {
                    Ok(())
                } else {
                    w.write_str(locales::decimal_point(self.locale))?;
                    if nano.is_multiple_of(1_000_000) {
                        write!(w, "{:03}", nano / 1_000_000)
                    } else if nano.is_multiple_of(1_000) {
                        write!(w, "{:06}", nano / 1_000)
                    } else {
                        write!(w, "{nano:09}")
                    }
                }
            }
            (Nanosecond3, _, Some(t), _) => {
                w.write_str(locales::decimal_point(self.locale))?;
                write!(w, "{:03}", t.nanosecond() / 1_000_000 % 1000)
            }
            (Nanosecond6, _, Some(t), _) => {
                w.write_str(locales::decimal_point(self.locale))?;
                write!(w, "{:06}", t.nanosecond() / 1_000 % 1_000_000)
            }
            (Nanosecond9, _, Some(t), _) => {
                w.write_str(locales::decimal_point(self.locale))?;
                write!(w, "{:09}", t.nanosecond() % 1_000_000_000)
            }
            (Internal(InternalFixed { val: Nanosecond3NoDot }), _, Some(t), _) => {
                write!(w, "{:03}", t.nanosecond() / 1_000_000 % 1_000)
            }
            (Internal(InternalFixed { val: Nanosecond6NoDot }), _, Some(t), _) => {
                write!(w, "{:06}", t.nanosecond() / 1_000 % 1_000_000)
            }
            (Internal(InternalFixed { val: Nanosecond9NoDot }), _, Some(t), _) => {
                write!(w, "{:09}", t.nanosecond() % 1_000_000_000)
            }
            (Internal(InternalFixed { val: TimezoneOffsetPermissive }), _, _, _) => {
                panic!("TimezoneOffsetPermissive is not supported for printing")
            }
            // This crate does not resolve time zone name abbreviations (see
            // the `%Z` note in the `strftime` module docs): print the offset
            // instead, exactly like `%:z`.
            (TimezoneName, _, _, Some((_, off))) => {
                let offset_format = OffsetFormat {
                    precision: OffsetPrecision::Minutes,
                    colons: Colons::Colon,
                    allow_zulu: false,
                    padding: Pad::Zero,
                };
                offset_format.format(w, *off)
            }
            (TimezoneOffset | TimezoneOffsetZ, _, _, Some((_, off))) => {
                let offset_format = OffsetFormat {
                    precision: OffsetPrecision::Minutes,
                    colons: Colons::Maybe,
                    allow_zulu: *spec == TimezoneOffsetZ,
                    padding: Pad::Zero,
                };
                offset_format.format(w, *off)
            }
            (TimezoneOffsetColon | TimezoneOffsetColonZ, _, _, Some((_, off))) => {
                let offset_format = OffsetFormat {
                    precision: OffsetPrecision::Minutes,
                    colons: Colons::Colon,
                    allow_zulu: *spec == TimezoneOffsetColonZ,
                    padding: Pad::Zero,
                };
                offset_format.format(w, *off)
            }
            (TimezoneOffsetDoubleColon, _, _, Some((_, off))) => {
                let offset_format = OffsetFormat {
                    precision: OffsetPrecision::Seconds,
                    colons: Colons::Colon,
                    allow_zulu: false,
                    padding: Pad::Zero,
                };
                offset_format.format(w, *off)
            }
            (TimezoneOffsetTripleColon, _, _, Some((_, off))) => {
                let offset_format = OffsetFormat {
                    precision: OffsetPrecision::Hours,
                    colons: Colons::None,
                    allow_zulu: false,
                    padding: Pad::Zero,
                };
                offset_format.format(w, *off)
            }
            (RFC2822, Some(d), Some(t), Some((_, off))) => {
                write_rfc2822(w, NaiveDateTime::new(d, t), *off)
            }
            (RFC3339, Some(d), Some(t), Some((_, off))) => {
                write_rfc3339(w, NaiveDateTime::new(d, t), *off, SecondsFormat::AutoSi, false)
            }
            _ => Err(fmt::Error), // insufficient arguments for given format
        }
    }
}

impl<'a, I: Iterator<Item = B> + Clone, B: Borrow<Item<'a>>> Display for DelayedFormat<I> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::new();
        self.write_to(&mut result)?;
        f.pad(&result)
    }
}

impl OffsetFormat {
    /// Writes an offset from UTC with the format defined by `self`.
    fn format(&self, w: &mut (impl Write + ?Sized), off: FixedOffset) -> fmt::Result {
        let off = off.local_minus_utc();
        if self.allow_zulu && off == 0 {
            w.write_char('Z')?;
            return Ok(());
        }
        let (sign, off) = if off < 0 { ('-', -off) } else { ('+', off) };

        let hours;
        let mut mins = 0;
        let mut secs = 0;
        let precision = match self.precision {
            OffsetPrecision::Hours => {
                // Minutes and seconds are simply truncated
                hours = (off / 3600) as u8;
                OffsetPrecision::Hours
            }
            OffsetPrecision::Minutes | OffsetPrecision::OptionalMinutes => {
                // Round seconds to the nearest minute, but never roll over to 24:00. A
                // `FixedOffset` may be as large as 23:59:59, and rounding that up would emit
                // an invalid "+24:00" that no offset parser (including ours) accepts.
                let minutes = ((off + 30) / 60).min(24 * 60 - 1);
                mins = (minutes % 60) as u8;
                hours = (minutes / 60) as u8;
                if self.precision == OffsetPrecision::OptionalMinutes && mins == 0 {
                    OffsetPrecision::Hours
                } else {
                    OffsetPrecision::Minutes
                }
            }
            OffsetPrecision::Seconds
            | OffsetPrecision::OptionalSeconds
            | OffsetPrecision::OptionalMinutesAndSeconds => {
                let minutes = off / 60;
                secs = (off % 60) as u8;
                mins = (minutes % 60) as u8;
                hours = (minutes / 60) as u8;
                if self.precision != OffsetPrecision::Seconds && secs == 0 {
                    if self.precision == OffsetPrecision::OptionalMinutesAndSeconds && mins == 0 {
                        OffsetPrecision::Hours
                    } else {
                        OffsetPrecision::Minutes
                    }
                } else {
                    OffsetPrecision::Seconds
                }
            }
        };
        let colons = self.colons == Colons::Colon;

        if hours < 10 {
            if self.padding == Pad::Space {
                w.write_char(' ')?;
            }
            w.write_char(sign)?;
            if self.padding == Pad::Zero {
                w.write_char('0')?;
            }
            w.write_char((b'0' + hours) as char)?;
        } else {
            w.write_char(sign)?;
            write_hundreds(w, hours)?;
        }
        if let OffsetPrecision::Minutes | OffsetPrecision::Seconds = precision {
            if colons {
                w.write_char(':')?;
            }
            write_hundreds(w, mins)?;
        }
        if let OffsetPrecision::Seconds = precision {
            if colons {
                w.write_char(':')?;
            }
            write_hundreds(w, secs)?;
        }
        Ok(())
    }
}

/// Specific formatting options for seconds. This may be extended in the
/// future, so exhaustive matching in external code is not recommended.
///
/// See [`DateTime::to_rfc3339_opts`](crate::DateTime::to_rfc3339_opts) for usage.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SecondsFormat {
    /// Format whole seconds only, with no decimal point nor subseconds.
    Secs,
    /// Use fixed 3 subsecond digits. This corresponds to [`Fixed::Nanosecond3`].
    Millis,
    /// Use fixed 6 subsecond digits. This corresponds to [`Fixed::Nanosecond6`].
    Micros,
    /// Use fixed 9 subsecond digits. This corresponds to [`Fixed::Nanosecond9`].
    Nanos,
    /// Automatically select one of `Secs`, `Millis`, `Micros`, or `Nanos` to
    /// display all available non-zero sub-second digits. This corresponds to
    /// [`Fixed::Nanosecond`].
    AutoSi,
}

/// Writes the date, time and offset to the string. Same as `%Y-%m-%dT%H:%M:%S%.f%:z`.
#[inline]
pub(crate) fn write_rfc3339(
    w: &mut (impl Write + ?Sized),
    dt: NaiveDateTime,
    off: FixedOffset,
    secform: SecondsFormat,
    use_z: bool,
) -> fmt::Result {
    let year = dt.date().year();
    if (0..=9999).contains(&year) {
        write_hundreds(w, (year / 100) as u8)?;
        write_hundreds(w, (year % 100) as u8)?;
    } else {
        // ISO 8601 requires the explicit sign for out-of-range years
        write!(w, "{year:+05}")?;
    }
    w.write_char('-')?;
    write_hundreds(w, dt.date().month() as u8)?;
    w.write_char('-')?;
    write_hundreds(w, dt.date().day() as u8)?;

    w.write_char('T')?;

    let (hour, min, mut sec) = dt.time().hms();
    let mut nano = dt.nanosecond();
    if nano >= 1_000_000_000 {
        sec += 1;
        nano -= 1_000_000_000;
    }
    write_hundreds(w, hour as u8)?;
    w.write_char(':')?;
    write_hundreds(w, min as u8)?;
    w.write_char(':')?;
    let sec = sec;
    write_hundreds(w, sec as u8)?;

    match secform {
        SecondsFormat::Secs => {}
        SecondsFormat::Millis => write!(w, ".{:03}", nano / 1_000_000)?,
        SecondsFormat::Micros => write!(w, ".{:06}", nano / 1000)?,
        SecondsFormat::Nanos => write!(w, ".{nano:09}")?,
        SecondsFormat::AutoSi => {
            if nano == 0 {
            } else if nano.is_multiple_of(1_000_000) {
                write!(w, ".{:03}", nano / 1_000_000)?
            } else if nano.is_multiple_of(1_000) {
                write!(w, ".{:06}", nano / 1_000)?
            } else {
                write!(w, ".{nano:09}")?
            }
        }
    };

    OffsetFormat {
        precision: OffsetPrecision::Minutes,
        colons: Colons::Colon,
        allow_zulu: use_z,
        padding: Pad::Zero,
    }
    .format(w, off)
}

/// Writes datetimes like `Tue, 1 Jul 2003 10:52:37 +0200`, same as `%a, %d %b %Y %H:%M:%S %z`.
pub(crate) fn write_rfc2822(
    w: &mut (impl Write + ?Sized),
    dt: NaiveDateTime,
    off: FixedOffset,
) -> fmt::Result {
    let year = dt.year();
    // RFC 2822 is only defined on years 0 through 9999
    if !(0..=9999).contains(&year) {
        return Err(fmt::Error);
    }

    // RFC 2822 mandates English weekday/month abbreviations regardless of
    // any locale the caller might otherwise be using.
    let english = locales::default_locale();

    w.write_str(locales::short_weekdays(english)[dt.weekday().num_days_from_sunday() as usize])?;
    w.write_str(", ")?;
    let day = dt.day();
    if day < 10 {
        w.write_char((b'0' + day as u8) as char)?;
    } else {
        write_hundreds(w, day as u8)?;
    }
    w.write_char(' ')?;
    w.write_str(locales::short_months(english)[dt.month0() as usize])?;
    w.write_char(' ')?;
    write_hundreds(w, (year / 100) as u8)?;
    write_hundreds(w, (year % 100) as u8)?;
    w.write_char(' ')?;

    let (hour, min, sec) = dt.time().hms();
    write_hundreds(w, hour as u8)?;
    w.write_char(':')?;
    write_hundreds(w, min as u8)?;
    w.write_char(':')?;
    let sec = sec + dt.nanosecond() / 1_000_000_000;
    write_hundreds(w, sec as u8)?;
    w.write_char(' ')?;
    OffsetFormat {
        precision: OffsetPrecision::Minutes,
        colons: Colons::None,
        allow_zulu: false,
        padding: Pad::Zero,
    }
    .format(w, off)
}

/// Equivalent to `{:02}` formatting for n < 100.
pub(crate) fn write_hundreds(w: &mut (impl Write + ?Sized), n: u8) -> fmt::Result {
    if n >= 100 {
        return Err(fmt::Error);
    }

    let tens = b'0' + n / 10;
    let ones = b'0' + n % 10;
    w.write_char(tens as char)?;
    w.write_char(ones as char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_hundreds_formats_two_digits() {
        let mut s = String::new();
        write_hundreds(&mut s, 7).unwrap();
        assert_eq!(s, "07");
        let mut s2 = String::new();
        write_hundreds(&mut s2, 42).unwrap();
        assert_eq!(s2, "42");
    }

    #[test]
    fn write_hundreds_rejects_values_over_99() {
        let mut s = String::new();
        assert!(write_hundreds(&mut s, 100).is_err());
    }

    #[test]
    fn offset_format_hours_precision_truncates_minutes_and_seconds() {
        let off = FixedOffset::east_opt(5 * 3600 + 45 * 60).unwrap(); // +05:45
        let fmt = OffsetFormat {
            precision: OffsetPrecision::Hours,
            colons: Colons::Colon,
            allow_zulu: false,
            padding: Pad::Zero,
        };
        let mut s = String::new();
        fmt.format(&mut s, off).unwrap();
        assert_eq!(s, "+05");
    }

    #[test]
    fn offset_format_minutes_precision_rounds_seconds_to_nearest_minute() {
        let fmt = OffsetFormat {
            precision: OffsetPrecision::Minutes,
            colons: Colons::Colon,
            allow_zulu: false,
            padding: Pad::Zero,
        };
        let off_down = FixedOffset::east_opt(3600 + 29).unwrap(); // +01:00:29 -> rounds down
        let mut s = String::new();
        fmt.format(&mut s, off_down).unwrap();
        assert_eq!(s, "+01:00");

        let off_up = FixedOffset::east_opt(3600 + 31).unwrap(); // +01:00:31 -> rounds up
        let mut s2 = String::new();
        fmt.format(&mut s2, off_up).unwrap();
        assert_eq!(s2, "+01:01");
    }

    #[test]
    fn offset_format_minutes_precision_never_rounds_up_to_24_00() {
        // `FixedOffset` can be as large as +23:59:59; rounding that to the
        // nearest minute must clamp at +23:59, never emit the invalid +24:00.
        let off = FixedOffset::east_opt(86_399).unwrap();
        let fmt = OffsetFormat {
            precision: OffsetPrecision::Minutes,
            colons: Colons::Colon,
            allow_zulu: false,
            padding: Pad::Zero,
        };
        let mut s = String::new();
        fmt.format(&mut s, off).unwrap();
        assert_eq!(s, "+23:59");
    }

    #[test]
    fn offset_format_seconds_precision_included_when_nonzero() {
        let off = FixedOffset::east_opt(3600 + 30 * 60 + 15).unwrap(); // +01:30:15
        let fmt = OffsetFormat {
            precision: OffsetPrecision::Seconds,
            colons: Colons::Colon,
            allow_zulu: false,
            padding: Pad::Zero,
        };
        let mut s = String::new();
        fmt.format(&mut s, off).unwrap();
        assert_eq!(s, "+01:30:15");
    }

    #[test]
    fn offset_format_optional_minutes_and_seconds_collapses_when_zero() {
        let off = FixedOffset::east_opt(5 * 3600).unwrap();
        let fmt = OffsetFormat {
            precision: OffsetPrecision::OptionalMinutesAndSeconds,
            colons: Colons::Colon,
            allow_zulu: false,
            padding: Pad::Zero,
        };
        let mut s = String::new();
        fmt.format(&mut s, off).unwrap();
        assert_eq!(s, "+05");
    }

    #[test]
    fn offset_format_zero_offset_with_zulu_allowed_prints_z() {
        let off = FixedOffset::east_opt(0).unwrap();
        let fmt = OffsetFormat {
            precision: OffsetPrecision::Minutes,
            colons: Colons::Colon,
            allow_zulu: true,
            padding: Pad::Zero,
        };
        let mut s = String::new();
        fmt.format(&mut s, off).unwrap();
        assert_eq!(s, "Z");
    }

    #[test]
    fn offset_format_padding_variants_for_single_digit_hours() {
        let off = FixedOffset::east_opt(5 * 3600).unwrap(); // +05:00
        let variants = [(Pad::Zero, "+05:00"), (Pad::None, "+5:00"), (Pad::Space, " +5:00")];
        for (padding, expected) in variants {
            let fmt = OffsetFormat {
                precision: OffsetPrecision::Minutes,
                colons: Colons::Colon,
                allow_zulu: false,
                padding,
            };
            let mut s = String::new();
            fmt.format(&mut s, off).unwrap();
            assert_eq!(s, expected);
        }
    }

    #[test]
    fn write_rfc3339_basic() {
        let dt = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap().and_hms_opt(9, 30, 0).unwrap();
        let off = FixedOffset::east_opt(0).unwrap();
        let mut s = String::new();
        write_rfc3339(&mut s, dt, off, SecondsFormat::Secs, false).unwrap();
        assert_eq!(s, "2023-06-15T09:30:00+00:00");
        let mut s2 = String::new();
        write_rfc3339(&mut s2, dt, off, SecondsFormat::Secs, true).unwrap();
        assert_eq!(s2, "2023-06-15T09:30:00Z");
    }

    #[test]
    fn write_rfc3339_negative_year_uses_explicit_sign() {
        let dt = NaiveDate::from_ymd_opt(-1, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let off = FixedOffset::east_opt(0).unwrap();
        let mut s = String::new();
        write_rfc3339(&mut s, dt, off, SecondsFormat::Secs, true).unwrap();
        assert_eq!(s, "-0001-01-01T00:00:00Z");
    }

    #[test]
    fn write_rfc2822_rejects_years_outside_0_9999() {
        let dt = NaiveDate::from_ymd_opt(-1, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let off = FixedOffset::east_opt(0).unwrap();
        let mut s = String::new();
        assert!(write_rfc2822(&mut s, dt, off).is_err());
    }

    #[test]
    fn delayed_format_numeric_padding_variants() {
        let date = NaiveDate::from_ymd_opt(2023, 3, 5).unwrap();
        let zero = [Item::Numeric(Numeric::Day, Pad::Zero)];
        assert_eq!(DelayedFormat::new(Some(date), None, zero.iter()).to_string(), "05");
        let none = [Item::Numeric(Numeric::Day, Pad::None)];
        assert_eq!(DelayedFormat::new(Some(date), None, none.iter()).to_string(), "5");
        let space = [Item::Numeric(Numeric::Day, Pad::Space)];
        assert_eq!(DelayedFormat::new(Some(date), None, space.iter()).to_string(), " 5");
    }

    #[test]
    fn delayed_format_errors_when_required_component_is_missing() {
        let items = [Item::Numeric(Numeric::Hour, Pad::Zero)];
        let df = DelayedFormat::new(None::<NaiveDate>, None::<NaiveTime>, items.iter());
        let mut out = String::new();
        assert!(df.write_to(&mut out).is_err());
    }

    #[test]
    fn delayed_format_am_pm_items() {
        let am_time = NaiveTime::from_hms_opt(9, 5, 0).unwrap();
        let lower = [Item::Fixed(Fixed::LowerAmPm)];
        assert_eq!(DelayedFormat::new(None, Some(am_time), lower.iter()).to_string(), "am");

        let pm_time = NaiveTime::from_hms_opt(21, 5, 0).unwrap();
        let upper = [Item::Fixed(Fixed::UpperAmPm)];
        assert_eq!(DelayedFormat::new(None, Some(pm_time), upper.iter()).to_string(), "PM");
    }

    #[test]
    fn delayed_format_nanosecond_fixed_precision_variants() {
        let time = NaiveTime::from_hms_nano_opt(0, 0, 0, 123_456_789).unwrap();
        assert_eq!(
            DelayedFormat::new(None, Some(time), [Item::Fixed(Fixed::Nanosecond3)].iter()).to_string(),
            ".123"
        );
        assert_eq!(
            DelayedFormat::new(None, Some(time), [Item::Fixed(Fixed::Nanosecond6)].iter()).to_string(),
            ".123456"
        );
        assert_eq!(
            DelayedFormat::new(None, Some(time), [Item::Fixed(Fixed::Nanosecond9)].iter()).to_string(),
            ".123456789"
        );
    }

    #[test]
    fn delayed_format_nanosecond_auto_is_empty_when_zero() {
        let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let items = [Item::Fixed(Fixed::Nanosecond)];
        assert_eq!(DelayedFormat::new(None, Some(time), items.iter()).to_string(), "");
    }

    #[test]
    fn delayed_format_nanosecond_auto_selects_shortest_precision() {
        let millis_only = NaiveTime::from_hms_nano_opt(0, 0, 0, 500_000_000).unwrap();
        assert_eq!(
            DelayedFormat::new(None, Some(millis_only), [Item::Fixed(Fixed::Nanosecond)].iter())
                .to_string(),
            ".500"
        );
        let micros_only = NaiveTime::from_hms_nano_opt(0, 0, 0, 500_500_000).unwrap();
        assert_eq!(
            DelayedFormat::new(None, Some(micros_only), [Item::Fixed(Fixed::Nanosecond)].iter())
                .to_string(),
            ".500500"
        );
        let full_nanos = NaiveTime::from_hms_nano_opt(0, 0, 0, 500_500_500).unwrap();
        assert_eq!(
            DelayedFormat::new(None, Some(full_nanos), [Item::Fixed(Fixed::Nanosecond)].iter())
                .to_string(),
            ".500500500"
        );
    }

    #[test]
    fn delayed_format_timezone_offset_triple_colon_is_hours_only() {
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let off = FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
        let items = [Item::Fixed(Fixed::TimezoneOffsetTripleColon)];
        let df = DelayedFormat::new_with_offset(Some(date), Some(time), &off, items.iter());
        assert_eq!(df.to_string(), "+05");
    }

    #[test]
    fn delayed_format_rfc2822_and_rfc3339_fixed_items() {
        let date = NaiveDate::from_ymd_opt(2003, 7, 1).unwrap();
        let time = NaiveTime::from_hms_opt(10, 52, 37).unwrap();
        let off = FixedOffset::east_opt(2 * 3600).unwrap();

        let rfc2822 = [Item::Fixed(Fixed::RFC2822)];
        let df = DelayedFormat::new_with_offset(Some(date), Some(time), &off, rfc2822.iter());
        assert_eq!(df.to_string(), "Tue, 1 Jul 2003 10:52:37 +0200");

        let rfc3339 = [Item::Fixed(Fixed::RFC3339)];
        let df2 = DelayedFormat::new_with_offset(Some(date), Some(time), &off, rfc3339.iter());
        assert_eq!(df2.to_string(), "2003-07-01T10:52:37+02:00");
    }

    #[test]
    fn delayed_format_quarter_and_weekday_numerics() {
        // 2023-06-15 is a Thursday in Q2 (hand-verified in `calendar.rs`'s
        // own weekday tests via the same epoch-anchored technique).
        let date = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        assert_eq!(
            DelayedFormat::new(Some(date), None, [Item::Numeric(Numeric::Quarter, Pad::None)].iter())
                .to_string(),
            "2"
        );
        assert_eq!(
            DelayedFormat::new(
                Some(date),
                None,
                [Item::Numeric(Numeric::NumDaysFromSun, Pad::None)].iter()
            )
            .to_string(),
            "4"
        );
        assert_eq!(
            DelayedFormat::new(
                Some(date),
                None,
                [Item::Numeric(Numeric::WeekdayFromMon, Pad::None)].iter()
            )
            .to_string(),
            "4"
        );
    }

    #[test]
    fn delayed_format_year_normal_range_has_no_explicit_sign() {
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let items = [Item::Numeric(Numeric::Year, Pad::Zero)];
        assert_eq!(DelayedFormat::new(Some(date), None, items.iter()).to_string(), "2023");
    }

    #[test]
    fn delayed_format_year_below_4_digits_has_no_sign_but_is_zero_padded() {
        // Year 50 is outside the 1000..=9999 fast path but still inside
        // 0..10_000, so the slow path applies without a forced sign.
        let date = NaiveDate::from_ymd_opt(50, 1, 1).unwrap();
        let items = [Item::Numeric(Numeric::Year, Pad::Zero)];
        assert_eq!(DelayedFormat::new(Some(date), None, items.iter()).to_string(), "0050");
    }

    #[test]
    fn delayed_format_year_outside_0_10000_uses_explicit_sign() {
        let date = NaiveDate::from_ymd_opt(-5, 1, 1).unwrap();
        let items = [Item::Numeric(Numeric::Year, Pad::Zero)];
        assert_eq!(DelayedFormat::new(Some(date), None, items.iter()).to_string(), "-0005");
    }

    #[test]
    fn delayed_format_timestamp_uses_utc_when_no_offset() {
        let date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let time = NaiveTime::from_hms_opt(0, 0, 1).unwrap();
        let items = [Item::Numeric(Numeric::Timestamp, Pad::None)];
        let df = DelayedFormat::new(Some(date), Some(time), items.iter());
        assert_eq!(df.to_string(), "1");
    }

    #[test]
    fn delayed_format_timestamp_subtracts_the_offset() {
        let date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let time = NaiveTime::from_hms_opt(1, 0, 0).unwrap(); // local wall-clock reading
        let off = FixedOffset::east_opt(3600).unwrap(); // +01:00
        let items = [Item::Numeric(Numeric::Timestamp, Pad::None)];
        let df = DelayedFormat::new_with_offset(Some(date), Some(time), &off, items.iter());
        // Local 01:00 at +01:00 is UTC 00:00 -> timestamp 0.
        assert_eq!(df.to_string(), "0");
    }
}
