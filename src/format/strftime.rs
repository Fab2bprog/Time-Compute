/*!
`strftime`/`strptime`-inspired date and time formatting syntax.

## Specifiers

The following specifiers are available both for formatting and parsing.

| Spec. | Example  | Description                                                                |
|-------|----------|-----------------------------------------------------------------------------|
|       |          | **DATE SPECIFIERS:**                                                       |
| `%Y`  | `2001`   | The full proleptic Gregorian year, zero-padded to 4 digits.                |
| `%C`  | `20`     | The proleptic Gregorian year divided by 100, zero-padded to 2 digits.      |
| `%y`  | `01`     | The proleptic Gregorian year modulo 100, zero-padded to 2 digits.          |
| `%q`  | `1`      | Quarter of year (1-4).                                                     |
| `%m`  | `07`     | Month number (01--12), zero-padded to 2 digits.                            |
| `%b`  | `Jul`    | Abbreviated month name. Always 3 letters.                                  |
| `%B`  | `July`   | Full month name. Also accepts corresponding abbreviation in parsing.       |
| `%h`  | `Jul`    | Same as `%b`.                                                              |
| `%d`  | `08`     | Day number (01--31), zero-padded to 2 digits.                              |
| `%e`  | ` 8`     | Same as `%d` but space-padded. Same as `%_d`.                              |
| `%a`  | `Sun`    | Abbreviated weekday name. Always 3 letters.                                |
| `%A`  | `Sunday` | Full weekday name. Also accepts corresponding abbreviation in parsing.     |
| `%w`  | `0`      | Sunday = 0, Monday = 1, ..., Saturday = 6.                                 |
| `%u`  | `7`      | Monday = 1, Tuesday = 2, ..., Sunday = 7 (ISO 8601).                       |
| `%U`  | `28`     | Week number starting with Sunday (00--53), zero-padded to 2 digits.        |
| `%W`  | `27`     | Same as `%U`, but week 1 starts with the first Monday in that year.        |
| `%G`  | `2001`   | Same as `%Y` but uses the year number in the ISO 8601 week date.           |
| `%g`  | `01`     | Same as `%y` but uses the year number in the ISO 8601 week date.           |
| `%V`  | `27`     | Same as `%U` but uses the week number in the ISO 8601 week date (01--53).  |
| `%j`  | `189`    | Day of the year (001--366), zero-padded to 3 digits.                       |
| `%D`  | `07/08/01`    | Month-day-year format. Same as `%m/%d/%y`.                            |
| `%x`  | `07/08/01`    | Same as `%D` (no locale support: always the US month/day/year form).  |
| `%F`  | `2001-07-08`  | Year-month-day format (ISO 8601). Same as `%Y-%m-%d`.                 |
| `%v`  | ` 8-Jul-2001` | Day-month-year format. Same as `%e-%b-%Y`.                            |
|       |          | **TIME SPECIFIERS:**                                                       |
| `%H`  | `00`     | Hour number (00--23), zero-padded to 2 digits.                             |
| `%k`  | ` 0`     | Same as `%H` but space-padded. Same as `%_H`.                              |
| `%I`  | `12`     | Hour number in 12-hour clocks (01--12), zero-padded to 2 digits.           |
| `%l`  | `12`     | Same as `%I` but space-padded. Same as `%_I`.                              |
| `%P`  | `am`     | `am` or `pm` in 12-hour clocks.                                            |
| `%p`  | `AM`     | `AM` or `PM` in 12-hour clocks.                                            |
| `%M`  | `34`     | Minute number (00--59), zero-padded to 2 digits.                           |
| `%S`  | `60`     | Second number (00--60), zero-padded to 2 digits (accounts for leap seconds). |
| `%f`  | `26490000`    | Number of nanoseconds since the last whole second.                    |
| `%.f` | `.026490`| Decimal fraction of a second. Consumes the leading dot.                    |
| `%.3f`| `.026`        | Decimal fraction of a second with a fixed length of 3.                |
| `%.6f`| `.026490`     | Decimal fraction of a second with a fixed length of 6.                |
| `%.9f`| `.026490000`  | Decimal fraction of a second with a fixed length of 9.                |
| `%3f` | `026`         | Like `%.3f` but without the leading dot.                              |
| `%6f` | `026490`      | Like `%.6f` but without the leading dot.                              |
| `%9f` | `026490000`   | Like `%.9f` but without the leading dot.                              |
| `%R`  | `00:34`       | Hour-minute format. Same as `%H:%M`.                                  |
| `%T`  | `00:34:60`    | Hour-minute-second format. Same as `%H:%M:%S`.                        |
| `%X`  | `00:34:60`    | Same as `%T` (no locale support).                                     |
| `%r`  | `12:34:60 AM` | 12-hour clock time. Same as `%I:%M:%S %p`.                            |
|       |          | **TIME ZONE SPECIFIERS:**                                                  |
| `%Z`  | `+09:30` | Prints the offset (this crate is not aware of timezone name abbreviations). Skips all non-whitespace characters during parsing. |
| `%z`  | `+0930`  | Offset from the local time to UTC (with UTC being `+0000`).                |
| `%:z` | `+09:30` | Same as `%z` but with a colon.                                             |
|`%::z`|`+09:30:00`| Offset from the local time to UTC with seconds.                            |
|`%:::z`| `+09`    | Offset from the local time to UTC without minutes.                         |
| `%#z` | `+09`    | *Parsing only:* Same as `%z` but allows minutes to be missing or present.  |
|       |          | **DATE & TIME SPECIFIERS:**                                                |
|`%c`|`Sun Jul  8 00:34:60 2001`| Same as `%a %b %e %H:%M:%S %Y`.                                |
| `%+`  | `2001-07-08T00:34:60.026490+09:30` | ISO 8601 / RFC 3339 date & time format. |
| `%s`  | `994518299`   | UNIX timestamp, the number of (non-leap) seconds since 1970-01-01 00:00 UTC. |
|       |          | **SPECIAL SPECIFIERS:**                                                    |
| `%t`  |          | Literal tab (`\t`).                                                        |
| `%n`  |          | Literal newline (`\n`).                                                    |
| `%%`  |          | Literal percent sign.                                                      |

It is possible to override the default padding behavior of numeric specifiers `%?`.
This is not allowed for other specifiers and results in a `BadFormat` parse error.

| Modifier | Description                                                          |
|----------|-----------------------------------------------------------------------|
| `%-?`    | Suppresses any padding including spaces and zeroes (e.g. `%j` = `012`, `%-j` = `12`). |
| `%_?`    | Uses spaces as padding (e.g. `%j` = `012`, `%_j` = ` 12`).            |
| `%0?`    | Uses zeroes as padding (e.g. `%e` = ` 9`, `%0e` = `09`).              |

By default, `%x`, `%X`, `%c` and `%r` use their English/US fallback forms,
and month/weekday names print in English. With the `unstable-locales`
crate feature enabled, [`StrftimeItems::new_with_locale`] adjusts `%x`/
`%X`/`%c`/`%r` to the given [`Locale`](super::Locale)'s formats; combine it
with a `format_localized`/`format_localized_with_items` method (see
[`NaiveDate`](crate::NaiveDate) or [`DateTime`](crate::DateTime)) to also
get localized month/weekday names.
*/

