//! Matariki (Māori New Year) public holiday dates (private implementation).
//!
//! # `time_compute` extension -- not part of chrono
//! Backs [`crate::NaiveDate::matariki`] -- see that method's rustdoc (in
//! `naive/date.rs`) for the public API and sourcing notes. This module
//! holds only the private lookup table.
//!
//! Unlike every other calendar/festival function in this crate, Matariki
//! is **not** computed from a formula (astronomical or arithmetic): the
//! date is fixed each year by New Zealand's Matariki Advisory Committee
//! and legislated as a public holiday. The committee's own criterion
//! ("the Friday closest to the Tangaroa lunar phase in the lunar month
//! of Pipiri, during which the Matariki star cluster/Pleiades makes its
//! pre-dawn midwinter rising") is a description of *how the committee
//! chooses*, not a closed-form rule a program can reliably reproduce --
//! Tangaroa is a several-day phase window, not a single instant, so
//! "closest Friday" can genuinely go either way at the edges (and, per
//! the committee's own published table, occasionally does not land on
//! the single naively-nearest Friday at all). Reverse-engineering an
//! approximate formula would risk silently disagreeing with the actual
//! legal public holiday date, which defeats the point.
//!
//! The dates below are transcribed from the Museum of New Zealand Te
//! Papa Tongarewa's official published table (2022-2052, "agreed upon
//! by the Matariki Advisory Committee"), cross-checked against RNZ's
//! 2022 report that the government announced all 30 years' worth of
//! dates at once. All 31 dates were verified to fall on a Friday (as
//! the committee's own rule requires) before being transcribed here.
//!
//! # `time_compute` extension -- not part of chrono
//! No chrono equivalent (a 2022-era New Zealand public holiday). Added
//! after Fabrice asked about Pacific/Oceania festivals following the
//! Chinese and Thai/Buddhist calendar work above.

use crate::naive::NaiveDate;

/// `(year, month, day)` for every officially published Matariki public
/// holiday, 2022-2052 inclusive. See the module doc comment for why
/// this is a fixed table rather than a computed rule.
const MATARIKI_DATES: [(i32, u32, u32); 31] = [
    (2022, 6, 24),
    (2023, 7, 14),
    (2024, 6, 28),
    (2025, 6, 20),
    (2026, 7, 10),
    (2027, 6, 25),
    (2028, 7, 14),
    (2029, 7, 6),
    (2030, 6, 21),
    (2031, 7, 11),
    (2032, 7, 2),
    (2033, 6, 24),
    (2034, 7, 7),
    (2035, 6, 29),
    (2036, 7, 18),
    (2037, 7, 10),
    (2038, 6, 25),
    (2039, 7, 15),
    (2040, 7, 6),
    (2041, 7, 19),
    (2042, 7, 11),
    (2043, 7, 3),
    (2044, 6, 24),
    (2045, 7, 7),
    (2046, 6, 29),
    (2047, 7, 19),
    (2048, 7, 3),
    (2049, 6, 25),
    (2050, 7, 15),
    (2051, 6, 30),
    (2052, 6, 21),
];

/// The officially published Matariki public holiday date for `year`, or
/// `None` if outside the 2022-2052 range this table covers (the
/// committee publishes these in batches, and has not published dates
/// beyond 2052 as of this writing).
pub(crate) fn matariki(year: i32) -> Option<NaiveDate> {
    let (_, month, day) = MATARIKI_DATES.iter().copied().find(|&(y, _, _)| y == year)?;
    NaiveDate::from_ymd_opt(year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Datelike;

    #[test]
    fn matariki_matches_te_papa_published_dates() {
        for &(y, m, d) in &MATARIKI_DATES {
            assert_eq!(matariki(y), NaiveDate::from_ymd_opt(y, m, d), "matariki({y})");
        }
    }

    #[test]
    fn matariki_is_always_a_friday() {
        // The committee's own rule: always the Friday closest to the
        // relevant lunar phase. A cheap, independent sanity check on
        // the transcribed table, using `Datelike::weekday` rather than
        // re-deriving the dates themselves.
        for &(y, _, _) in &MATARIKI_DATES {
            let d = matariki(y).unwrap_or_else(|| panic!("matariki({y}) returned None"));
            assert_eq!(d.weekday(), crate::Weekday::Fri, "matariki({y}) = {d:?} is not a Friday");
        }
    }

    #[test]
    fn matariki_is_none_outside_the_published_range() {
        assert_eq!(matariki(2021), None);
        assert_eq!(matariki(2053), None);
    }
}
