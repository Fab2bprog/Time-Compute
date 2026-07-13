//! Various scanning routines for the parser.

use super::{ParseResult, INVALID, OUT_OF_RANGE, TOO_SHORT};
use crate::Weekday;

/// Tries to parse the non-negative number from `min` to `max` digits.
///
/// The absence of digits at all is an unconditional error.
/// More than `max` digits are consumed up to the first `max` digits.
/// Any number that does not fit in `i64` is an error.
#[inline]
pub(super) fn number(s: &str, min: usize, max: usize) -> ParseResult<(&str, i64)> {
    assert!(min <= max);

    // We are only interested in ascii numbers, so we can work with the `str`
    // as bytes. We stop on the first non-numeric byte, which may be another
    // ascii character or the beginning of a multi-byte UTF-8 character.
    let bytes = s.as_bytes();
    if bytes.len() < min {
        return Err(TOO_SHORT);
    }

    let mut n = 0i64;
    for (i, c) in bytes.iter().take(max).copied().enumerate() {
        if !c.is_ascii_digit() {
            if i < min {
                return Err(INVALID);
            } else {
                return Ok((&s[i..], n));
            }
        }

        n = match n.checked_mul(10).and_then(|n| n.checked_add((c - b'0') as i64)) {
            Some(n) => n,
            None => return Err(OUT_OF_RANGE),
        };
    }

    Ok((&s[core::cmp::min(max, bytes.len())..], n))
}

/// Tries to consume at least one digit as a fractional second.
/// Returns the number of whole nanoseconds (0--999,999,999).
pub(super) fn nanosecond(s: &str) -> ParseResult<(&str, u32)> {
    // record the number of digits consumed for later scaling.
    let origlen = s.len();
    let (s, v) = number(s, 1, 9)?;
    let v = u32::try_from(v).expect("999,999,999 should fit u32");
    let consumed = origlen - s.len();

    // scale the number accordingly.
    const SCALE: [u32; 10] =
        [0, 100_000_000, 10_000_000, 1_000_000, 100_000, 10_000, 1_000, 100, 10, 1];
    let v = v.checked_mul(SCALE[consumed]).ok_or(OUT_OF_RANGE)?;

    // if there are more than 9 digits, skip the remaining digits.
    let s = s.trim_start_matches(|c: char| c.is_ascii_digit());

    Ok((s, v))
}

/// Tries to consume a fixed number of digits as a fractional second.
/// Returns the number of whole nanoseconds (0--999,999,999).
pub(super) fn nanosecond_fixed(s: &str, digits: usize) -> ParseResult<(&str, i64)> {
    // record the number of digits consumed for later scaling.
    let (s, v) = number(s, digits, digits)?;

    // scale the number accordingly.
    static SCALE: [i64; 10] =
        [0, 100_000_000, 10_000_000, 1_000_000, 100_000, 10_000, 1_000, 100, 10, 1];
    let v = v.checked_mul(SCALE[digits]).ok_or(OUT_OF_RANGE)?;

    Ok((s, v))
}

/// Tries to parse the month index (0 through 11) with the first three ASCII letters.
pub(super) fn short_month0(s: &str) -> ParseResult<(&str, u8)> {
    if s.len() < 3 {
        return Err(TOO_SHORT);
    }
    let buf = s.as_bytes();
    let month0 = match (buf[0] | 32, buf[1] | 32, buf[2] | 32) {
        (b'j', b'a', b'n') => 0,
        (b'f', b'e', b'b') => 1,
        (b'm', b'a', b'r') => 2,
        (b'a', b'p', b'r') => 3,
        (b'm', b'a', b'y') => 4,
        (b'j', b'u', b'n') => 5,
        (b'j', b'u', b'l') => 6,
        (b'a', b'u', b'g') => 7,
        (b's', b'e', b'p') => 8,
        (b'o', b'c', b't') => 9,
        (b'n', b'o', b'v') => 10,
        (b'd', b'e', b'c') => 11,
        _ => return Err(INVALID),
    };
    Ok((&s[3..], month0))
}