#[cfg(feature = "unstable-locales")]
use super::{locales, Locale};
use super::{BAD_FORMAT, Fixed, InternalInternal, Item, Numeric, Pad, ParseError};
use super::{fixed, internal_fixed, num, num0, nums};
use std::vec::Vec;

/// Parsing iterator for `strftime`-like format strings.
///
/// See the [`strftime` module](self) for the supported formatting specifiers.
///
/// `StrftimeItems` is used in combination with more low-level functions such
/// as [`format::parse()`](super::parse()). If formatting or parsing date and
/// time values is not performance-critical, the `format`/`parse_from_str`
/// methods on types such as [`DateTime`](crate::DateTime) are easier to use.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StrftimeItems<'a> {
    /// Remaining portion of the string.
    remainder: &'a str,
    /// If the current specifier is composed of multiple formatting items
    /// (e.g. `%+`), `queue` stores a slice of `Item`s to be returned one by
    /// one.
    queue: &'static [Item<'static>],
    lenient: bool,
    /// Remaining portion of a locale-specific format string (e.g. the
    /// locale's `%x` expansion), while it is being parsed.
    #[cfg(feature = "unstable-locales")]
    locale_str: &'a str,
    #[cfg(feature = "unstable-locales")]
    locale: Option<Locale>,
}

