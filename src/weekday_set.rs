//! [`WeekdaySet`]: a compact, `Copy` set of [`Weekday`]s.

use core::fmt::{self, Debug};
use core::iter::FusedIterator;

use crate::weekday::Weekday;

/// A set of [`Weekday`]s, packed into a single byte.
///
/// Bits 0 through 6 (from the least significant bit) correspond to
/// Monday through Sunday; the 8th bit is always zero. `WeekdaySet` is
/// `Copy` and most of its operations are `const fn`.
///
/// API aligned with `chrono::WeekdaySet`.
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct WeekdaySet(u8);

impl WeekdaySet {
    /// An empty set.
    pub const EMPTY: Self = Self(0b000_0000);

    /// The set containing all seven weekdays.
    pub const ALL: Self = Self(0b111_1111);

    /// Builds a set from an array of weekdays (duplicates are fine).
    pub const fn from_array<const C: usize>(days: [Weekday; C]) -> Self {
        let mut acc = Self::EMPTY;
        let mut idx = 0;
        while idx < days.len() {
            acc.0 |= Self::single(days[idx]).0;
            idx += 1;
        }
        acc
    }

    /// Builds a set containing a single weekday.
    pub const fn single(weekday: Weekday) -> Self {
        match weekday {
            Weekday::Mon => Self(0b000_0001),
            Weekday::Tue => Self(0b000_0010),
            Weekday::Wed => Self(0b000_0100),
            Weekday::Thu => Self(0b000_1000),
            Weekday::Fri => Self(0b001_0000),
            Weekday::Sat => Self(0b010_0000),
            Weekday::Sun => Self(0b100_0000),
        }
    }

    /// Returns the single weekday in this set, or `None` if the set is
    /// empty or holds more than one day.
    pub const fn single_day(self) -> Option<Weekday> {
        match self {
            Self(0b000_0001) => Some(Weekday::Mon),
            Self(0b000_0010) => Some(Weekday::Tue),
            Self(0b000_0100) => Some(Weekday::Wed),
            Self(0b000_1000) => Some(Weekday::Thu),
            Self(0b001_0000) => Some(Weekday::Fri),
            Self(0b010_0000) => Some(Weekday::Sat),
            Self(0b100_0000) => Some(Weekday::Sun),
            _ => None,
        }
    }

    /// Adds `day` to the set. Returns `true` if it was not already present.
    pub fn insert(&mut self, day: Weekday) -> bool {
        if self.contains(day) {
            return false;
        }
        self.0 |= Self::single(day).0;
        true
    }

    /// Removes `day` from the set. Returns `true` if it was present.
    pub fn remove(&mut self, day: Weekday) -> bool {
        if !self.contains(day) {
            return false;
        }
        self.0 &= !Self::single(day).0;
        true
    }

    /// Returns `true` if every day of `self` is also in `other`.
    pub const fn is_subset(self, other: Self) -> bool {
        self.intersection(other).0 == self.0
    }

    /// Returns the days present in both `self` and `other`.
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Returns the days present in `self`, `other`, or both.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns the days present in exactly one of `self` and `other`.
    pub const fn symmetric_difference(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }

    /// Returns the days present in `self` but not in `other`.
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Returns the earliest day in the set, starting the count at Monday.
    pub const fn first(self) -> Option<Weekday> {
        if self.is_empty() {
            return None;
        }
        let bit = 1 << self.0.trailing_zeros();
        Self(bit).single_day()
    }

    /// Returns the latest day in the set, starting the count at Monday
    /// (so this is the day closest to Sunday).
    pub fn last(self) -> Option<Weekday> {
        if self.is_empty() {
            return None;
        }
        let bit = 1 << (7 - self.0.leading_zeros());
        Self(bit).single_day()
    }

    /// Splits the set at `weekday`: the first half holds the days strictly
    /// before `weekday` (starting from Monday), the second half holds
    /// `weekday` itself and every day after it, up to Sunday.
    const fn split_at(self, weekday: Weekday) -> (Self, Self) {
        let from_weekday_on = 0b1000_0000 - Self::single(weekday).0;
        let before_weekday = from_weekday_on ^ 0b0111_1111;
        (Self(self.0 & before_weekday), Self(self.0 & from_weekday_on))
    }