/// Tries to parse the weekday with the first three ASCII letters.
pub(super) fn short_weekday(s: &str) -> ParseResult<(&str, Weekday)> {
    if s.len() < 3 {
        return Err(TOO_SHORT);
    }
    let buf = s.as_bytes();
    let weekday = match (buf[0] | 32, buf[1] | 32, buf[2] | 32) {
        (b'm', b'o', b'n') => Weekday::Mon,
        (b't', b'u', b'e') => Weekday::Tue,
        (b'w', b'e', b'd') => Weekday::Wed,
        (b't', b'h', b'u') => Weekday::Thu,
        (b'f', b'r', b'i') => Weekday::Fri,
        (b's', b'a', b't') => Weekday::Sat,
        (b's', b'u', b'n') => Weekday::Sun,
        _ => return Err(INVALID),
    };
    Ok((&s[3..], weekday))
}

/// Tries to parse the month index (0 through 11) with short or long month names.
/// It prefers long month names to short month names when both are possible.
pub(super) fn short_or_long_month0(s: &str) -> ParseResult<(&str, u8)> {
    // lowercased month names, minus first three chars
    static LONG_MONTH_SUFFIXES: [&[u8]; 12] = [
        b"uary", b"ruary", b"ch", b"il", b"", b"e", b"y", b"ust", b"tember", b"ober", b"ember",
        b"ember",
    ];

    let (mut s, month0) = short_month0(s)?;

    // tries to consume the suffix if possible
    let suffix = LONG_MONTH_SUFFIXES[month0 as usize];
    if s.len() >= suffix.len() && s.as_bytes()[..suffix.len()].eq_ignore_ascii_case(suffix) {
        s = &s[suffix.len()..];
    }

    Ok((s, month0))
}

/// Tries to parse the weekday with short or long weekday names.
/// It prefers long weekday names to short weekday names when both are possible.
pub(super) fn short_or_long_weekday(s: &str) -> ParseResult<(&str, Weekday)> {
    // lowercased weekday names, minus first three chars
    static LONG_WEEKDAY_SUFFIXES: [&[u8]; 7] =
        [b"day", b"sday", b"nesday", b"rsday", b"day", b"urday", b"day"];

    let (mut s, weekday) = short_weekday(s)?;

    // tries to consume the suffix if possible
    let suffix = LONG_WEEKDAY_SUFFIXES[weekday.num_days_from_monday() as usize];
    if s.len() >= suffix.len() && s.as_bytes()[..suffix.len()].eq_ignore_ascii_case(suffix) {
        s = &s[suffix.len()..];
    }

    Ok((s, weekday))
}

/// Tries to consume exactly one given character.
pub(super) fn char(s: &str, c1: u8) -> ParseResult<&str> {
    match s.as_bytes().first() {
        Some(&c) if c == c1 => Ok(&s[1..]),
        Some(_) => Err(INVALID),
        None => Err(TOO_SHORT),
    }
}

/// Tries to consume one or more whitespace.
pub(super) fn space(s: &str) -> ParseResult<&str> {
    let s_ = s.trim_start();
    if s_.len() < s.len() {
        Ok(s_)
    } else if s.is_empty() {
        Err(TOO_SHORT)
    } else {
        Err(INVALID)
    }
}

/// Consumes any number (including zero) of colons or spaces.
pub(crate) fn colon_or_space(s: &str) -> ParseResult<&str> {
    Ok(s.trim_start_matches(|c: char| c == ':' || c.is_whitespace()))
}

