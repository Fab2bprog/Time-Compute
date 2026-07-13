//! Date and time parsing routines.

use core::borrow::Borrow;
use core::str;

use super::scan;
use super::{ParseError, ParseResult};
use super::{Fixed, InternalFixed, InternalInternal, Item, Numeric, Pad, Parsed};
use super::{BAD_FORMAT, INVALID, OUT_OF_RANGE, TOO_LONG, TOO_SHORT};
use crate::{DateTime, FixedOffset, MappedLocalTime, NaiveDate, NaiveTime, Weekday};

fn set_weekday_with_num_days_from_sunday(p: &mut Parsed, v: i64) -> ParseResult<()> {
    p.set_weekday(match v {
        0 => Weekday::Sun,
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        6 => Weekday::Sat,
        _ => return Err(OUT_OF_RANGE),
    })
}

fn set_weekday_with_number_from_monday(p: &mut Parsed, v: i64) -> ParseResult<()> {
    p.set_weekday(match v {
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        6 => Weekday::Sat,
        7 => Weekday::Sun,
        _ => return Err(OUT_OF_RANGE),
    })
}

fn parse_rfc2822<'a>(parsed: &mut Parsed, mut s: &'a str) -> ParseResult<(&'a str, ())> {
    macro_rules! try_consume {
        ($e:expr) => {{
            let (s_, v) = $e?;
            s = s_;
            v
        }};
    }

    // an adapted RFC 2822 syntax from Section 3.3 and 4.3. Notes:
    // - quoted characters can be in any mixture of lower and upper case.
    // - we do not recognize folding white space (FWS) or comments (CFWS)
    //   exactly; we accept any sequence of Unicode whitespace, and for
    //   comments any text within parentheses while respecting escaped
    //   parentheses.
    // - a two-digit year < 50 is interpreted by adding 2000, a two-digit
    //   year >= 50 or three-digit year by adding 1900; four-or-more-digit
    //   years are never affected by this rule.
    // - a mismatching day-of-week is always an error.
    // - zones can range from -9959 to +9959, but `FixedOffset` does not
    //   support offsets larger than 24 hours; this is not problematic since
    //   we do not directly build a `DateTime` here (the offset can still be
    //   recovered from `Parsed`).

    s = s.trim_start();

    if let Ok((s_, weekday)) = scan::short_weekday(s) {
        if !s_.starts_with(',') {
            return Err(INVALID);
        }
        s = &s_[1..];
        parsed.set_weekday(weekday)?;
    }

    s = s.trim_start();
    parsed.set_day(try_consume!(scan::number(s, 1, 2)))?;
    s = scan::space(s)?; // mandatory
    parsed.set_month(1 + i64::from(try_consume!(scan::short_month0(s))))?;
    s = scan::space(s)?; // mandatory

    // distinguish two- and three-digit years from four-digit years
    let prevlen = s.len();
    let mut year = try_consume!(scan::number(s, 2, usize::MAX));
    let yearlen = prevlen - s.len();
    match (yearlen, year) {
        (2, 0..=49) => year += 2000,  //   47 -> 2047,   05 -> 2005
        (2, 50..=99) => year += 1900, //   79 -> 1979
        (3, _) => year += 1900,       //  112 -> 2012,  009 -> 1909
        (_, _) => {}                  // 1987 -> 1987, 0654 -> 0654
    }
    parsed.set_year(year)?;

    s = scan::space(s)?; // mandatory
    parsed.set_hour(try_consume!(scan::number(s, 2, 2)))?;
    s = scan::char(s.trim_start(), b':')?.trim_start(); // *S ":" *S
    parsed.set_minute(try_consume!(scan::number(s, 2, 2)))?;
    if let Ok(s_) = scan::char(s.trim_start(), b':') {
        // [ ":" *S 2DIGIT ]
        parsed.set_second(try_consume!(scan::number(s_, 2, 2)))?;
    }

    s = scan::space(s)?; // mandatory
    parsed.set_offset(i64::from(try_consume!(scan::timezone_offset_2822(s))))?;

    // optional comments
    while let Ok((s_out, ())) = scan::comment_2822(s) {
        s = s_out;
    }

    Ok((s, ()))
}

