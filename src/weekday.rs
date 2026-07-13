//! Day of the week.

use core::fmt;

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// A day of the week, from Monday to Sunday (ISO 8601 convention: the week
/// starts on Monday).
///
/// API aligned with `chrono::Weekday`: same variants, same methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, RkyvDeserialize, RkyvSerialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    /// Builds a `Weekday` from a 0..=6 index where 0 = Monday.
    pub(crate) const fn from_num_days_from_monday(n: u32) -> Self {
        match n % 7 {
            0 => Weekday::Mon,
            1 => Weekday::Tue,
            2 => Weekday::Wed,
            3 => Weekday::Thu,
            4 => Weekday::Fri,
            5 => Weekday::Sat,
            _ => Weekday::Sun,
        }
    }

    /// The next day (wraps around after Sunday).
    pub const fn succ(&self) -> Weekday {
        match self {
            Weekday::Mon => Weekday::Tue,
            Weekday::Tue => Weekday::Wed,
            Weekday::Wed => Weekday::Thu,
            Weekday::Thu => Weekday::Fri,
            Weekday::Fri => Weekday::Sat,
            Weekday::Sat => Weekday::Sun,
            Weekday::Sun => Weekday::Mon,
        }
    }

    /// The previous day (wraps around before Monday).
    pub const fn pred(&self) -> Weekday {
        match self {
            Weekday::Mon => Weekday::Sun,
            Weekday::Tue => Weekday::Mon,
            Weekday::Wed => Weekday::Tue,
            Weekday::Thu => Weekday::Wed,
            Weekday::Fri => Weekday::Thu,
            Weekday::Sat => Weekday::Fri,
            Weekday::Sun => Weekday::Sat,
        }
    }

    /// Day number counting from Monday = 0.
    pub const fn num_days_from_monday(&self) -> u32 {
        match self {
            Weekday::Mon => 0,
            Weekday::Tue => 1,
            Weekday::Wed => 2,
            Weekday::Thu => 3,
            Weekday::Fri => 4,
            Weekday::Sat => 5,
            Weekday::Sun => 6,
        }
    }

    /// Day number counting from Sunday = 0.
    pub const fn num_days_from_sunday(&self) -> u32 {
        match self {
            Weekday::Sun => 0,
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
        }
    }

    /// Day number counting from Monday = 1 (ISO 8601 convention).
    pub const fn number_from_monday(&self) -> u32 {
        self.num_days_from_monday() + 1
    }

    /// Day number counting from Sunday = 1.
    pub const fn number_from_sunday(&self) -> u32 {
        self.num_days_from_sunday() + 1
    }

    /// The number of days since the previous occurrence (or the same day, if
    /// equal) of `other`. Always in `0..7`.
    pub const fn days_since(&self, other: Weekday) -> u32 {
        let lhs = *self as u32;
        let rhs = other as u32;
        if lhs < rhs { 7 + lhs - rhs } else { lhs - rhs }
    }

    const fn short_name(&self) -> &'static str {
        match self {
            Weekday::Mon => "Mon",
            Weekday::Tue => "Tue",
            Weekday::Wed => "Wed",
            Weekday::Thu => "Thu",
            Weekday::Fri => "Fri",
            Weekday::Sat => "Sat",
            Weekday::Sun => "Sun",
        }
    }

    const fn long_name(&self) -> &'static str {
        match self {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        }
    }
}

/// Prints the 3-letter English abbreviation, like `chrono::Weekday` (e.g.
/// "Mon", "Tue"), to stay compatible with the `%a` format specifier.
impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.short_name())
    }
}

/// Error returned when a string does not match any weekday name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ParseWeekdayError;

impl fmt::Display for ParseWeekdayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid weekday name")
    }
}

impl core::str::FromStr for Weekday {
    type Err = ParseWeekdayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const DAYS: [Weekday; 7] = [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ];
        for day in DAYS {
            if s.eq_ignore_ascii_case(day.short_name()) || s.eq_ignore_ascii_case(day.long_name()) {
                return Ok(day);
            }
        }
        Err(ParseWeekdayError)
    }
}

/// `serde` support: serializes as the 3-letter English name (`Display`),
/// deserializes short or long names case-insensitively (`FromStr`).
#[cfg(feature = "serde")]
mod serde_impl {
    use super::Weekday;
    use core::fmt;
    use serde::{de, ser};

