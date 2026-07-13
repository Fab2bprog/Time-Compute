//! Japanese era (nengo) support.
//!
//! # `time_compute` extension -- not part of chrono
//! This entire module has **no chrono equivalent** -- chrono has no
//! notion of Japanese eras at all. Added at Fabrice's request, alongside
//! the Hebrew and Hijri calendar support in `naive/date.rs`.
//!
//! Only the five **modern** eras are covered, i.e. the "one reign, one
//! era name" system adopted in 1868 (Meiji, Taisho, Showa, Heisei,
//! Reiwa). Pre-Meiji era names (of which there are over 200, some
//! lasting only months) are out of scope.

use crate::naive::NaiveDate;

/// A modern Japanese era ("nengo", 元号), one per imperial reign since the
/// "one reign, one era name" system was formalized in 1868 (law-codified
/// in 1979).
///
/// # `time_compute` extension -- not part of chrono
/// No chrono equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JapaneseEra {
    /// Meiji (明治), 25 January 1868 -- 29 July 1912.
    Meiji,
    /// Taisho (大正), 30 July 1912 -- 24 December 1926.
    Taisho,
    /// Showa (昭和), 25 December 1926 -- 7 January 1989.
    Showa,
    /// Heisei (平成), 8 January 1989 -- 30 April 2019.
    Heisei,
    /// Reiwa (令和), 1 May 2019 -- present (open-ended).
    Reiwa,
}

impl JapaneseEra {
    /// The first day of this era (proleptic Gregorian calendar), the day
    /// the corresponding emperor's reign began.
    ///
    /// These five dates are the ones commonly used in software (e.g.
    /// ICU/Unicode CLDR) for Gregorian conversion. Note the historical
    /// nuance around Meiji, which predates Japan's 1873 adoption of the
    /// Gregorian calendar.
    #[must_use]
    pub const fn start_date(&self) -> NaiveDate {
        let opt = match self {
            JapaneseEra::Meiji => NaiveDate::from_ymd_opt(1868, 1, 25),
            JapaneseEra::Taisho => NaiveDate::from_ymd_opt(1912, 7, 30),
            JapaneseEra::Showa => NaiveDate::from_ymd_opt(1926, 12, 25),
            JapaneseEra::Heisei => NaiveDate::from_ymd_opt(1989, 1, 8),
            JapaneseEra::Reiwa => NaiveDate::from_ymd_opt(2019, 5, 1),
        };
        match opt {
            Some(date) => date,
            // Unreachable: all five dates above are valid calendar dates.
            None => NaiveDate::MIN,
        }
    }

    /// The first day of the *next* era, i.e. the exclusive end of this
    /// one -- or `None` if this is the current, open-ended era (Reiwa
    /// today; will need updating if/when a new era begins).
    #[must_use]
    pub const fn end_date_exclusive(&self) -> Option<NaiveDate> {
        match self {
            JapaneseEra::Meiji => Some(JapaneseEra::Taisho.start_date()),
            JapaneseEra::Taisho => Some(JapaneseEra::Showa.start_date()),
            JapaneseEra::Showa => Some(JapaneseEra::Heisei.start_date()),
            JapaneseEra::Heisei => Some(JapaneseEra::Reiwa.start_date()),
            JapaneseEra::Reiwa => None,
        }
    }

    /// Romanized name of the era ("Meiji", "Taisho", "Showa", "Heisei",
    /// "Reiwa").
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            JapaneseEra::Meiji => "Meiji",
            JapaneseEra::Taisho => "Taisho",
            JapaneseEra::Showa => "Showa",
            JapaneseEra::Heisei => "Heisei",
            JapaneseEra::Reiwa => "Reiwa",
        }
    }
}

impl std::fmt::Display for JapaneseEra {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