pub(crate) fn parse_rfc3339(mut s: &str) -> ParseResult<DateTime<FixedOffset>> {
    macro_rules! try_consume {
        ($e:expr) => {{
            let (s_, v) = $e?;
            s = s_;
            v
        }};
    }

    // an adapted RFC 3339 syntax from Section 5.6. Notes:
    // - quoted characters can be in any mixture of lower and upper case.
    // - any number of fractional digits is accepted for seconds; digits past
    //   the first 9 are skipped.
    // - unlike RFC 2822, the valid offset ranges from -23:59 to +23:59 (this
    //   restriction is unique to RFC 3339, not ISO 8601, so it is checked
    //   explicitly here).
    // - for readability a full-date and a full-time may be separated by a
    //   space character instead of `T`.

    let bytes = s.as_bytes();
    if bytes.len() < 19 {
        return Err(TOO_SHORT);
    }

    let fixed = <&[u8; 19]>::try_from(&bytes[..19]).unwrap(); // we just checked the length
    let year = digit(fixed, 0)? as u16 * 1000
        + digit(fixed, 1)? as u16 * 100
        + digit(fixed, 2)? as u16 * 10
        + digit(fixed, 3)? as u16;
    if bytes.get(4) != Some(&b'-') {
        return Err(INVALID);
    }

    let month = digit(fixed, 5)? * 10 + digit(fixed, 6)?;
    if bytes.get(7) != Some(&b'-') {
        return Err(INVALID);
    }

    let day = digit(fixed, 8)? * 10 + digit(fixed, 9)?;
    let date =
        NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32).ok_or(OUT_OF_RANGE)?;

    if !matches!(bytes.get(10), Some(&b't' | &b'T' | &b' ')) {
        return Err(INVALID);
    }

    let hour = digit(fixed, 11)? * 10 + digit(fixed, 12)?;
    if bytes.get(13) != Some(&b':') {
        return Err(INVALID);
    }

    let min = digit(fixed, 14)? * 10 + digit(fixed, 15)?;
    if bytes.get(16) != Some(&b':') {
        return Err(INVALID);
    }

    let sec = digit(fixed, 17)? * 10 + digit(fixed, 18)?;
    let (sec, extra_nanos) = match sec {
        60 => (59, 1_000_000_000), // rfc3339 allows leap seconds
        _ => (sec, 0),
    };

    let nano = if bytes.get(19) == Some(&b'.') {
        let nanosecond = try_consume!(scan::nanosecond(&s[20..]));
        extra_nanos + nanosecond
    } else {
        s = &s[19..];
        extra_nanos
    };

    let time = NaiveTime::from_hms_nano_opt(hour as u32, min as u32, sec as u32, nano)
        .ok_or(OUT_OF_RANGE)?;

    // Max for the hours field is `23`, and for the minutes field `59`.
    let offset = try_consume!(scan::timezone_offset(s, |s| scan::char(s, b':'), true, false, true));
    if !s.is_empty() {
        return Err(TOO_LONG);
    }

    let tz = FixedOffset::east_opt(offset).ok_or(OUT_OF_RANGE)?;
    Ok(match date.and_time(time).and_local_timezone(tz) {
        MappedLocalTime::Single(dt) => dt,
        // `FixedOffset`'s `TimeZone` impl never returns `Ambiguous`, and only
        // returns `None` on invalid data, which was already ruled out above.
        MappedLocalTime::Ambiguous(_, _) | MappedLocalTime::None => unreachable!(),
    })
}

#[inline]
fn digit(bytes: &[u8; 19], index: usize) -> ParseResult<u8> {
    match bytes[index].is_ascii_digit() {
        true => Ok(bytes[index] - b'0'),
        false => Err(INVALID),
    }
}

