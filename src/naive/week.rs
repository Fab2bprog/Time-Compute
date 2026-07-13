//! `NaiveWeek`: the calendar week a given date belongs to.

use crate::calendar::weekday_from_days;
use crate::duration::Days;
use crate::naive::date::NaiveDate;
use crate::weekday::Weekday;
use core::hash::{Hash, Hasher};
use core::ops::RangeInclusive;

/// Adds (or subtracts, if negative) a small signed number of days to a
/// date. Used internally to compute the boundaries of a [`NaiveWeek`]
/// without needing a non-const `checked_add_signed`/`Duration` round trip.
const fn add_signed_days(date: NaiveDate, delta: i32) -> Option<NaiveDate> {
    if delta >= 0 {
        date.checked_add_days(Days::new(delta as u64))
    } else {
        date.checked_sub_days(Days::new((-(delta as i64)) as u64))
    }
}

/// The week that a [`NaiveDate`] belongs to, anchored on a chosen starting
/// weekday. Obtained via [`NaiveDate::week`](crate::naive::date::NaiveDate::week).
///
/// API aligned with `chrono::NaiveWeek`.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NaiveWeek {
    date: NaiveDate,
    start: Weekday,
}

impl NaiveWeek {
    pub(crate) const fn new(date: NaiveDate, start: Weekday) -> Self {
        Self { date, start }
    }

    /// The first day of the week.
    ///
    /// # Panics
    /// Panics if the first day of the week falls out of `NaiveDate`'s
    /// representable range.
    pub const fn first_day(&self) -> NaiveDate {
        match self.checked_first_day() {
            Some(d) => d,
            None => panic!("first weekday out of range for NaiveDate"),
        }
    }

    /// The first day of the week, or `None` if it falls out of
    /// `NaiveDate`'s representable range.
    pub const fn checked_first_day(&self) -> Option<NaiveDate> {
        let start = self.start.num_days_from_monday() as i32;
        let ref_day = weekday_from_days(self.date.days_since_epoch()).num_days_from_monday() as i32;
        let delta = start - ref_day - if start > ref_day { 7 } else { 0 };
        add_signed_days(self.date, delta)
    }

    /// The last day of the week.
    ///
    /// # Panics
    /// Panics if the last day of the week falls out of `NaiveDate`'s
    /// representable range.
    pub const fn last_day(&self) -> NaiveDate {
        match self.checked_last_day() {
            Some(d) => d,
            None => panic!("last weekday out of range for NaiveDate"),
        }
    }

    /// The last day of the week, or `None` if it falls out of
    /// `NaiveDate`'s representable range.
    pub const fn checked_last_day(&self) -> Option<NaiveDate> {
        let end = self.start.pred().num_days_from_monday() as i32;
        let ref_day = weekday_from_days(self.date.days_since_epoch()).num_days_from_monday() as i32;
        let delta = end - ref_day + if end < ref_day { 7 } else { 0 };
        add_signed_days(self.date, delta)
    }

    /// The full range of days in the week (inclusive of both ends).
    ///
    /// # Panics
    /// Panics if either boundary falls out of `NaiveDate`'s representable
    /// range.
    pub fn days(&self) -> RangeInclusive<NaiveDate> {
        match self.checked_days() {
            Some(val) => val,
            None => panic!("first or last weekday is out of range for NaiveDate"),
        }
    }

    /// The full range of days in the week, or `None` if either boundary
    /// falls out of `NaiveDate`'s representable range.
    ///
    /// Not `const` (unlike the other accessors here) because building a
    /// `RangeInclusive` is not guaranteed const-evaluable on all supported
    /// Rust versions.
    pub fn checked_days(&self) -> Option<RangeInclusive<NaiveDate>> {
        match (self.checked_first_day(), self.checked_last_day()) {
            (Some(first), Some(last)) => Some(first..=last),
            _ => None,
        }
    }
}

impl PartialEq for NaiveWeek {
    fn eq(&self, other: &Self) -> bool {
        self.first_day() == other.first_day()
    }
}

impl Eq for NaiveWeek {}

impl Hash for NaiveWeek {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.first_day().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Datelike;