    /// Iterates over the days in the set starting at `start` and wrapping
    /// around the week (Sunday is followed by Monday).
    pub const fn iter(self, start: Weekday) -> WeekdaySetIter {
        WeekdaySetIter { days: self, start }
    }

    /// Returns `true` if the set contains `day`.
    pub const fn contains(self, day: Weekday) -> bool {
        self.0 & Self::single(day).0 != 0
    }

    /// Returns `true` if the set has no days.
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Returns the number of days in the set (0 to 7).
    pub const fn len(self) -> u8 {
        self.0.count_ones() as u8
    }
}

/// Prints the raw 7-bit mask, e.g. `WeekdaySet(0000001)` for a set holding
/// only Monday.
impl Debug for WeekdaySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WeekdaySet({:0>7b})", self.0)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for WeekdaySet {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(
            f,
            "WeekdaySet({}{}{}{}{}{}{})",
            0x1 & (self.0 >> 6),
            0x1 & (self.0 >> 5),
            0x1 & (self.0 >> 4),
            0x1 & (self.0 >> 3),
            0x1 & (self.0 >> 2),
            0x1 & (self.0 >> 1),
            0x1 & (self.0 >> 0),
        )
    }
}

/// Prints the set as a bracketed, comma-separated list of weekdays in
/// Monday-first order, e.g. `[Mon, Fri, Sun]`.
impl fmt::Display for WeekdaySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut iter = self.iter(Weekday::Mon);
        if let Some(first) = iter.next() {
            write!(f, "{first}")?;
        }
        for weekday in iter {
            write!(f, ", {weekday}")?;
        }
        write!(f, "]")
    }
}

impl FromIterator<Weekday> for WeekdaySet {
    fn from_iter<T: IntoIterator<Item = Weekday>>(iter: T) -> Self {
        iter.into_iter().map(Self::single).fold(Self::EMPTY, Self::union)
    }
}

/// Iterator over the [`Weekday`]s of a [`WeekdaySet`], produced by
/// [`WeekdaySet::iter`].
#[derive(Debug, Clone)]
pub struct WeekdaySetIter {
    days: WeekdaySet,
    start: Weekday,
}

impl Iterator for WeekdaySetIter {
    type Item = Weekday;

    fn next(&mut self) -> Option<Self::Item> {
        if self.days.is_empty() {
            return None;
        }
        // Days from `start` onward take priority; once exhausted, wrap
        // around to the days before `start`.
        let (before, from_start_on) = self.days.split_at(self.start);
        let days = if from_start_on.is_empty() { before } else { from_start_on };
        let next = days.first().expect("checked non-empty above");
        self.days.remove(next);
        Some(next)
    }
}

impl DoubleEndedIterator for WeekdaySetIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.days.is_empty() {
            return None;
        }
        // From the back, days strictly before `start` take priority; once
        // exhausted, wrap around to `start` and the days after it.
        let (before, from_start_on) = self.days.split_at(self.start);
        let days = if before.is_empty() { from_start_on } else { before };
        let next = days.last().expect("checked non-empty above");
        self.days.remove(next);
        Some(next)
    }
}

impl ExactSizeIterator for WeekdaySetIter {
    fn len(&self) -> usize {
        self.days.len() as usize
    }
}