/// Tries to parse the given string into `parsed` with the given formatting
/// items. Returns `Ok` when the entire string has been parsed (otherwise
/// `parsed` should not be used). There should be no trailing string after
/// parsing; use a stray [`Item::Space`] to trim whitespace.
///
/// This parser is:
/// - Greedy: it consumes the longest possible prefix. For example, `April`
///   is always consumed entirely when the long month name is requested; it
///   equally accepts `Apr`, but prefers the longer prefix in this case.
/// - Padding-agnostic (for numeric items): the [`Pad`] field is completely
///   ignored, so one can prepend any number of whitespace then any number of
///   zeroes before numbers.
/// - Still obeying the intrinsic parsing width. This allows, for example,
///   parsing `HHMMSS`.
pub fn parse<'a, I, B>(parsed: &mut Parsed, s: &str, items: I) -> ParseResult<()>
where
    I: Iterator<Item = B>,
    B: Borrow<Item<'a>>,
{
    match parse_internal(parsed, s, items) {
        Ok("") => Ok(()),
        Ok(_) => Err(TOO_LONG), // if there are trailing chars it is an error
        Err(e) => Err(e),
    }
}

/// Tries to parse the given string into `parsed` with the given formatting
/// items. Returns `Ok` with a slice of the unparsed remainder.
pub fn parse_and_remainder<'a, 'b, I, B>(
    parsed: &mut Parsed,
    s: &'b str,
    items: I,
) -> ParseResult<&'b str>
where
    I: Iterator<Item = B>,
    B: Borrow<Item<'a>>,
{
    parse_internal(parsed, s, items)
}