impl<'a> StrftimeItems<'a> {
    /// Creates a new parsing iterator from a `strftime`-like format string.
    ///
    /// While iterating, [`Item::Error`] is returned if the format string
    /// contains an invalid or unrecognized formatting specifier.
    #[must_use]
    pub const fn new(s: &'a str) -> StrftimeItems<'a> {
        StrftimeItems {
            remainder: s,
            queue: &[],
            lenient: false,
            #[cfg(feature = "unstable-locales")]
            locale_str: "",
            #[cfg(feature = "unstable-locales")]
            locale: None,
        }
    }

    /// The same as [`StrftimeItems::new`], but returns [`Item::Literal`]
    /// instead of [`Item::Error`].
    ///
    /// Useful for formatting according to potentially invalid format
    /// strings.
    #[must_use]
    pub const fn new_lenient(s: &'a str) -> StrftimeItems<'a> {
        StrftimeItems {
            remainder: s,
            queue: &[],
            lenient: true,
            #[cfg(feature = "unstable-locales")]
            locale_str: "",
            #[cfg(feature = "unstable-locales")]
            locale: None,
        }
    }

    /// Creates a new parsing iterator from a `strftime`-like format string,
    /// with `%x`/`%X`/`%c`/`%r` adjusted to the given [`Locale`](super::Locale).
    ///
    /// Note: this only localizes the *format*. Combine it with a
    /// `format_localized`/`format_localized_with_items` method (see
    /// [`NaiveDate`](crate::NaiveDate) or [`DateTime`](crate::DateTime)) to
    /// also get localized month/weekday names. `%r` falls back to the
    /// 24-hour `%X` format if the locale has no 12-hour clock format.
    #[cfg(feature = "unstable-locales")]
    #[must_use]
    pub const fn new_with_locale(s: &'a str, locale: Locale) -> StrftimeItems<'a> {
        StrftimeItems { remainder: s, queue: &[], lenient: false, locale_str: "", locale: Some(locale) }
    }

    /// Parses the format string into a `Vec` of formatting [`Item`]s.
    ///
    /// If you need to format or parse multiple values with the same format
    /// string, it is more efficient to convert it to a `Vec` of formatting
    /// [`Item`]s than to re-parse the format string on every use.
    pub fn parse(self) -> Result<Vec<Item<'a>>, ParseError> {
        self.into_iter()
            .map(|item| match item == Item::Error {
                false => Ok(item),
                true => Err(BAD_FORMAT),
            })
            .collect()
    }

    /// Parses the format string into a `Vec` of [`Item`]s that contain no
    /// references to slices of the format string.
    pub fn parse_to_owned(self) -> Result<Vec<Item<'static>>, ParseError> {
        self.into_iter()
            .map(|item| match item == Item::Error {
                false => Ok(item.to_owned()),
                true => Err(BAD_FORMAT),
            })
            .collect()
    }

    fn parse_next_item(&mut self, mut remainder: &'a str) -> Option<(&'a str, Item<'a>)> {
        use InternalInternal::*;
        use Item::{Literal, Space};
        use Numeric::*;

        let (original, mut remainder) = match remainder.chars().next()? {
            // the next item is a specifier
            '%' => (remainder, &remainder[1..]),

            // the next item is space
            c if c.is_whitespace() => {
                let nextspec =
                    remainder.find(|c: char| !c.is_whitespace()).unwrap_or(remainder.len());
                assert!(nextspec > 0);
                let item = Space(&remainder[..nextspec]);
                remainder = &remainder[nextspec..];
                return Some((remainder, item));
            }

            // the next item is literal
            _ => {
                let nextspec = remainder
                    .find(|c: char| c.is_whitespace() || c == '%')
                    .unwrap_or(remainder.len());
                assert!(nextspec > 0);
                let item = Literal(&remainder[..nextspec]);
                remainder = &remainder[nextspec..];
                return Some((remainder, item));
            }
        };

        macro_rules! next {
            () => {
                match remainder.chars().next() {
                    Some(x) => {
                        remainder = &remainder[x.len_utf8()..];
                        x
                    }
                    None => return Some((remainder, self.error(original, remainder))),
                }
            };
        }

        let spec = next!();
        let pad_override = match spec {
            '-' => Some(Pad::None),
            '0' => Some(Pad::Zero),
            '_' => Some(Pad::Space),
            _ => None,
        };

        let is_alternate = spec == '#';
        let spec = if pad_override.is_some() || is_alternate { next!() } else { spec };
        if is_alternate && !HAVE_ALTERNATES.contains(spec) {
            return Some((remainder, self.error(original, remainder)));
        }

        macro_rules! queue {
            [$head:expr, $($tail:expr),+ $(,)*] => ({
                const QUEUE: &'static [Item<'static>] = &[$($tail),+];
                self.queue = QUEUE;
                $head
            })
        }

        // Only used by the `%X`/`%c`/`%r`/`%x` arms below when
        // `unstable-locales` is disabled; those arms switch to
        // `switch_to_locale_str` instead when it's enabled, which would
        // otherwise leave this macro unused.
        #[cfg(not(feature = "unstable-locales"))]
        macro_rules! queue_from_slice {
            ($slice:expr) => {{
                self.queue = &$slice[1..];
                $slice[0].clone()
            }};
        }

        let item = match spec {
            'A' => fixed(Fixed::LongWeekdayName),
            'B' => fixed(Fixed::LongMonthName),
            'C' => num0(YearDiv100),
            'D' => {
                queue![num0(Month), Literal("/"), num0(Day), Literal("/"), num0(YearMod100)]
            }
            'F' => queue![num0(Year), Literal("-"), num0(Month), Literal("-"), num0(Day)],
            'G' => num0(IsoYear),
            'H' => num0(Hour),
            'I' => num0(Hour12),
            'M' => num0(Minute),
            'P' => fixed(Fixed::LowerAmPm),
            'R' => queue![num0(Hour), Literal(":"), num0(Minute)],
            'S' => num0(Second),
            'T' => {
                queue![num0(Hour), Literal(":"), num0(Minute), Literal(":"), num0(Second)]
            }
            'U' => num0(WeekFromSun),
            'V' => num0(IsoWeek),
            'W' => num0(WeekFromMon),
            #[cfg(not(feature = "unstable-locales"))]
            'X' => queue_from_slice!(T_FMT),
            #[cfg(feature = "unstable-locales")]
            'X' => self.switch_to_locale_str(locales::t_fmt, T_FMT),
            'Y' => num0(Year),
            'Z' => fixed(Fixed::TimezoneName),
            'a' => fixed(Fixed::ShortWeekdayName),
            'b' | 'h' => fixed(Fixed::ShortMonthName),
            #[cfg(not(feature = "unstable-locales"))]
            'c' => queue_from_slice!(D_T_FMT),
            #[cfg(feature = "unstable-locales")]
            'c' => self.switch_to_locale_str(locales::d_t_fmt, D_T_FMT),
            'd' => num0(Day),
            'e' => nums(Day),
            'f' => num0(Nanosecond),
            'g' => num0(IsoYearMod100),
            'j' => num0(Ordinal),
            'k' => nums(Hour),
            'l' => nums(Hour12),
            'm' => num0(Month),
            'n' => Space("\n"),
            'p' => fixed(Fixed::UpperAmPm),
            'q' => num(Quarter),
            #[cfg(not(feature = "unstable-locales"))]
            'r' => queue_from_slice!(T_FMT_AMPM),
            #[cfg(feature = "unstable-locales")]
            'r' => {
                if self.locale.is_some() && locales::t_fmt_ampm(self.locale.unwrap()).is_empty() {
                    // This locale has no 12-hour clock format; fall back to 24-hour.
                    self.switch_to_locale_str(locales::t_fmt, T_FMT)
                } else {
                    self.switch_to_locale_str(locales::t_fmt_ampm, T_FMT_AMPM)
                }
            }
            's' => num(Timestamp),
            't' => Space("\t"),
            'u' => num(WeekdayFromMon),
            'v' => {
                queue![
                    nums(Day),
                    Literal("-"),
                    fixed(Fixed::ShortMonthName),
                    Literal("-"),
                    num0(Year)
                ]
            }
            'w' => num(NumDaysFromSun),
            #[cfg(not(feature = "unstable-locales"))]
            'x' => queue_from_slice!(D_FMT),
            #[cfg(feature = "unstable-locales")]
            'x' => self.switch_to_locale_str(locales::d_fmt, D_FMT),
            'y' => num0(YearMod100),
            'z' => {
                if is_alternate {
                    internal_fixed(TimezoneOffsetPermissive)
                } else {
                    fixed(Fixed::TimezoneOffset)
                }
            }
            '+' => fixed(Fixed::RFC3339),
            ':' => {
                if remainder.starts_with("::z") {
                    remainder = &remainder[3..];
                    fixed(Fixed::TimezoneOffsetTripleColon)
                } else if remainder.starts_with(":z") {
                    remainder = &remainder[2..];
                    fixed(Fixed::TimezoneOffsetDoubleColon)
                } else if remainder.starts_with('z') {
                    remainder = &remainder[1..];
                    fixed(Fixed::TimezoneOffsetColon)
                } else {
                    self.error(original, remainder)
                }
            }
            '.' => match next!() {
                '3' => match next!() {
                    'f' => fixed(Fixed::Nanosecond3),
                    _ => self.error(original, remainder),
                },
                '6' => match next!() {
                    'f' => fixed(Fixed::Nanosecond6),
                    _ => self.error(original, remainder),
                },
                '9' => match next!() {
                    'f' => fixed(Fixed::Nanosecond9),
                    _ => self.error(original, remainder),
                },
                'f' => fixed(Fixed::Nanosecond),
                _ => self.error(original, remainder),
            },
            '3' => match next!() {
                'f' => internal_fixed(Nanosecond3NoDot),
                _ => self.error(original, remainder),
            },
            '6' => match next!() {
                'f' => internal_fixed(Nanosecond6NoDot),
                _ => self.error(original, remainder),
            },
            '9' => match next!() {
                'f' => internal_fixed(Nanosecond9NoDot),
                _ => self.error(original, remainder),
            },
            '%' => Literal("%"),
            _ => self.error(original, remainder),
        };

        // Adjust `item` if we have any padding modifier.
        // Not allowed on non-numeric items or on specifiers composed out of
        // multiple formatting items.
        if let Some(new_pad) = pad_override {
            match item {
                Item::Numeric(ref kind, _pad) if self.queue.is_empty() => {
                    Some((remainder, Item::Numeric(kind.clone(), new_pad)))
                }
                _ => Some((remainder, self.error(original, remainder))),
            }
        } else {
            Some((remainder, item))
        }
    }