/// Parses a timezone from `s` and returns the offset in seconds.
///
/// The `consume_colon` function is used to parse a mandatory or optional `:`
/// separator between the hours offset and the minutes offset.
///
/// The `allow_missing_minutes` flag allows the timezone minutes offset to be
/// missing from `s`.
///
/// The `allow_tz_minus_sign` flag allows the timezone offset negative
/// character to also be `-` MINUS SIGN (U+2212) in addition to the typical
/// ASCII-compatible `-` HYPHEN-MINUS (U+2D). This is part of RFC 3339 & ISO 8601.
pub(crate) fn timezone_offset<F>(
    mut s: &str,
    mut consume_colon: F,
    allow_zulu: bool,
    allow_missing_minutes: bool,
    allow_tz_minus_sign: bool,
) -> ParseResult<(&str, i32)>
where
    F: FnMut(&str) -> ParseResult<&str>,
{
    if allow_zulu {
        if let Some(&b'Z' | &b'z') = s.as_bytes().first() {
            return Ok((&s[1..], 0));
        }
    }

    const fn digits(s: &str) -> ParseResult<(u8, u8)> {
        let b = s.as_bytes();
        if b.len() < 2 {
            Err(TOO_SHORT)
        } else {
            Ok((b[0], b[1]))
        }
    }
    let negative = match s.chars().next() {
        Some('+') => {
            // PLUS SIGN (U+2B)
            s = &s['+'.len_utf8()..];
            false
        }
        Some('-') => {
            // HYPHEN-MINUS (U+2D)
            s = &s['-'.len_utf8()..];
            true
        }
        Some('\u{2212}') => {
            // MINUS SIGN (U+2212)
            if !allow_tz_minus_sign {
                return Err(INVALID);
            }
            s = &s['\u{2212}'.len_utf8()..];
            true
        }
        Some(_) => return Err(INVALID),
        None => return Err(TOO_SHORT),
    };

    // hours (00--99)
    let hours = match digits(s)? {
        (h1 @ b'0'..=b'9', h2 @ b'0'..=b'9') => i32::from((h1 - b'0') * 10 + (h2 - b'0')),
        _ => return Err(INVALID),
    };
    s = &s[2..];

    // colons (and possibly other separators)
    s = consume_colon(s)?;

    // minutes (00--59)
    // if the next two items are digits then we have to add minutes
    let minutes = if let Ok(ds) = digits(s) {
        match ds {
            (m1 @ b'0'..=b'5', m2 @ b'0'..=b'9') => i32::from((m1 - b'0') * 10 + (m2 - b'0')),
            (b'6'..=b'9', b'0'..=b'9') => return Err(OUT_OF_RANGE),
            _ => return Err(INVALID),
        }
    } else if allow_missing_minutes {
        0
    } else {
        return Err(TOO_SHORT);
    };
    s = match s.len() {
        len if len >= 2 => &s[2..],
        0 => s,
        _ => return Err(TOO_SHORT),
    };

    let seconds = hours * 3600 + minutes * 60;
    Ok((s, if negative { -seconds } else { seconds }))
}

/// Same as `timezone_offset` but also allows for RFC 2822 legacy timezones.
/// See RFC 2822 Section 4.3.
pub(super) fn timezone_offset_2822(s: &str) -> ParseResult<(&str, i32)> {
    // tries to parse legacy time zone names
    let upto = s.as_bytes().iter().position(|&c| !c.is_ascii_alphabetic()).unwrap_or(s.len());
    if upto > 0 {
        let name = &s.as_bytes()[..upto];
        let s = &s[upto..];
        let offset_hours = |o| Ok((s, o * 3600));
        // RFC 2822 requires support for some named North America timezones, a
        // small subset of all named timezones.
        if name.eq_ignore_ascii_case(b"gmt")
            || name.eq_ignore_ascii_case(b"ut")
            || name.eq_ignore_ascii_case(b"z")
        {
            return offset_hours(0);
        } else if name.eq_ignore_ascii_case(b"edt") {
            return offset_hours(-4);
        } else if name.eq_ignore_ascii_case(b"est") || name.eq_ignore_ascii_case(b"cdt") {
            return offset_hours(-5);
        } else if name.eq_ignore_ascii_case(b"cst") || name.eq_ignore_ascii_case(b"mdt") {
            return offset_hours(-6);
        } else if name.eq_ignore_ascii_case(b"mst") || name.eq_ignore_ascii_case(b"pdt") {
            return offset_hours(-7);
        } else if name.eq_ignore_ascii_case(b"pst") {
            return offset_hours(-8);
        } else if name.len() == 1 {
            if let b'a'..=b'i' | b'k'..=b'y' | b'A'..=b'I' | b'K'..=b'Y' = name[0] {
                // recommended by RFC 2822: consume but treat it as -0000
                return Ok((s, 0));
            }
        }
        Err(INVALID)
    } else {
        timezone_offset(s, |s| Ok(s), false, false, false)
    }
}