fn parse_internal<'a, 'b, I, B>(
    parsed: &mut Parsed,
    mut s: &'b str,
    items: I,
) -> Result<&'b str, ParseError>
where
    I: Iterator<Item = B>,
    B: Borrow<Item<'a>>,
{
    macro_rules! try_consume {
        ($e:expr) => {{
            match $e {
                Ok((s_, v)) => {
                    s = s_;
                    v
                }
                Err(e) => return Err(e),
            }
        }};
    }

    for item in items {
        match *item.borrow() {
            Item::Literal(prefix) => {
                if s.len() < prefix.len() {
                    return Err(TOO_SHORT);
                }
                if !s.starts_with(prefix) {
                    return Err(INVALID);
                }
                s = &s[prefix.len()..];
            }

            Item::OwnedLiteral(ref prefix) => {
                if s.len() < prefix.len() {
                    return Err(TOO_SHORT);
                }
                if !s.starts_with(&prefix[..]) {
                    return Err(INVALID);
                }
                s = &s[prefix.len()..];
            }

            Item::Space(_) => {
                s = s.trim_start();
            }

            Item::OwnedSpace(_) => {
                s = s.trim_start();
            }

            Item::Numeric(ref spec, ref _pad) => {
                use super::Numeric::*;
                type Setter = fn(&mut Parsed, i64) -> ParseResult<()>;

                let (width, signed, set): (usize, bool, Setter) = match *spec {
                    Year => (4, true, Parsed::set_year),
                    YearDiv100 => (2, false, Parsed::set_year_div_100),
                    YearMod100 => (2, false, Parsed::set_year_mod_100),
                    IsoYear => (4, true, Parsed::set_isoyear),
                    IsoYearDiv100 => (2, false, Parsed::set_isoyear_div_100),
                    IsoYearMod100 => (2, false, Parsed::set_isoyear_mod_100),
                    Quarter => (1, false, Parsed::set_quarter),
                    Month => (2, false, Parsed::set_month),
                    Day => (2, false, Parsed::set_day),
                    WeekFromSun => (2, false, Parsed::set_week_from_sun),
                    WeekFromMon => (2, false, Parsed::set_week_from_mon),
                    IsoWeek => (2, false, Parsed::set_isoweek),
                    NumDaysFromSun => (1, false, set_weekday_with_num_days_from_sunday),
                    WeekdayFromMon => (1, false, set_weekday_with_number_from_monday),
                    Ordinal => (3, false, Parsed::set_ordinal),
                    Hour => (2, false, Parsed::set_hour),
                    Hour12 => (2, false, Parsed::set_hour12),
                    Minute => (2, false, Parsed::set_minute),
                    Second => (2, false, Parsed::set_second),
                    Nanosecond => (9, false, Parsed::set_nanosecond),
                    Timestamp => (usize::MAX, false, Parsed::set_timestamp),

                    // for future expansion
                    Internal(ref int) => match int._dummy {},
                };

                s = s.trim_start();
                let v = if signed {
                    if s.starts_with('-') {
                        let v = try_consume!(scan::number(&s[1..], 1, usize::MAX));
                        0i64.checked_sub(v).ok_or(OUT_OF_RANGE)?
                    } else if s.starts_with('+') {
                        try_consume!(scan::number(&s[1..], 1, usize::MAX))
                    } else {
                        // if there is no explicit sign, we respect the original `width`
                        try_consume!(scan::number(s, 1, width))
                    }
                } else {
                    try_consume!(scan::number(s, 1, width))
                };
                set(parsed, v)?;
            }

            Item::Fixed(ref spec) => {
                use super::Fixed::*;

                match spec {
                    &ShortMonthName => {
                        let month0 = try_consume!(scan::short_month0(s));
                        parsed.set_month(i64::from(month0) + 1)?;
                    }

                    &LongMonthName => {
                        let month0 = try_consume!(scan::short_or_long_month0(s));
                        parsed.set_month(i64::from(month0) + 1)?;
                    }

                    &ShortWeekdayName => {
                        let weekday = try_consume!(scan::short_weekday(s));
                        parsed.set_weekday(weekday)?;
                    }

                    &LongWeekdayName => {
                        let weekday = try_consume!(scan::short_or_long_weekday(s));
                        parsed.set_weekday(weekday)?;
                    }

                    &LowerAmPm | &UpperAmPm => {
                        if s.len() < 2 {
                            return Err(TOO_SHORT);
                        }
                        let ampm = match (s.as_bytes()[0] | 32, s.as_bytes()[1] | 32) {
                            (b'a', b'm') => false,
                            (b'p', b'm') => true,
                            _ => return Err(INVALID),
                        };
                        parsed.set_ampm(ampm)?;
                        s = &s[2..];
                    }

                    &Nanosecond => {
                        if s.starts_with('.') {
                            let nano = try_consume!(scan::nanosecond(&s[1..]));
                            parsed.set_nanosecond(nano as i64)?;
                        }
                    }

                    &Nanosecond3 => {
                        if s.starts_with('.') {
                            let nano = try_consume!(scan::nanosecond_fixed(&s[1..], 3));
                            parsed.set_nanosecond(nano)?;
                        }
                    }

                    &Nanosecond6 => {
                        if s.starts_with('.') {
                            let nano = try_consume!(scan::nanosecond_fixed(&s[1..], 6));
                            parsed.set_nanosecond(nano)?;
                        }
                    }

                    &Nanosecond9 => {
                        if s.starts_with('.') {
                            let nano = try_consume!(scan::nanosecond_fixed(&s[1..], 9));
                            parsed.set_nanosecond(nano)?;
                        }
                    }

                    &Internal(InternalFixed { val: InternalInternal::Nanosecond3NoDot }) => {
                        if s.len() < 3 {
                            return Err(TOO_SHORT);
                        }
                        let nano = try_consume!(scan::nanosecond_fixed(s, 3));
                        parsed.set_nanosecond(nano)?;
                    }

                    &Internal(InternalFixed { val: InternalInternal::Nanosecond6NoDot }) => {
                        if s.len() < 6 {
                            return Err(TOO_SHORT);
                        }
                        let nano = try_consume!(scan::nanosecond_fixed(s, 6));
                        parsed.set_nanosecond(nano)?;
                    }

                    &Internal(InternalFixed { val: InternalInternal::Nanosecond9NoDot }) => {
                        if s.len() < 9 {
                            return Err(TOO_SHORT);
                        }
                        let nano = try_consume!(scan::nanosecond_fixed(s, 9));
                        parsed.set_nanosecond(nano)?;
                    }

                    &TimezoneName => {
                        try_consume!(Ok((s.trim_start_matches(|c: char| !c.is_whitespace()), ())));
                    }

                    &TimezoneOffsetColon
                    | &TimezoneOffsetDoubleColon
                    | &TimezoneOffsetTripleColon
                    | &TimezoneOffset => {
                        let offset = try_consume!(scan::timezone_offset(
                            s.trim_start(),
                            scan::colon_or_space,
                            false,
                            false,
                            true,
                        ));
                        parsed.set_offset(i64::from(offset))?;
                    }

                    &TimezoneOffsetColonZ | &TimezoneOffsetZ => {
                        let offset = try_consume!(scan::timezone_offset(
                            s.trim_start(),
                            scan::colon_or_space,
                            true,
                            false,
                            true,
                        ));
                        parsed.set_offset(i64::from(offset))?;
                    }
                    &Internal(InternalFixed {
                        val: InternalInternal::TimezoneOffsetPermissive,
                    }) => {
                        let offset = try_consume!(scan::timezone_offset(
                            s.trim_start(),
                            scan::colon_or_space,
                            true,
                            true,
                            true,
                        ));
                        parsed.set_offset(i64::from(offset))?;
                    }

                    &RFC2822 => try_consume!(parse_rfc2822(parsed, s)),
                    &RFC3339 => {
                        // Used for the `%+` specifier: "Same as
                        // `%Y-%m-%dT%H:%M:%S%.f%:z`, ... also supports having
                        // a `Z` or `UTC` in place of `%:z`." Use the relaxed
                        // parser to match this description.
                        try_consume!(parse_rfc3339_relaxed(parsed, s))
                    }
                }
            }

            Item::Error => {
                return Err(BAD_FORMAT);
            }
        }
    }
    Ok(s)
}

