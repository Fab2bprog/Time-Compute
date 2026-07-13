//! Locale data used by localized formatting/parsing (`unstable-locales`
//! feature only).
//!
//! When the feature is disabled, this module still exists but only ever
//! exposes the English/POSIX defaults: everything routes through the same
//! lookup functions either way, so the rest of the formatting engine does
//! not need two separate code paths.

#[cfg(feature = "unstable-locales")]
mod enabled {
    use pure_rust_locales::{locale_match, Locale};

    pub(crate) const fn default_locale() -> Locale {
        Locale::POSIX
    }

    pub(crate) const fn short_months(locale: Locale) -> &'static [&'static str] {
        locale_match!(locale => LC_TIME::ABMON)
    }

    pub(crate) const fn long_months(locale: Locale) -> &'static [&'static str] {
        locale_match!(locale => LC_TIME::MON)
    }

    pub(crate) const fn short_weekdays(locale: Locale) -> &'static [&'static str] {
        locale_match!(locale => LC_TIME::ABDAY)
    }

    pub(crate) const fn long_weekdays(locale: Locale) -> &'static [&'static str] {
        locale_match!(locale => LC_TIME::DAY)
    }

    pub(crate) const fn am_pm(locale: Locale) -> &'static [&'static str] {
        locale_match!(locale => LC_TIME::AM_PM)
    }

    pub(crate) const fn decimal_point(locale: Locale) -> &'static str {
        locale_match!(locale => LC_NUMERIC::DECIMAL_POINT)
    }

    pub(crate) const fn d_fmt(locale: Locale) -> &'static str {
        locale_match!(locale => LC_TIME::D_FMT)
    }

    pub(crate) const fn d_t_fmt(locale: Locale) -> &'static str {
        locale_match!(locale => LC_TIME::D_T_FMT)
    }

    pub(crate) const fn t_fmt(locale: Locale) -> &'static str {
        locale_match!(locale => LC_TIME::T_FMT)
    }

    pub(crate) const fn t_fmt_ampm(locale: Locale) -> &'static str {
        locale_match!(locale => LC_TIME::T_FMT_AMPM)
    }
}

#[cfg(feature = "unstable-locales")]
pub(crate) use enabled::*;
/// Re-exported at the crate root as `time_compute::Locale` (only when the
/// `unstable-locales` feature is enabled).
#[cfg(feature = "unstable-locales")]
pub use pure_rust_locales::Locale;

/// Stand-in used when the `unstable-locales` feature is disabled: a
/// zero-sized type, since there is only ever one (English/POSIX) locale
/// available in that configuration.
#[cfg(not(feature = "unstable-locales"))]
mod disabled {
    #[derive(Copy, Clone, Debug)]
    pub(crate) struct Locale;

    pub(crate) const fn default_locale() -> Locale {
        Locale
    }

    pub(crate) const fn short_months(_locale: Locale) -> &'static [&'static str] {
        &["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
    }

    pub(crate) const fn long_months(_locale: Locale) -> &'static [&'static str] {
        &[
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ]
    }

    // Indexed by `Weekday::num_days_from_sunday()` (Sunday = 0), matching
    // the POSIX/glibc convention used by `pure-rust-locales`.
    pub(crate) const fn short_weekdays(_locale: Locale) -> &'static [&'static str] {
        &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
    }

    pub(crate) const fn long_weekdays(_locale: Locale) -> &'static [&'static str] {
        &["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"]
    }

    pub(crate) const fn am_pm(_locale: Locale) -> &'static [&'static str] {
        &["AM", "PM"]
    }

    pub(crate) const fn decimal_point(_locale: Locale) -> &'static str {
        "."
    }
}

#[cfg(not(feature = "unstable-locales"))]
pub(crate) use disabled::*;

#[cfg(test)]
mod tests {
    use super::*;

    // Only the functions common to both the `enabled` (`unstable-locales`)
    // and `disabled` variants are exercised here, so these tests hold
    // regardless of which one is compiled in. `default_locale()` always
    // resolves to the English/POSIX defaults in either configuration.

    #[test]
    fn default_locale_short_and_long_months_are_english() {
        let locale = default_locale();
        assert_eq!(short_months(locale).len(), 12);
        assert_eq!(short_months(locale)[0], "Jan");
        assert_eq!(short_months(locale)[11], "Dec");
        assert_eq!(long_months(locale).len(), 12);
        assert_eq!(long_months(locale)[0], "January");
        assert_eq!(long_months(locale)[11], "December");
    }

    #[test]
    fn default_locale_weekdays_are_indexed_from_sunday() {
        // Matches the POSIX/glibc convention: index 0 is Sunday, not Monday.
        let locale = default_locale();
        assert_eq!(short_weekdays(locale).len(), 7);
        assert_eq!(short_weekdays(locale)[0], "Sun");
        assert_eq!(short_weekdays(locale)[6], "Sat");
        assert_eq!(long_weekdays(locale)[0], "Sunday");
        assert_eq!(long_weekdays(locale)[6], "Saturday");
    }

    #[test]
    fn default_locale_am_pm_and_decimal_point() {
        let locale = default_locale();
        assert_eq!(am_pm(locale)[0], "AM");
        assert_eq!(am_pm(locale)[1], "PM");
        assert_eq!(decimal_point(locale), ".");
    }
}
