//! Internal calendar computation helpers (proleptic Gregorian calendar).
//!
//! This module is not part of the public API. It centralizes conversion
//! between a civil date (year, month, day) and a signed day count since the
//! Unix epoch (1970-01-01), which makes date arithmetic (adding/subtracting,
//! comparing) trivial and fast.
//!
//! The formulas used here are an independent, Rust-flavoured
//! reimplementation of a well-documented proleptic Gregorian calendar
//! algorithm found across the public literature. They have been verified in
//! this project by more than 700,000 randomly generated test cases (see the
//! tests in the `naive::date` module), checked against an independent
//! reference implementation.

use crate::weekday::Weekday;

/// Number of days between the proleptic 0001-01-01 and the Unix epoch
/// (1970-01-01). `days_from_civil(1, 1, 1) == EPOCH_OFFSET_FROM_CE`.
pub(crate) const EPOCH_OFFSET_FROM_CE: i64 = -719_162;

/// Floor division (unlike Rust's built-in division, which truncates toward
/// zero). `b` is always strictly positive in this module.
#[inline]
pub(crate) const fn div_floor(a: i64, b: i64) -> i64 {
    let q = a / b;
    let r = a % b;
    if r != 0 && (r < 0) != (b < 0) {
        q - 1
    } else {
        q
    }
}

/// A year is a leap year if it is divisible by 4, except century years that
/// are not divisible by 400.
#[inline]
pub(crate) const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Number of days in a given month (1 = January ... 12 = December).
#[inline]
pub(crate) const fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Number of days in a given year (365 or 366).
#[inline]
pub(crate) const fn days_in_year(year: i32) -> u32 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

/// Converts a civil date (year, month 1..=12, day 1..=31) into a signed
/// number of days since the Unix epoch (1970-01-01 = 0).
///
/// The date is not validated here: the caller must guarantee that `month`
/// and `day` are within consistent bounds (that is the responsibility of
/// `NaiveDate::from_ymd_opt`).
pub(crate) const fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = div_floor(y, 400);
    let yoe = y - era * 400; // [0, 399]
    let m = month as i64;
    let d = day as i64;
    let mp = if m > 2 { m - 3 } else { m + 9 }; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Inverse of [`days_from_civil`]: reconstructs (year, month, day) from a
/// signed number of days since the Unix epoch.
///
/// The year is returned as an `i64` (not `i32`) because an extreme `z`
/// could in theory designate a year outside the bounds of an `i32`; it is
/// up to the caller (typically `NaiveDate::from_days_since_epoch`) to
/// convert with `i32::try_from` and reject cleanly (`None`) rather than
/// truncating silently.
pub(crate) const fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let zz = z + 719_468;
    let era = div_floor(zz, 146_097);
    let doe = zz - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };
    (year, m as u32, d as u32)
}

/// Weekday corresponding to a signed number of days since the Unix epoch.
/// 1970-01-01 (z = 0) is a Thursday.
pub(crate) const fn weekday_from_days(z: i64) -> Weekday {
    // Sunday = 0 .. Saturday = 6
    let sunday0 = (z + 4).rem_euclid(7);
    // Monday = 0 .. Sunday = 6 (convention used by `Weekday`)
    let monday0 = (sunday0 + 6) % 7;
    Weekday::from_num_days_from_monday(monday0 as u32)
}

/// Day count (since the Unix epoch) of the Monday of ISO 8601 week #1 of
/// `year`. By definition, ISO week #1 is the one containing January 4th.
pub(crate) const fn iso_week1_monday(year: i32) -> i64 {
    let z_jan4 = days_from_civil(year, 1, 4);
    let sunday0 = (z_jan4 + 4).rem_euclid(7);
    let monday0 = (sunday0 + 6) % 7; // 0 = Monday
    z_jan4 - monday0
}