    fn error<'b>(&mut self, original: &'b str, remainder: &'b str) -> Item<'b> {
        match self.lenient {
            false => Item::Error,
            true => Item::Literal(&original[..original.len() - remainder.len()]),
        }
    }

    /// Switches to parsing a locale-specific format string (e.g. the
    /// locale's `%x` expansion) if a locale was given, or falls back to the
    /// English/POSIX `fallback` items otherwise.
    #[cfg(feature = "unstable-locales")]
    fn switch_to_locale_str(
        &mut self,
        localized_fmt_str: impl Fn(Locale) -> &'static str,
        fallback: &'static [Item<'static>],
    ) -> Item<'a> {
        if let Some(locale) = self.locale {
            assert!(self.locale_str.is_empty());
            let (fmt_str, item) = self.parse_next_item(localized_fmt_str(locale)).unwrap();
            self.locale_str = fmt_str;
            item
        } else {
            self.queue = &fallback[1..];
            fallback[0].clone()
        }
    }
}

impl<'a> Iterator for StrftimeItems<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Item<'a>> {
        // We have items queued to return from a specifier composed of
        // multiple formatting items.
        if let Some((item, remainder)) = self.queue.split_first() {
            self.queue = remainder;
            return Some(item.clone());
        }

        // We are in the middle of parsing a locale-specific format string.
        #[cfg(feature = "unstable-locales")]
        if !self.locale_str.is_empty() {
            let (remainder, item) = self.parse_next_item(self.locale_str)?;
            self.locale_str = remainder;
            return Some(item);
        }

