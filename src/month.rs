//! `Month`: the month of the year as a standalone enum (distinct from the
//! plain `u32` returned by `Datelike::month()`).

use core::fmt;

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// The month of the year, `January` to `December`.
///
/// This is a convenience type: dates themselves store their month as a
/// plain `u32` (see [`Datelike::month`](crate::Datelike::month)). Convert
/// between the two with [`Month::try_from`] and
/// [`Month::number_from_month`].
///
/// API aligned with `chrono::Month`.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash, PartialOrd, Ord)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Month {
    January = 0,
    February = 1,
    March = 2,
    April = 3,
    May = 4,
    June = 5,
    July = 6,
    August = 7,
    September = 8,
    October = 9,
    November = 10,
    December = 11,
}

impl Month {
    /// The next month, wrapping from December to January.
    pub const fn succ(&self) -> Month {
        match *self {
            Month::January => Month::February,
            Month::February => Month::March,
            Month::March => Month::April,
            Month::April => Month::May,
            Month::May => Month::June,
            Month::June => Month::July,
            Month::July => Month::August,
            Month::August => Month::September,
            Month::September => Month::October,
            Month::October => Month::November,
            Month::November => Month::December,
            Month::December => Month::January,
        }
    }

    /// The previous month, wrapping from January to December.
    pub const fn pred(&self) -> Month {
        match *self {
            Month::January => Month::December,
            Month::February => Month::January,
            Month::March => Month::February,
            Month::April => Month::March,
            Month::May => Month::April,
            Month::June => Month::May,
            Month::July => Month::June,
            Month::August => Month::July,
            Month::September => Month::August,
            Month::October => Month::September,
            Month::November => Month::October,
            Month::December => Month::November,
        }
    }

    /// Month number starting from January = 1.
    pub const fn number_from_month(&self) -> u32 {
        match *self {
            Month::January => 1,
            Month::February => 2,
            Month::March => 3,
            Month::April => 4,
            Month::May => 5,
            Month::June => 6,
            Month::July => 7,
            Month::August => 8,
            Month::September => 9,
            Month::October => 10,
            Month::November => 11,
            Month::December => 12,
        }
    }

    /// Full English name, e.g. `"January"`.
    pub const fn name(&self) -> &'static str {
        match *self {
            Month::January => "January",
            Month::February => "February",
            Month::March => "March",
            Month::April => "April",
            Month::May => "May",
            Month::June => "June",
            Month::July => "July",
            Month::August => "August",
            Month::September => "September",
            Month::October => "October",
            Month::November => "November",
            Month::December => "December",
        }
    }

    const fn short_name(&self) -> &'static str {
        match *self {
            Month::January => "Jan",
            Month::February => "Feb",
            Month::March => "Mar",
            Month::April => "Apr",
            Month::May => "May",
            Month::June => "Jun",
            Month::July => "Jul",
            Month::August => "Aug",
            Month::September => "Sep",
            Month::October => "Oct",
            Month::November => "Nov",
            Month::December => "Dec",
        }
    }

    /// Number of days in this month for a given year. `None` only if
    /// `year` is out of range (checked only for February, which is the
    /// only month whose length depends on the year); other months always
    /// return `Some`, matching `chrono`.
    pub fn num_days(&self, year: i32) -> Option<u8> {
        Some(match *self {
            Month::January => 31,
            Month::February => {
                if crate::NaiveDate::from_ymd_opt(year, 2, 1)?.leap_year() {
                    29
                } else {
                    28
                }
            }
            Month::March => 31,
            Month::April => 30,
            Month::May => 31,
            Month::June => 30,
            Month::July => 31,
            Month::August => 31,
            Month::September => 30,
            Month::October => 31,
            Month::November => 30,
            Month::December => 31,
        })
    }
}

/// Error returned when converting an out-of-range number to a [`Month`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OutOfRange;

impl fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("out of range")
    }
}

impl TryFrom<u8> for Month {
    type Error = OutOfRange;

    fn try_from(value: u8) -> Result<Self, OutOfRange> {
        match value {
            1 => Ok(Month::January),
            2 => Ok(Month::February),
            3 => Ok(Month::March),
            4 => Ok(Month::April),
            5 => Ok(Month::May),
            6 => Ok(Month::June),
            7 => Ok(Month::July),
            8 => Ok(Month::August),
            9 => Ok(Month::September),
            10 => Ok(Month::October),
            11 => Ok(Month::November),
            12 => Ok(Month::December),
            _ => Err(OutOfRange),
        }
    }
}

/// Error returned when a string does not match any month name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ParseMonthError;

impl fmt::Display for ParseMonthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid month name")
    }
}

impl core::str::FromStr for Month {
    type Err = ParseMonthError;

    /// Accepts the short (3-letter) or full English name, case-insensitive
    /// (e.g. `"jan"`, `"Jan"` or `"January"`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const MONTHS: [Month; 12] = [
            Month::January,
            Month::February,
            Month::March,
            Month::April,
            Month::May,
            Month::June,
            Month::July,
            Month::August,
            Month::September,
            Month::October,
            Month::November,
            Month::December,
        ];
        for month in MONTHS {
            if s.eq_ignore_ascii_case(month.short_name()) || s.eq_ignore_ascii_case(month.name()) {
                return Ok(month);
            }
        }
        Err(ParseMonthError)
    }
}