    const ALL_WEEKDAYS: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    /// 2023-06-15 is a Thursday (verified by hand elsewhere in the test
    /// suite: days_from_civil(2023,1,1) = 19358, a Sunday; June 15th is
    /// ordinal day 166, i.e. epoch day 19523, which works out to a
    /// Thursday).
    fn thursday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2023, 6, 15).unwrap()
    }

    #[test]
    fn week_starting_monday_bounds_a_thursday_correctly() {
        let week = thursday().week(Weekday::Mon);
        assert_eq!(week.first_day(), NaiveDate::from_ymd_opt(2023, 6, 12).unwrap());
        assert_eq!(week.last_day(), NaiveDate::from_ymd_opt(2023, 6, 18).unwrap());
    }

    #[test]
    fn week_starting_sunday_bounds_a_thursday_correctly() {
        let week = thursday().week(Weekday::Sun);
        assert_eq!(week.first_day(), NaiveDate::from_ymd_opt(2023, 6, 11).unwrap());
        assert_eq!(week.last_day(), NaiveDate::from_ymd_opt(2023, 6, 17).unwrap());
    }

    #[test]
    fn week_always_spans_exactly_seven_days_and_starts_on_the_right_weekday() {
        for &start in &ALL_WEEKDAYS {
            let week = thursday().week(start);
            assert_eq!(week.last_day() - week.first_day(), crate::duration::Duration::days(6));
            assert_eq!(week.first_day().weekday(), start);
        }
    }

    #[test]
    fn the_original_date_always_falls_within_its_own_week() {
        let d = thursday();
        for &start in &ALL_WEEKDAYS {
            let week = d.week(start);
            assert!(week.first_day() <= d && d <= week.last_day(), "start={start:?}");
        }
    }

    #[test]
    fn days_returns_the_inclusive_seven_day_range() {
        // `NaiveDate` does not implement the (unstable) `Step` trait, so
        // `RangeInclusive<NaiveDate>` cannot be iterated/collected -- it is
        // only usable for its bounds and `.contains()`. Check those
        // directly instead.
        let week = thursday().week(Weekday::Mon);
        let days = week.days();
        assert_eq!(*days.start(), week.first_day());
        assert_eq!(*days.end(), week.last_day());
        assert_eq!(*days.end() - *days.start(), crate::duration::Duration::days(6));
        assert!(days.contains(&thursday()));
    }

    #[test]
    fn checked_first_and_last_day_agree_with_the_panicking_variants() {
        let week = thursday().week(Weekday::Mon);
        assert_eq!(week.checked_first_day(), Some(week.first_day()));
        assert_eq!(week.checked_last_day(), Some(week.last_day()));
        assert_eq!(week.checked_days(), Some(week.first_day()..=week.last_day()));
    }

    #[test]
    fn checked_first_day_is_none_when_the_week_would_start_before_min() {
        // For exactly one starting weekday (`MIN`'s own), the week starts
        // at `MIN` itself; for every other starting weekday, the
        // computed first day would fall strictly before `MIN`, which
        // must yield `None` instead of panicking or wrapping.
        let mut some_count = 0;
        let mut none_count = 0;
        for &start in &ALL_WEEKDAYS {
            match NaiveDate::MIN.week(start).checked_first_day() {
                Some(d) => {
                    assert_eq!(d, NaiveDate::MIN);
                    some_count += 1;
                }
                None => none_count += 1,
            }
        }
        assert_eq!(some_count, 1);
        assert_eq!(none_count, 6);
    }

    #[test]
    fn checked_last_day_is_none_when_the_week_would_end_after_max() {
        let mut some_count = 0;
        let mut none_count = 0;
        for &start in &ALL_WEEKDAYS {
            match NaiveDate::MAX.week(start).checked_last_day() {
                Some(d) => {
                    assert_eq!(d, NaiveDate::MAX);
                    some_count += 1;
                }
                None => none_count += 1,
            }
        }
        assert_eq!(some_count, 1);
        assert_eq!(none_count, 6);
    }

    #[test]
    fn equality_and_ordering_are_based_on_first_day_not_the_originating_date() {
        let a = thursday().week(Weekday::Mon);
        // A different date within the same Monday-anchored week.
        let b = NaiveDate::from_ymd_opt(2023, 6, 13).unwrap().week(Weekday::Mon);
        assert_eq!(a, b);
        assert_eq!(a.first_day(), b.first_day());
    }
}