        // Normal: we are parsing the formatting string.
        let (remainder, item) = self.parse_next_item(self.remainder)?;
        self.remainder = remainder;
        Some(item)
    }
}

// `%x`: month-day-year (US form, no locale support).
static D_FMT: &[Item<'static>] = &[
    num0(Numeric::Month),
    Item::Literal("/"),
    num0(Numeric::Day),
    Item::Literal("/"),
    num0(Numeric::YearMod100),
];
// `%c`: same as `%a %b %e %H:%M:%S %Y`.
static D_T_FMT: &[Item<'static>] = &[
    fixed(Fixed::ShortWeekdayName),
    Item::Space(" "),
    fixed(Fixed::ShortMonthName),
    Item::Space(" "),
    nums(Numeric::Day),
    Item::Space(" "),
    num0(Numeric::Hour),
    Item::Literal(":"),
    num0(Numeric::Minute),
    Item::Literal(":"),
    num0(Numeric::Second),
    Item::Space(" "),
    num0(Numeric::Year),
];
// `%X`: same as `%T`.
static T_FMT: &[Item<'static>] = &[
    num0(Numeric::Hour),
    Item::Literal(":"),
    num0(Numeric::Minute),
    Item::Literal(":"),
    num0(Numeric::Second),
];
// `%r`: 12-hour clock time.
static T_FMT_AMPM: &[Item<'static>] = &[
    num0(Numeric::Hour12),
    Item::Literal(":"),
    num0(Numeric::Minute),
    Item::Literal(":"),
    num0(Numeric::Second),
    Item::Space(" "),
    fixed(Fixed::UpperAmPm),
];