/// Accepts a relaxed form of RFC 3339. A space or a `T` are accepted as the
/// separator between the date and time parts, and additional spaces are
/// allowed between components. All of these are equivalent:
/// `"2012-12-12T12:12:12Z"`, `"2012-12-12 12:12:12Z"`, `"2012-  12-12T12:  12:12Z"`.
impl str::FromStr for DateTime<FixedOffset> {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<DateTime<FixedOffset>> {
        let mut parsed = Parsed::new();
        let (s, _) = parse_rfc3339_relaxed(&mut parsed, s)?;
        if !s.trim_start().is_empty() {
            return Err(TOO_LONG);
        }
        parsed.to_datetime()
    }
}

/// Accepts a relaxed form of RFC 3339.
///
/// Differences with RFC 3339:
/// - Values don't require padding to two digits.
/// - Years outside the range 0..=9999 are accepted, but they must include a sign.
/// - `UTC` is accepted as a valid timezone name/offset (for compatibility
///   with the `Debug` format of `DateTime<Utc>`).
/// - There can be spaces between any of the components.
/// - The colon in the offset may be missing.
fn parse_rfc3339_relaxed<'a>(parsed: &mut Parsed, mut s: &'a str) -> ParseResult<(&'a str, ())> {
    const DATE_ITEMS: &[Item<'static>] = &[
        Item::Numeric(Numeric::Year, Pad::Zero),
        Item::Space(""),
        Item::Literal("-"),
        Item::Numeric(Numeric::Month, Pad::Zero),
        Item::Space(""),
        Item::Literal("-"),
        Item::Numeric(Numeric::Day, Pad::Zero),
    ];
    const TIME_ITEMS: &[Item<'static>] = &[
        Item::Numeric(Numeric::Hour, Pad::Zero),
        Item::Space(""),
        Item::Literal(":"),
        Item::Numeric(Numeric::Minute, Pad::Zero),
        Item::Space(""),
        Item::Literal(":"),
        Item::Numeric(Numeric::Second, Pad::Zero),
        Item::Fixed(Fixed::Nanosecond),
        Item::Space(""),
    ];

    s = parse_internal(parsed, s, DATE_ITEMS.iter())?;

    s = match s.as_bytes().first() {
        Some(&b't' | &b'T' | &b' ') => &s[1..],
        Some(_) => return Err(INVALID),
        None => return Err(TOO_SHORT),
    };

    s = parse_internal(parsed, s, TIME_ITEMS.iter())?;
    s = s.trim_start();
    let (s, offset) = if s.len() >= 3 && "UTC".as_bytes().eq_ignore_ascii_case(&s.as_bytes()[..3]) {
        (&s[3..], 0)
    } else {
        scan::timezone_offset(s, scan::colon_or_space, true, false, true)?
    };
    parsed.set_offset(i64::from(offset))?;
    Ok((s, ()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Datelike, Timelike};

    #[test]
    fn parse_numeric_and_literal_sequence_sets_fields() {
        let mut parsed = Parsed::new();
        let items = [
            Item::Numeric(Numeric::Year, Pad::Zero),
            Item::Literal("-"),
            Item::Numeric(Numeric::Month, Pad::Zero),
            Item::Literal("-"),
            Item::Numeric(Numeric::Day, Pad::Zero),
        ];
        parse(&mut parsed, "2023-06-15", items.iter()).unwrap();
        assert_eq!(parsed.year(), Some(2023));
        assert_eq!(parsed.month(), Some(6));
        assert_eq!(parsed.day(), Some(15));
        assert_eq!(parsed.to_naive_date().unwrap(), NaiveDate::from_ymd_opt(2023, 6, 15).unwrap());
    }

    #[test]
    fn parse_rejects_trailing_input() {
        let mut parsed = Parsed::new();
        let items = [Item::Numeric(Numeric::Year, Pad::Zero)];
        assert_eq!(parse(&mut parsed, "2023extra", items.iter()), Err(TOO_LONG));
    }

    #[test]
    fn parse_and_remainder_returns_the_unparsed_tail() {
        let mut parsed = Parsed::new();
        let items = [Item::Numeric(Numeric::Year, Pad::Zero)];
        let rest = parse_and_remainder(&mut parsed, "2023extra", items.iter()).unwrap();
        assert_eq!(rest, "extra");
        assert_eq!(parsed.year(), Some(2023));
    }

    #[test]
    fn parse_literal_mismatch_and_too_short() {
        let mut parsed = Parsed::new();
        let items = [Item::Literal("abc")];
        assert_eq!(parse(&mut parsed, "xyz", items.iter()), Err(INVALID));
        assert_eq!(parse(&mut parsed, "ab", items.iter()), Err(TOO_SHORT));
    }

    #[test]
    fn parse_space_item_trims_leading_whitespace() {
        let mut parsed = Parsed::new();
        let items = [Item::Space(""), Item::Literal("x")];
        parse(&mut parsed, "   x", items.iter()).unwrap();
    }

    #[test]
    fn parse_error_item_yields_bad_format() {
        let mut parsed = Parsed::new();
        let items = [Item::Error];
        assert_eq!(parse(&mut parsed, "anything", items.iter()), Err(BAD_FORMAT));
    }

    #[test]
    fn parse_year_accepts_explicit_sign() {
        let items = [Item::Numeric(Numeric::Year, Pad::Zero)];

        let mut negative = Parsed::new();
        parse(&mut negative, "-0044", items.iter()).unwrap();
        assert_eq!(negative.year(), Some(-44));

        let mut positive = Parsed::new();
        parse(&mut positive, "+12345", items.iter()).unwrap();
        assert_eq!(positive.year(), Some(12345));
    }

    #[test]
    fn parse_year_mod_100_is_unsigned_and_width_limited() {
        let mut parsed = Parsed::new();
        let items = [Item::Numeric(Numeric::YearMod100, Pad::Zero)];
        parse(&mut parsed, "23", items.iter()).unwrap();
        assert_eq!(parsed.year_mod_100(), Some(23));
    }

    #[test]
    fn parse_num_days_from_sun_and_weekday_from_mon_set_weekday() {
        let mut parsed = Parsed::new();
        let items = [Item::Numeric(Numeric::NumDaysFromSun, Pad::None)];
        parse(&mut parsed, "4", items.iter()).unwrap();
        assert_eq!(parsed.weekday(), Some(Weekday::Thu));

        let mut parsed2 = Parsed::new();
        let items2 = [Item::Numeric(Numeric::WeekdayFromMon, Pad::None)];
        parse(&mut parsed2, "7", items2.iter()).unwrap();
        assert_eq!(parsed2.weekday(), Some(Weekday::Sun));
    }

    #[test]
    fn parse_rfc2822_fixed_item_parses_and_sets_offset() {
        let mut parsed = Parsed::new();
        let items = [Item::Fixed(Fixed::RFC2822)];
        parse(&mut parsed, "Tue, 1 Jul 2003 10:52:37 +0200", items.iter()).unwrap();
        assert_eq!(parsed.year(), Some(2003));
        assert_eq!(parsed.month(), Some(7));
        assert_eq!(parsed.day(), Some(1));
        assert_eq!(parsed.weekday(), Some(Weekday::Tue));
        assert_eq!(parsed.offset(), Some(7200));
    }

    #[test]
    fn parse_rfc2822_mismatching_weekday_is_caught_on_resolution() {
        // Parsing itself only records the weekday text; the year/month/day
        // vs weekday consistency check happens later, in `to_naive_date`.
        let mut parsed = Parsed::new();
        let items = [Item::Fixed(Fixed::RFC2822)];
        // 2003-07-01 is a Tuesday, not a Monday.
        parse(&mut parsed, "Mon, 1 Jul 2003 10:52:37 +0200", items.iter()).unwrap();
        assert!(parsed.to_naive_date().is_err());
    }

    #[test]
    fn parse_rfc2822_two_digit_year_heuristic() {
        let mut parsed = Parsed::new();
        let items = [Item::Fixed(Fixed::RFC2822)];
        parse(&mut parsed, "Tue, 1 Jul 03 10:52:37 +0200", items.iter()).unwrap();
        assert_eq!(parsed.year(), Some(2003));
    }

    #[test]
    fn parse_rfc3339_fixed_item_accepts_z_and_relaxed_spacing() {
        let mut parsed = Parsed::new();
        let items = [Item::Fixed(Fixed::RFC3339)];
        parse(&mut parsed, "2023-06-15T09:30:00Z", items.iter()).unwrap();
        assert_eq!(parsed.to_datetime().unwrap().to_rfc3339(), "2023-06-15T09:30:00+00:00");
    }

    #[test]
    fn parse_rfc3339_fixed_item_accepts_utc_and_space_separator_and_colonless_offset() {
        let items = [Item::Fixed(Fixed::RFC3339)];

        let mut parsed = Parsed::new();
        parse(&mut parsed, "2023-06-15 09:30:00 UTC", items.iter()).unwrap();
        assert_eq!(parsed.offset(), Some(0));

        let mut parsed2 = Parsed::new();
        parse(&mut parsed2, "2023-06-15T09:30:00+0200", items.iter()).unwrap();
        assert_eq!(parsed2.offset(), Some(7200));
    }

    #[test]
    fn parse_rfc3339_strict_parses_basic_datetime() {
        let dt = parse_rfc3339("1996-12-19T16:39:57-08:00").unwrap();
        assert_eq!((dt.year(), dt.month(), dt.day()), (1996, 12, 19));
        assert_eq!(dt.offset().local_minus_utc(), -8 * 3600);
    }

    #[test]
    fn parse_rfc3339_strict_handles_leap_second() {
        let dt = parse_rfc3339("2016-12-31T23:59:60Z").unwrap();
        assert_eq!(dt.second(), 59); // `second()` never reports 60.
        assert_eq!(dt.nanosecond(), 1_000_000_000);
    }

    #[test]
    fn parse_rfc3339_strict_rejects_too_short_input() {
        assert_eq!(parse_rfc3339("2023-06-15"), Err(TOO_SHORT));
    }

    #[test]
    fn parse_rfc3339_strict_rejects_invalid_date() {
        assert!(parse_rfc3339("2023-02-30T00:00:00Z").is_err());
    }

    #[test]
    fn parse_rfc3339_strict_requires_colon_in_offset() {
        // Unlike the relaxed `%+`/`FromStr` parser, the strict RFC 3339
        // parser (used by `DateTime::parse_from_rfc3339`) requires a colon.
        assert!(parse_rfc3339("2023-06-15T09:30:00+0200").is_err());
    }

    #[test]
    fn fromstr_datetime_fixedoffset_accepts_relaxed_spacing_and_utc() {
        let dt: DateTime<FixedOffset> = "2012-12-12 12:12:12Z".parse().unwrap();
        assert_eq!(dt.offset().local_minus_utc(), 0);

        let dt2: DateTime<FixedOffset> = "2012-  12-12T12:  12:12Z".parse().unwrap();
        assert_eq!((dt2.year(), dt2.month(), dt2.day()), (2012, 12, 12));

        let dt3: DateTime<FixedOffset> = "2012-12-12T12:12:12 UTC".parse().unwrap();
        assert_eq!(dt3.offset().local_minus_utc(), 0);
    }

    #[test]
    fn fromstr_datetime_fixedoffset_rejects_trailing_garbage() {
        assert!("2012-12-12T12:12:12Z trailing".parse::<DateTime<FixedOffset>>().is_err());
    }
}