impl FusedIterator for WeekdaySetIter {}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_WEEKDAYS: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    #[test]
    fn empty_and_all_constants() {
        assert!(WeekdaySet::EMPTY.is_empty());
        assert_eq!(WeekdaySet::EMPTY.len(), 0);
        assert_eq!(WeekdaySet::ALL.len(), 7);
        assert!(!WeekdaySet::ALL.is_empty());
        for day in ALL_WEEKDAYS {
            assert!(WeekdaySet::ALL.contains(day));
            assert!(!WeekdaySet::EMPTY.contains(day));
        }
    }

    #[test]
    fn default_is_empty() {
        assert_eq!(WeekdaySet::default(), WeekdaySet::EMPTY);
    }

    #[test]
    fn single_and_single_day_round_trip() {
        for day in ALL_WEEKDAYS {
            let set = WeekdaySet::single(day);
            assert_eq!(set.len(), 1);
            assert_eq!(set.single_day(), Some(day));
            assert!(set.contains(day));
        }
        assert_eq!(WeekdaySet::EMPTY.single_day(), None);
        assert_eq!(WeekdaySet::ALL.single_day(), None);
    }

    #[test]
    fn from_array_and_from_iterator_agree() {
        let arr = WeekdaySet::from_array([Weekday::Mon, Weekday::Wed, Weekday::Mon]); // duplicate is fine
        assert_eq!(arr.len(), 2);
        assert!(arr.contains(Weekday::Mon));
        assert!(arr.contains(Weekday::Wed));

        let from_iter: WeekdaySet = [Weekday::Mon, Weekday::Wed].into_iter().collect();
        assert_eq!(from_iter, arr);
    }

    #[test]
    fn insert_and_remove_report_whether_the_day_changed() {
        let mut set = WeekdaySet::EMPTY;
        assert!(set.insert(Weekday::Mon)); // newly inserted
        assert!(!set.insert(Weekday::Mon)); // already present
        assert_eq!(set.len(), 1);

        assert!(set.remove(Weekday::Mon)); // was present
        assert!(!set.remove(Weekday::Mon)); // already absent
        assert!(set.is_empty());
    }

    #[test]
    fn set_operations() {
        let a = WeekdaySet::from_array([Weekday::Mon, Weekday::Tue, Weekday::Wed]);
        let b = WeekdaySet::from_array([Weekday::Tue, Weekday::Wed, Weekday::Thu]);

        assert_eq!(a.intersection(b), WeekdaySet::from_array([Weekday::Tue, Weekday::Wed]));
        assert_eq!(
            a.union(b),
            WeekdaySet::from_array([Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu])
        );
        assert_eq!(a.symmetric_difference(b), WeekdaySet::from_array([Weekday::Mon, Weekday::Thu]));
        assert_eq!(a.difference(b), WeekdaySet::from_array([Weekday::Mon]));

        assert!(WeekdaySet::from_array([Weekday::Mon]).is_subset(a));
        assert!(!a.is_subset(WeekdaySet::from_array([Weekday::Mon])));
    }

    #[test]
    fn first_and_last() {
        let set = WeekdaySet::from_array([Weekday::Wed, Weekday::Fri, Weekday::Mon]);
        assert_eq!(set.first(), Some(Weekday::Mon));
        assert_eq!(set.last(), Some(Weekday::Fri));
        assert_eq!(WeekdaySet::EMPTY.first(), None);
        assert_eq!(WeekdaySet::EMPTY.last(), None);
    }

    #[test]
    fn iter_starts_at_the_given_weekday_and_wraps() {
        let set = WeekdaySet::from_array([Weekday::Mon, Weekday::Wed, Weekday::Fri]);
        let collected: Vec<Weekday> = set.iter(Weekday::Wed).collect();
        assert_eq!(collected, vec![Weekday::Wed, Weekday::Fri, Weekday::Mon]);
    }

    #[test]
    fn iter_is_double_ended_and_exact_size() {
        let set = WeekdaySet::from_array([Weekday::Mon, Weekday::Wed, Weekday::Fri]);
        let mut iter = set.iter(Weekday::Mon);
        assert_eq!(iter.len(), 3);
        assert_eq!(iter.next(), Some(Weekday::Mon));
        assert_eq!(iter.next_back(), Some(Weekday::Fri));
        assert_eq!(iter.next(), Some(Weekday::Wed));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn debug_prints_the_raw_bit_mask() {
        assert_eq!(format!("{:?}", WeekdaySet::single(Weekday::Mon)), "WeekdaySet(0000001)");
        assert_eq!(format!("{:?}", WeekdaySet::single(Weekday::Sun)), "WeekdaySet(1000000)");
        assert_eq!(format!("{:?}", WeekdaySet::EMPTY), "WeekdaySet(0000000)");
    }

    #[test]
    fn display_lists_days_in_monday_first_order_regardless_of_insertion_order() {
        let set = WeekdaySet::from_array([Weekday::Sun, Weekday::Mon, Weekday::Fri]);
        assert_eq!(set.to_string(), "[Mon, Fri, Sun]");
        assert_eq!(WeekdaySet::EMPTY.to_string(), "[]");
    }
}