    impl ser::Serialize for Weekday {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            serializer.collect_str(self)
        }
    }

    struct WeekdayVisitor;

    impl de::Visitor<'_> for WeekdayVisitor {
        type Value = Weekday;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("Weekday")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(|_| E::custom("short or long weekday names expected"))
        }
    }

    impl<'de> de::Deserialize<'de> for Weekday {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(WeekdayVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    #[test]
    fn succ_wraps_from_sunday_to_monday() {
        assert_eq!(Weekday::Sun.succ(), Weekday::Mon);
        assert_eq!(Weekday::Mon.succ(), Weekday::Tue);
        assert_eq!(Weekday::Sat.succ(), Weekday::Sun);
    }

    #[test]
    fn pred_wraps_from_monday_to_sunday() {
        assert_eq!(Weekday::Mon.pred(), Weekday::Sun);
        assert_eq!(Weekday::Tue.pred(), Weekday::Mon);
        assert_eq!(Weekday::Sun.pred(), Weekday::Sat);
    }

    #[test]
    fn succ_and_pred_are_inverse_for_every_day() {
        for &day in &ALL {
            assert_eq!(day.succ().pred(), day);
            assert_eq!(day.pred().succ(), day);
        }
    }

    #[test]
    fn num_days_from_monday_is_0_indexed_starting_monday() {
        let expected = [0u32, 1, 2, 3, 4, 5, 6];
        for (day, &exp) in ALL.iter().zip(expected.iter()) {
            assert_eq!(day.num_days_from_monday(), exp, "{day:?}");
        }
    }

    #[test]
    fn num_days_from_sunday_is_0_indexed_starting_sunday() {
        assert_eq!(Weekday::Sun.num_days_from_sunday(), 0);
        assert_eq!(Weekday::Mon.num_days_from_sunday(), 1);
        assert_eq!(Weekday::Sat.num_days_from_sunday(), 6);
    }

    #[test]
    fn number_from_monday_and_sunday_are_1_indexed() {
        assert_eq!(Weekday::Mon.number_from_monday(), 1);
        assert_eq!(Weekday::Sun.number_from_monday(), 7);
        assert_eq!(Weekday::Sun.number_from_sunday(), 1);
        assert_eq!(Weekday::Sat.number_from_sunday(), 7);
    }

    #[test]
    fn days_since_same_day_is_zero() {
        for &day in &ALL {
            assert_eq!(day.days_since(day), 0);
        }
    }

    #[test]
    fn days_since_typical_and_wraparound_cases() {
        // Wednesday is 2 days after Monday.
        assert_eq!(Weekday::Wed.days_since(Weekday::Mon), 2);
        // Monday is 6 days after Tuesday (wraps around the week).
        assert_eq!(Weekday::Mon.days_since(Weekday::Tue), 6);
        // Sunday is 6 days after Monday.
        assert_eq!(Weekday::Sun.days_since(Weekday::Mon), 6);
    }

    #[test]
    fn from_num_days_from_monday_round_trips_and_wraps_modulo_7() {
        for (n, &day) in ALL.iter().enumerate() {
            assert_eq!(Weekday::from_num_days_from_monday(n as u32), day);
            // Adding a multiple of 7 must land on the same day.
            assert_eq!(Weekday::from_num_days_from_monday(n as u32 + 7), day);
            assert_eq!(Weekday::from_num_days_from_monday(n as u32 + 70), day);
        }
    }

    #[test]
    fn display_prints_three_letter_english_abbreviation() {
        let expected = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (day, &exp) in ALL.iter().zip(expected.iter()) {
            assert_eq!(day.to_string(), exp);
        }
    }

    #[test]
    fn from_str_accepts_short_and_long_names_case_insensitively() {
        assert_eq!("Mon".parse::<Weekday>().unwrap(), Weekday::Mon);
        assert_eq!("mon".parse::<Weekday>().unwrap(), Weekday::Mon);
        assert_eq!("MON".parse::<Weekday>().unwrap(), Weekday::Mon);
        assert_eq!("Monday".parse::<Weekday>().unwrap(), Weekday::Mon);
        assert_eq!("monday".parse::<Weekday>().unwrap(), Weekday::Mon);
        assert_eq!("Sunday".parse::<Weekday>().unwrap(), Weekday::Sun);
    }

    #[test]
    fn from_str_rejects_invalid_names() {
        assert!("Frog".parse::<Weekday>().is_err());
        assert!("".parse::<Weekday>().is_err());
        assert!("Mo".parse::<Weekday>().is_err());
    }

    #[test]
    fn display_and_from_str_round_trip_for_every_day() {
        for &day in &ALL {
            let s = day.to_string();
            assert_eq!(s.parse::<Weekday>().unwrap(), day);
        }
    }

    #[test]
    fn ordering_follows_monday_to_sunday_declaration_order() {
        for pair in ALL.windows(2) {
            assert!(pair[0] < pair[1], "{:?} should be < {:?}", pair[0], pair[1]);
        }
        assert_eq!(ALL.iter().min().copied(), Some(Weekday::Mon));
        assert_eq!(ALL.iter().max().copied(), Some(Weekday::Sun));
    }

    #[test]
    fn clone_copy_and_equality() {
        let day = Weekday::Thu;
        let cloned = day;
        assert_eq!(day, cloned);
        assert_ne!(Weekday::Thu, Weekday::Fri);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip_for_every_day() {
        for &day in &ALL {
            let json = serde_json::to_string(&day).unwrap();
            let back: Weekday = serde_json::from_str(&json).unwrap();
            assert_eq!(back, day);
        }
    }
}