const HAVE_ALTERNATES: &str = "z";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_and_space_segments_are_preserved() {
        let items = StrftimeItems::new("Hello  World").parse().unwrap();
        assert_eq!(items, vec![Item::Literal("Hello"), Item::Space("  "), Item::Literal("World")]);
    }

    #[test]
    fn simple_date_specifiers_map_to_expected_items() {
        assert_eq!(StrftimeItems::new("%Y").parse().unwrap(), vec![num0(Numeric::Year)]);
        assert_eq!(StrftimeItems::new("%m").parse().unwrap(), vec![num0(Numeric::Month)]);
        assert_eq!(StrftimeItems::new("%d").parse().unwrap(), vec![num0(Numeric::Day)]);
        assert_eq!(StrftimeItems::new("%e").parse().unwrap(), vec![nums(Numeric::Day)]);
        assert_eq!(StrftimeItems::new("%q").parse().unwrap(), vec![num(Numeric::Quarter)]);
        assert_eq!(StrftimeItems::new("%b").parse().unwrap(), vec![fixed(Fixed::ShortMonthName)]);
        assert_eq!(StrftimeItems::new("%h").parse().unwrap(), vec![fixed(Fixed::ShortMonthName)]);
        assert_eq!(StrftimeItems::new("%B").parse().unwrap(), vec![fixed(Fixed::LongMonthName)]);
        assert_eq!(StrftimeItems::new("%A").parse().unwrap(), vec![fixed(Fixed::LongWeekdayName)]);
        assert_eq!(StrftimeItems::new("%a").parse().unwrap(), vec![fixed(Fixed::ShortWeekdayName)]);
    }

    #[test]
    fn weekday_number_specifiers() {
        assert_eq!(StrftimeItems::new("%w").parse().unwrap(), vec![num(Numeric::NumDaysFromSun)]);
        assert_eq!(StrftimeItems::new("%u").parse().unwrap(), vec![num(Numeric::WeekdayFromMon)]);
    }

    #[test]
    fn composed_date_specifiers_expand_to_multiple_items() {
        assert_eq!(
            StrftimeItems::new("%F").parse().unwrap(),
            vec![
                num0(Numeric::Year),
                Item::Literal("-"),
                num0(Numeric::Month),
                Item::Literal("-"),
                num0(Numeric::Day)
            ]
        );
        assert_eq!(
            StrftimeItems::new("%D").parse().unwrap(),
            vec![
                num0(Numeric::Month),
                Item::Literal("/"),
                num0(Numeric::Day),
                Item::Literal("/"),
                num0(Numeric::YearMod100)
            ]
        );
        assert_eq!(
            StrftimeItems::new("%T").parse().unwrap(),
            vec![
                num0(Numeric::Hour),
                Item::Literal(":"),
                num0(Numeric::Minute),
                Item::Literal(":"),
                num0(Numeric::Second)
            ]
        );
        assert_eq!(
            StrftimeItems::new("%R").parse().unwrap(),
            vec![num0(Numeric::Hour), Item::Literal(":"), num0(Numeric::Minute)]
        );
        assert_eq!(
            StrftimeItems::new("%v").parse().unwrap(),
            vec![
                nums(Numeric::Day),
                Item::Literal("-"),
                fixed(Fixed::ShortMonthName),
                Item::Literal("-"),
                num0(Numeric::Year)
            ]
        );
    }

    #[test]
    fn x_c_r_specifiers_use_english_fallback_forms() {
        // `StrftimeItems::new` always leaves `locale` as `None`, so
        // `switch_to_locale_str` falls back to the same English/POSIX
        // items whether or not the `unstable-locales` feature is compiled
        // in -- these expectations hold either way.
        assert_eq!(
            StrftimeItems::new("%X").parse().unwrap(),
            vec![
                num0(Numeric::Hour),
                Item::Literal(":"),
                num0(Numeric::Minute),
                Item::Literal(":"),
                num0(Numeric::Second)
            ]
        );
        assert_eq!(
            StrftimeItems::new("%x").parse().unwrap(),
            vec![
                num0(Numeric::Month),
                Item::Literal("/"),
                num0(Numeric::Day),
                Item::Literal("/"),
                num0(Numeric::YearMod100)
            ]
        );
        assert_eq!(
            StrftimeItems::new("%r").parse().unwrap(),
            vec![
                num0(Numeric::Hour12),
                Item::Literal(":"),
                num0(Numeric::Minute),
                Item::Literal(":"),
                num0(Numeric::Second),
                Item::Space(" "),
                fixed(Fixed::UpperAmPm)
            ]
        );
        assert_eq!(
            StrftimeItems::new("%c").parse().unwrap(),
            vec![
                fixed(Fixed::ShortWeekdayName),
                Item::Space(" "),
                fixed(Fixed::ShortMonthName),
                Item::Space(" "),
                nums(Numeric::Day),
                Item::Space(" "),
                num0(Numeric::Hour),
                Item::Literal(":"),
                num0(Numeric::Minute),
                Item::Literal(":"),
                num0(Numeric::Second),
                Item::Space(" "),
                num0(Numeric::Year)
            ]
        );
    }

    #[test]
    fn padding_modifiers_override_the_default_pad() {
        assert_eq!(
            StrftimeItems::new("%-j").parse().unwrap(),
            vec![Item::Numeric(Numeric::Ordinal, Pad::None)]
        );
        assert_eq!(
            StrftimeItems::new("%_j").parse().unwrap(),
            vec![Item::Numeric(Numeric::Ordinal, Pad::Space)]
        );
        assert_eq!(
            StrftimeItems::new("%0e").parse().unwrap(),
            vec![Item::Numeric(Numeric::Day, Pad::Zero)]
        );
    }

    #[test]
    fn padding_modifier_on_composed_specifier_is_an_error() {
        assert_eq!(StrftimeItems::new("%-F").parse(), Err(BAD_FORMAT));
    }

    #[test]
    fn padding_modifier_on_non_numeric_specifier_is_an_error() {
        assert_eq!(StrftimeItems::new("%-A").parse(), Err(BAD_FORMAT));
    }

    #[test]
    fn nanosecond_fraction_specifiers() {
        assert_eq!(StrftimeItems::new("%.3f").parse().unwrap(), vec![fixed(Fixed::Nanosecond3)]);
        assert_eq!(StrftimeItems::new("%.6f").parse().unwrap(), vec![fixed(Fixed::Nanosecond6)]);
        assert_eq!(StrftimeItems::new("%.9f").parse().unwrap(), vec![fixed(Fixed::Nanosecond9)]);
        assert_eq!(StrftimeItems::new("%.f").parse().unwrap(), vec![fixed(Fixed::Nanosecond)]);
        assert_eq!(
            StrftimeItems::new("%3f").parse().unwrap(),
            vec![internal_fixed(InternalInternal::Nanosecond3NoDot)]
        );
        assert_eq!(
            StrftimeItems::new("%6f").parse().unwrap(),
            vec![internal_fixed(InternalInternal::Nanosecond6NoDot)]
        );
        assert_eq!(
            StrftimeItems::new("%9f").parse().unwrap(),
            vec![internal_fixed(InternalInternal::Nanosecond9NoDot)]
        );
    }

    #[test]
    fn nanosecond_fraction_specifier_rejects_bad_suffix() {
        assert_eq!(StrftimeItems::new("%.4f").parse(), Err(BAD_FORMAT));
    }

    #[test]
    fn timezone_specifiers() {
        assert_eq!(StrftimeItems::new("%z").parse().unwrap(), vec![fixed(Fixed::TimezoneOffset)]);
        assert_eq!(
            StrftimeItems::new("%:z").parse().unwrap(),
            vec![fixed(Fixed::TimezoneOffsetColon)]
        );
        assert_eq!(
            StrftimeItems::new("%::z").parse().unwrap(),
            vec![fixed(Fixed::TimezoneOffsetDoubleColon)]
        );
        assert_eq!(
            StrftimeItems::new("%:::z").parse().unwrap(),
            vec![fixed(Fixed::TimezoneOffsetTripleColon)]
        );
        assert_eq!(
            StrftimeItems::new("%#z").parse().unwrap(),
            vec![internal_fixed(InternalInternal::TimezoneOffsetPermissive)]
        );
        assert_eq!(StrftimeItems::new("%Z").parse().unwrap(), vec![fixed(Fixed::TimezoneName)]);
    }

    #[test]
    fn alternate_flag_only_supported_for_z() {
        assert_eq!(StrftimeItems::new("%#Y").parse(), Err(BAD_FORMAT));
    }

    #[test]
    fn rfc3339_and_timestamp_specifiers() {
        assert_eq!(StrftimeItems::new("%+").parse().unwrap(), vec![fixed(Fixed::RFC3339)]);
        assert_eq!(StrftimeItems::new("%s").parse().unwrap(), vec![num(Numeric::Timestamp)]);
    }

    #[test]
    fn special_specifiers() {
        assert_eq!(StrftimeItems::new("%t").parse().unwrap(), vec![Item::Space("\t")]);
        assert_eq!(StrftimeItems::new("%n").parse().unwrap(), vec![Item::Space("\n")]);
        assert_eq!(StrftimeItems::new("%%").parse().unwrap(), vec![Item::Literal("%")]);
    }

    #[test]
    fn unknown_specifier_is_a_bad_format_error() {
        assert_eq!(StrftimeItems::new("%Q").parse(), Err(BAD_FORMAT));
    }

    #[test]
    fn new_lenient_turns_errors_into_literals() {
        let items: Vec<Item> = StrftimeItems::new_lenient("%Q").collect();
        assert_eq!(items, vec![Item::Literal("%Q")]);
    }

    #[test]
    fn trailing_percent_with_no_specifier_is_an_error() {
        assert_eq!(StrftimeItems::new("abc%").parse(), Err(BAD_FORMAT));
    }

    #[test]
    fn parse_to_owned_produces_owned_items() {
        let owned = StrftimeItems::new("%Y-literal").parse_to_owned().unwrap();
        assert_eq!(owned[0], num0(Numeric::Year).to_owned());
        match &owned[1] {
            Item::OwnedLiteral(s) => assert_eq!(&**s, "-literal"),
            other => panic!("expected OwnedLiteral, got {other:?}"),
        }
    }
}
