//! Iterators over `NaiveDate` ranges, stepping by day or by week.

use crate::duration::Days;
use crate::naive::date::NaiveDate;
use core::iter::FusedIterator;

/// An iterator over `NaiveDate`, advancing one day at a time.
///
/// Created by [`NaiveDate::iter_days`](crate::naive::date::NaiveDate::iter_days).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NaiveDateDaysIterator {
    pub(crate) value: NaiveDate,
}

impl Iterator for NaiveDateDaysIterator {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<NaiveDate> {
        let current = self.value;
        // `succ_opt` only fails once `current` is `NaiveDate::MAX`, in
        // which case the iterator simply stops.
        self.value = current.succ_opt()?;
        Some(current)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = NaiveDate::MAX.signed_duration_since(self.value).num_days();
        (remaining as usize, Some(remaining as usize))
    }
}

impl ExactSizeIterator for NaiveDateDaysIterator {}

impl DoubleEndedIterator for NaiveDateDaysIterator {
    fn next_back(&mut self) -> Option<NaiveDate> {
        let current = self.value;
        self.value = current.pred_opt()?;
        Some(current)
    }
}

impl FusedIterator for NaiveDateDaysIterator {}

/// An iterator over `NaiveDate`, advancing one week (7 days) at a time.
///
/// Created by [`NaiveDate::iter_weeks`](crate::naive::date::NaiveDate::iter_weeks).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NaiveDateWeeksIterator {
    pub(crate) value: NaiveDate,
}

impl Iterator for NaiveDateWeeksIterator {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<NaiveDate> {
        let current = self.value;
        self.value = current.checked_add_days(Days::new(7))?;
        Some(current)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = NaiveDate::MAX.signed_duration_since(self.value).num_weeks();
        (remaining as usize, Some(remaining as usize))
    }
}

impl ExactSizeIterator for NaiveDateWeeksIterator {}

impl DoubleEndedIterator for NaiveDateWeeksIterator {
    fn next_back(&mut self) -> Option<NaiveDate> {
        let current = self.value;
        self.value = current.checked_sub_days(Days::new(7))?;
        Some(current)
    }
}

impl FusedIterator for NaiveDateWeeksIterator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_iterator_advances_one_day_at_a_time() {
        let start = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let days: Vec<_> = start.iter_days().take(3).collect();
        assert_eq!(
            days,
            vec![
                start,
                NaiveDate::from_ymd_opt(2023, 6, 16).unwrap(),
                NaiveDate::from_ymd_opt(2023, 6, 17).unwrap(),
            ]
        );
    }

    #[test]
    fn days_iterator_next_back_goes_backward() {
        let start = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let mut it = start.iter_days();
        assert_eq!(it.next_back(), Some(start));
        assert_eq!(it.next_back(), Some(NaiveDate::from_ymd_opt(2023, 6, 14).unwrap()));
    }

    #[test]
    fn days_iterator_never_yields_max_when_starting_there() {
        // Subtle quirk of the `current.succ_opt()?` short-circuit: when
        // `succ_opt()` returns `None` (only true when `current == MAX`),
        // the `?` operator returns `None` from `next()` immediately,
        // *before* reaching `Some(current)`. So an iterator whose value is
        // already `NaiveDate::MAX` yields nothing at all, not even `MAX`
        // itself. This mirrors chrono's own iterator, not a bug introduced
        // here -- verified by tracing the code by hand.
        let mut it = NaiveDate::MAX.iter_days();
        assert_eq!(it.next(), None);
        assert_eq!(it.next(), None); // fused: stays None
    }

    #[test]
    fn days_iterator_yields_the_day_before_max_but_not_max_itself() {
        let start = NaiveDate::MAX.pred_opt().unwrap();
        let mut it = start.iter_days();
        assert_eq!(it.next(), Some(start));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn days_iterator_size_hint_reflects_remaining_days() {
        let start = NaiveDate::MAX.pred_opt().unwrap();
        let it = start.iter_days();
        assert_eq!(it.size_hint(), (1, Some(1)));
    }

    #[test]
    fn weeks_iterator_advances_seven_days_at_a_time() {
        let start = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let weeks: Vec<_> = start.iter_weeks().take(2).collect();
        assert_eq!(weeks, vec![start, NaiveDate::from_ymd_opt(2023, 6, 22).unwrap()]);
    }

    #[test]
    fn weeks_iterator_next_back_goes_backward_by_a_week() {
        let start = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let mut it = start.iter_weeks();
        assert_eq!(it.next_back(), Some(start));
        assert_eq!(it.next_back(), Some(NaiveDate::from_ymd_opt(2023, 6, 8).unwrap()));
    }
}