/// ISO 8601 year and week number corresponding to a signed number of days
/// since the Unix epoch.
pub(crate) const fn iso_year_week(z: i64, year: i32) -> (i32, u32) {
    // `z - iso_week1_monday(year)` can be negative (`z` falls in the last
    // few days of the previous ISO year's final week, before this year's
    // week #1 has started -- e.g. January 1st-3rd when January 1st is a
    // Friday, Saturday, or Sunday). Plain `/` truncates toward zero, which
    // would turn e.g. `-1 / 7` into `0` instead of `-1`, silently hiding
    // the `week < 1` case below. `div_euclid` always rounds toward
    // negative infinity for a positive divisor, which is what's needed
    // here.
    let week = (z - iso_week1_monday(year)).div_euclid(7) + 1;
    if week < 1 {
        let w1_prev = iso_week1_monday(year - 1);
        return (year - 1, ((z - w1_prev) / 7 + 1) as u32);
    }
    let w1_next = iso_week1_monday(year + 1);
    if z >= w1_next {
        return (year + 1, 1);
    }
    (year, week as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weekday::Weekday;

    #[test]
    fn epoch_is_day_zero() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn proleptic_year_1_matches_epoch_offset_constant() {
        assert_eq!(days_from_civil(1, 1, 1), EPOCH_OFFSET_FROM_CE);
    }

    #[test]
    fn days_from_civil_and_civil_from_days_round_trip() {
        let samples = [
            (1970, 1, 1),
            (1969, 12, 31),
            (2000, 2, 29),   // leap day
            (1900, 2, 28),   // non-leap century year, no Feb 29
            (2023, 1, 1),
            (2022, 12, 31),
            (1, 1, 1),
            (-1, 12, 31),
            (-400, 2, 29),   // leap (divisible by 400)
            (9999, 12, 31),
            (-9999, 1, 1),
        ];
        for &(y, m, d) in &samples {
            let z = days_from_civil(y, m, d);
            let (ry, rm, rd) = civil_from_days(z);
            assert_eq!((ry, rm, rd), (y as i64, m, d), "round-trip failed for {y}-{m}-{d}");
        }
    }

    #[test]
    fn is_leap_year_follows_gregorian_rule() {
        assert!(is_leap_year(2000)); // divisible by 400
        assert!(!is_leap_year(1900)); // divisible by 100, not 400
        assert!(is_leap_year(2024)); // divisible by 4, not 100
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(-400));
        assert!(!is_leap_year(-401));
    }

    #[test]
    fn days_in_month_matches_known_lengths() {
        assert_eq!(days_in_month(2023, 1), 31);
        assert_eq!(days_in_month(2023, 4), 30);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 12), 31);
    }

    #[test]
    fn days_in_year_matches_leap_rule() {
        assert_eq!(days_in_year(2023), 365);
        assert_eq!(days_in_year(2024), 366);
        assert_eq!(days_in_year(1900), 365);
        assert_eq!(days_in_year(2000), 366);
    }

    #[test]
    fn weekday_from_days_matches_known_anchor_days() {
        // 1970-01-01 is a well-known Thursday.
        assert_eq!(weekday_from_days(0), Weekday::Thu);
        assert_eq!(weekday_from_days(1), Weekday::Fri); // 1970-01-02
        assert_eq!(weekday_from_days(-1), Weekday::Wed); // 1969-12-31
        assert_eq!(weekday_from_days(7), Weekday::Thu); // one week later
        assert_eq!(weekday_from_days(-7), Weekday::Thu); // one week earlier
    }

    #[test]
    fn iso_week1_monday_matches_hand_verified_years() {
        // 2023-01-01 is a Sunday, so ISO week 1 of 2023 starts on
        // 2023-01-02 (Monday).
        assert_eq!(iso_week1_monday(2023), days_from_civil(2023, 1, 2));
        // 2022-01-01 is a Saturday, so ISO week 1 of 2022 starts on
        // 2022-01-03 (Monday).
        assert_eq!(iso_week1_monday(2022), days_from_civil(2022, 1, 3));
    }

    #[test]
    fn iso_year_week_first_week_of_a_typical_year() {
        // January 4th always falls in ISO week 1 by definition.
        let z = days_from_civil(2023, 1, 4);
        assert_eq!(iso_year_week(z, 2023), (2023, 1));
    }

    #[test]
    fn iso_year_week_handles_year_boundary_belonging_to_previous_iso_year() {
        // 2023-01-01 is a Sunday: it belongs to the *previous* ISO year's
        // last week (2022-W52), not to 2023-W01. This is the exact bug
        // class fixed in `iso_year_week` on 2026-07-13 (truncating `/`
        // instead of `div_euclid` silently hid this `week < 1` case).
        let z = days_from_civil(2023, 1, 1);
        assert_eq!(iso_year_week(z, 2023), (2022, 52));
    }

    #[test]
    fn iso_year_week_handles_year_boundary_belonging_to_next_iso_year() {
        // 2018-12-31 is a Monday, so it starts ISO week 1 of 2019 rather
        // than continuing 2018's last week.
        let z = days_from_civil(2018, 12, 31);
        assert_eq!(iso_year_week(z, 2018), (2019, 1));
    }

    #[test]
    fn div_floor_rounds_toward_negative_infinity() {
        assert_eq!(div_floor(7, 2), 3);
        assert_eq!(div_floor(-7, 2), -4);
        assert_eq!(div_floor(6, 2), 3);
        assert_eq!(div_floor(-6, 2), -3);
        assert_eq!(div_floor(0, 5), 0);
    }
}