/// Tries to consume an RFC 2822 comment including the preceding whitespace.
///
/// Returns the remaining string after the closing parenthesis.
pub(super) fn comment_2822(s: &str) -> ParseResult<(&str, ())> {
    use CommentState::*;

    let s = s.trim_start();

    let mut state = Start;
    for (i, c) in s.bytes().enumerate() {
        state = match (state, c) {
            (Start, b'(') => Next(1),
            (Next(1), b')') => return Ok((&s[i + 1..], ())),
            (Next(depth), b'\\') => Escape(depth),
            (Next(depth), b'(') => Next(depth + 1),
            (Next(depth), b')') => Next(depth - 1),
            (Next(depth), _) | (Escape(depth), _) => Next(depth),
            _ => return Err(INVALID),
        };
    }

    Err(TOO_SHORT)
}

enum CommentState {
    Start,
    Next(usize),
    Escape(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_parses_within_min_max_digits() {
        assert_eq!(number("2023", 1, 4), Ok(("", 2023)));
        assert_eq!(number("12ab", 1, 2), Ok(("ab", 12)));
    }

    #[test]
    fn number_rejects_fewer_than_min_digits() {
        assert_eq!(number("1ab", 2, 2), Err(INVALID));
        assert_eq!(number("", 1, 2), Err(TOO_SHORT));
    }

    #[test]
    fn number_rejects_overflow() {
        assert_eq!(number("99999999999999999999", 1, 20), Err(OUT_OF_RANGE));
    }

    #[test]
    fn nanosecond_scales_by_digit_count() {
        assert_eq!(nanosecond("5"), Ok(("", 500_000_000)));
        assert_eq!(nanosecond("123"), Ok(("", 123_000_000)));
        assert_eq!(nanosecond("123456789"), Ok(("", 123_456_789)));
        assert_eq!(nanosecond("12abc"), Ok(("abc", 120_000_000)));
    }

    #[test]
    fn nanosecond_skips_digits_beyond_nine() {
        // Only the first 9 digits count; any further digits are consumed
        // and discarded rather than left in the remainder.
        assert_eq!(nanosecond("1234567890"), Ok(("", 123_456_789)));
    }

    #[test]
    fn nanosecond_fixed_requires_exact_digit_count() {
        assert_eq!(nanosecond_fixed("123", 3), Ok(("", 123_000_000)));
        assert_eq!(nanosecond_fixed("12", 3), Err(TOO_SHORT));
        // Unlike `nanosecond`, extra digits are left in the remainder.
        assert_eq!(nanosecond_fixed("1234", 3), Ok(("4", 123_000_000)));
    }

    #[test]
    fn short_month0_parses_case_insensitively() {
        assert_eq!(short_month0("JAN"), Ok(("", 0)));
        assert_eq!(short_month0("Decxyz"), Ok(("xyz", 11)));
        assert_eq!(short_month0("xyz"), Err(INVALID));
        assert_eq!(short_month0("ja"), Err(TOO_SHORT));
    }

    #[test]
    fn short_weekday_parses_case_insensitively() {
        assert_eq!(short_weekday("SUN"), Ok(("", Weekday::Sun)));
        assert_eq!(short_weekday("Monxyz"), Ok(("xyz", Weekday::Mon)));
        assert_eq!(short_weekday("xyz"), Err(INVALID));
    }

    #[test]
    fn short_or_long_month0_prefers_long_form_when_present() {
        assert_eq!(short_or_long_month0("January"), Ok(("", 0)));
        assert_eq!(short_or_long_month0("Jan"), Ok(("", 0)));
        assert_eq!(short_or_long_month0("Marchxyz"), Ok(("xyz", 2)));
    }

    #[test]
    fn short_or_long_weekday_prefers_long_form_when_present() {
        assert_eq!(short_or_long_weekday("Sunday"), Ok(("", Weekday::Sun)));
        assert_eq!(short_or_long_weekday("Monday"), Ok(("", Weekday::Mon)));
        assert_eq!(short_or_long_weekday("Tuesday"), Ok(("", Weekday::Tue)));
        assert_eq!(short_or_long_weekday("Wednesday"), Ok(("", Weekday::Wed)));
        assert_eq!(short_or_long_weekday("Tue"), Ok(("", Weekday::Tue)));
    }

    #[test]
    fn char_consumes_exactly_one_matching_byte() {
        assert_eq!(char("abc", b'a'), Ok("bc"));
        assert_eq!(char("abc", b'x'), Err(INVALID));
        assert_eq!(char("", b'a'), Err(TOO_SHORT));
    }

    #[test]
    fn space_requires_at_least_one_whitespace_char() {
        assert_eq!(space("  abc"), Ok("abc"));
        assert_eq!(space("abc"), Err(INVALID));
        assert_eq!(space(""), Err(TOO_SHORT));
    }

    #[test]
    fn colon_or_space_consumes_zero_or_more_and_never_errors() {
        assert_eq!(colon_or_space("::  abc"), Ok("abc"));
        assert_eq!(colon_or_space("abc"), Ok("abc"));
        assert_eq!(colon_or_space(""), Ok(""));
    }

    #[test]
    fn timezone_offset_parses_colon_and_no_colon_forms() {
        assert_eq!(timezone_offset("+09:00", colon_or_space, false, false, true), Ok(("", 9 * 3600)));
        assert_eq!(timezone_offset("-0400", colon_or_space, false, false, true), Ok(("", -4 * 3600)));
    }

    #[test]
    fn timezone_offset_allows_zulu_when_requested() {
        assert_eq!(timezone_offset("Z", |s| Ok(s), true, true, true), Ok(("", 0)));
    }

    #[test]
    fn timezone_offset_allows_missing_minutes_when_requested() {
        assert_eq!(timezone_offset("+05", colon_or_space, false, true, true), Ok(("", 5 * 3600)));
        assert_eq!(timezone_offset("+05", colon_or_space, false, false, true), Err(TOO_SHORT));
    }

    #[test]
    fn timezone_offset_rejects_out_of_range_minutes() {
        assert_eq!(timezone_offset("+1260", colon_or_space, false, false, true), Err(OUT_OF_RANGE));
    }

    #[test]
    fn timezone_offset_unicode_minus_sign_requires_the_flag() {
        assert_eq!(
            timezone_offset("\u{2212}0400", colon_or_space, false, false, true),
            Ok(("", -4 * 3600))
        );
        assert_eq!(
            timezone_offset("\u{2212}0400", colon_or_space, false, false, false),
            Err(INVALID)
        );
    }

    #[test]
    fn timezone_offset_2822_parses_named_north_american_zones() {
        assert_eq!(timezone_offset_2822("GMT"), Ok(("", 0)));
        assert_eq!(timezone_offset_2822("EST"), Ok(("", -5 * 3600)));
        assert_eq!(timezone_offset_2822("PST"), Ok(("", -8 * 3600)));
    }

    #[test]
    fn timezone_offset_2822_falls_back_to_numeric_offset() {
        assert_eq!(timezone_offset_2822("+0200"), Ok(("", 2 * 3600)));
    }

    #[test]
    fn timezone_offset_2822_single_letter_military_zone_is_treated_as_zero() {
        assert_eq!(timezone_offset_2822("A"), Ok(("", 0)));
    }

    #[test]
    fn timezone_offset_2822_rejects_letter_j() {
        // "J" (military "juliet") is explicitly excluded from the
        // fallback-to-zero rule in RFC 2822.
        assert_eq!(timezone_offset_2822("J"), Err(INVALID));
    }

    #[test]
    fn comment_2822_consumes_a_simple_parenthesized_comment() {
        assert_eq!(comment_2822(" (comment) rest"), Ok((" rest", ())));
    }

    #[test]
    fn comment_2822_handles_nested_parentheses() {
        assert_eq!(comment_2822("(a(b)c)rest"), Ok(("rest", ())));
    }

    #[test]
    fn comment_2822_rejects_unterminated_comment() {
        assert_eq!(comment_2822("(abc"), Err(TOO_SHORT));
    }

    #[test]
    fn comment_2822_rejects_missing_opening_paren() {
        assert_eq!(comment_2822("abc"), Err(INVALID));
    }
}