/// `serde` support: serializes as the full English name (e.g. `"January"`),
/// deserializes short or long names case-insensitively (`FromStr`).
#[cfg(feature = "serde")]
mod serde_impl {
    use super::Month;
    use core::fmt;
    use serde::{de, ser};

    impl ser::Serialize for Month {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            serializer.collect_str(self.name())
        }
    }

    struct MonthVisitor;

    impl de::Visitor<'_> for MonthVisitor {
        type Value = Month;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("Month")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(|_| E::custom("short (3-letter) or full month names expected"))
        }
    }

    impl<'de> de::Deserialize<'de> for Month {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(MonthVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Month; 12] = [
        Month::January,
        Month::February,
        Month::March,
        Month::April,
        Month::May,
        Month::June,
        Month::July,
        Month::August,
        Month::September,
        Month::October,
        Month::November,
        Month::December,
    ];

    #[test]
    fn try_from_u8_accepts_1_through_12() {
        for n in 1u8..=12 {
            assert!(Month::try_from(n).is_ok(), "n={n}");
        }
    }

    #[test]
    fn try_from_u8_rejects_0_and_13_and_beyond() {
        assert_eq!(Month::try_from(0), Err(OutOfRange));
        assert_eq!(Month::try_from(13), Err(OutOfRange));
        assert_eq!(Month::try_from(255), Err(OutOfRange));
    }

    #[test]
    fn try_from_round_trips_with_number_from_month() {
        for n in 1u8..=12 {
            let month = Month::try_from(n).unwrap();
            assert_eq!(month.number_from_month(), n as u32);
        }
    }

    #[test]
    fn succ_wraps_from_december_to_january() {
        assert_eq!(Month::December.succ(), Month::January);
        assert_eq!(Month::January.succ(), Month::February);
    }

    #[test]
    fn pred_wraps_from_january_to_december() {
        assert_eq!(Month::January.pred(), Month::December);
        assert_eq!(Month::February.pred(), Month::January);
    }

    #[test]
    fn succ_and_pred_are_inverse_for_every_month() {
        for &month in &ALL {
            assert_eq!(month.succ().pred(), month);
            assert_eq!(month.pred().succ(), month);
        }
    }

    #[test]
    fn name_returns_full_english_name() {
        assert_eq!(Month::January.name(), "January");
        assert_eq!(Month::December.name(), "December");
    }

    #[test]
    fn from_str_accepts_short_and_long_names_case_insensitively() {
        assert_eq!("Jan".parse::<Month>().unwrap(), Month::January);
        assert_eq!("jan".parse::<Month>().unwrap(), Month::January);
        assert_eq!("JANUARY".parse::<Month>().unwrap(), Month::January);
        assert_eq!("December".parse::<Month>().unwrap(), Month::December);
        assert_eq!("dec".parse::<Month>().unwrap(), Month::December);
    }

    #[test]
    fn from_str_rejects_invalid_names() {
        assert!("Smarch".parse::<Month>().is_err());
        assert!("".parse::<Month>().is_err());
        assert!("Ja".parse::<Month>().is_err());
    }

    #[test]
    fn name_and_from_str_round_trip_for_every_month() {
        for &month in &ALL {
            assert_eq!(month.name().parse::<Month>().unwrap(), month);
        }
    }

    #[test]
    fn num_days_for_31_day_months() {
        for &month in
            &[Month::January, Month::March, Month::May, Month::July, Month::August, Month::October, Month::December]
        {
            assert_eq!(month.num_days(2023), Some(31), "{month:?}");
        }
    }

    #[test]
    fn num_days_for_30_day_months() {
        for &month in &[Month::April, Month::June, Month::September, Month::November] {
            assert_eq!(month.num_days(2023), Some(30), "{month:?}");
        }
    }

    #[test]
    fn num_days_for_february_leap_year_rules() {
        // Ordinary non-leap year.
        assert_eq!(Month::February.num_days(2023), Some(28));
        // Divisible by 4 -> leap.
        assert_eq!(Month::February.num_days(2024), Some(29));
        // Divisible by 100 but not 400 -> not leap.
        assert_eq!(Month::February.num_days(1900), Some(28));
        // Divisible by 400 -> leap.
        assert_eq!(Month::February.num_days(2000), Some(29));
        // Negative (proleptic) years follow the same rule.
        assert_eq!(Month::February.num_days(-400), Some(29));
        assert_eq!(Month::February.num_days(-401), Some(28));
    }

    #[test]
    fn ordering_follows_january_to_december_declaration_order() {
        for pair in ALL.windows(2) {
            assert!(pair[0] < pair[1], "{:?} should be < {:?}", pair[0], pair[1]);
        }
        assert_eq!(ALL.iter().min().copied(), Some(Month::January));
        assert_eq!(ALL.iter().max().copied(), Some(Month::December));
    }

    #[test]
    fn clone_copy_and_equality() {
        let month = Month::June;
        let cloned = month;
        assert_eq!(month, cloned);
        assert_ne!(Month::June, Month::July);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip_for_every_month() {
        for &month in &ALL {
            let json = serde_json::to_string(&month).unwrap();
            assert_eq!(json, format!("\"{}\"", month.name()));
            let back: Month = serde_json::from_str(&json).unwrap();
            assert_eq!(back, month);
        }
    }
}
