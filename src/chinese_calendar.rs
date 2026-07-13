//! Chinese lunisolar calendar engine (private implementation).
//!
//! # `time_compute` extension -- not part of chrono
//! Backs [`crate::NaiveDate::to_chinese_ymd`], [`crate::NaiveDate::from_chinese_ymd`],
//! [`crate::NaiveDate::chinese_new_year`], [`crate::NaiveDate::duanwu`],
//! [`crate::NaiveDate::zhongqiu`], and [`crate::NaiveDate::qingming`] --
//! see those methods' rustdoc (in `naive/date.rs`) for the public API.
//! This module holds only the private machinery; the algorithm and
//! verification notes below are this module's own.
//!
//! Implements the standard modern Chinese calendar algorithm (in use,
//! with minor refinements, since the calendar reform of 1645; formalized
//! in China's national standard GB/T 33661-2017):
//!
//! - The first day of a lunar month is the (China Standard Time, UTC+8)
//!   calendar day on which a new moon (lunar conjunction) falls.
//! - The month containing the December solstice is always month 11.
//! - The period from one "month 11" to the next ("suì", 歲) has either 12
//!   or 13 complete lunar months. If 13, exactly one is a leap
//!   ("intercalary") month: the first one (after month 11) that contains
//!   no "zhongqi" (major solar term, i.e. a moment the Sun's ecliptic
//!   longitude is an exact multiple of 30 degrees). The leap month takes
//!   the same number as the regular month immediately before it.
//!
//! This is the crate's second use of the `astro` dependency (after the
//! Japanese solar-term functions -- see `Cargo.toml` for why that
//! exception exists) and, like those functions, is
//! floating-point and not `const fn`. The low-level solar-longitude
//! root-finder below is a separate copy of the one in `naive/date.rs`
//! (`NaiveDate::bisect_solar_longitude`) rather than a shared/refactored
//! helper: this avoided touching already-verified working code while
//! adding a second, independent, much more involved consumer.
//!
//! Verified in a from-scratch Python prototype (`pymeeus`, an independent
//! pure-Python implementation of the same Meeus algorithms) before this
//! Rust translation: all 8 Chinese New Year dates 2020-2027, all 7 leap
//! month positions in a curated list spanning 2020-2036 (including the
//! notoriously tricky "exceptional year" 2033, leap month 11), Dragon
//! Boat and Mid-Autumn festival dates for 2023-2025, and an exhaustive
//! invariant check (exactly one regular
//! occurrence of each month 1-12, at most one leap month, every month 29
//! or 30 days) across all 201 years from 1900 to 2100 -- zero mismatches.

use crate::duration::Duration;
use crate::naive::NaiveDate;
use crate::traits::Datelike;
use std::cell::RefCell;
use std::collections::HashMap;

const TROPICAL_YEAR_DAYS: f64 = 365.2422;
const SYNODIC_MONTH_DAYS: f64 = 29.530588861;

/// Same Gregorian leap-year rule `astro::time::decimal_year` uses
/// internally (year divisible by 4, except centuries not divisible by
/// 400) -- needed by [`nearest_new_moon_jde`] to reconstruct that
/// function's own arithmetic exactly. See that function's doc comment.
fn is_gregorian_leap_year(year: i16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// `(month number 1..=12, is_leap, first day of the month in CST)`.
type MonthEntry = (u32, bool, NaiveDate);

// ---- Low-level astronomy helpers (see the module-level doc comment for
// why this duplicates, rather than reuses, `NaiveDate`'s private solar
// longitude helpers) ----

fn solar_longitude_deg(jde: f64) -> f64 {
    let (ecl_point, _sun_earth_dist_au) = astro::sun::geocent_ecl_pos(jde);
    let deg = ecl_point.long.to_degrees() % 360.0;
    if deg < 0.0 {
        deg + 360.0
    } else {
        deg
    }
}

fn signed_longitude_distance_deg(a: f64, target: f64) -> f64 {
    let d = (a - target) % 360.0;
    if d <= -180.0 {
        d + 360.0
    } else if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

/// Bisects for the Julian Ephemeris Day, within `+/- window_days` of
/// `seed_jde`, at which the Sun's apparent geocentric ecliptic longitude
/// crosses `target_deg` degrees.
fn bisect_solar_longitude(seed_jde: f64, target_deg: f64, window_days: f64) -> f64 {
    let f = |jde: f64| signed_longitude_distance_deg(solar_longitude_deg(jde), target_deg);
    let step = window_days / 16.0;
    let mut lo = seed_jde - window_days;
    let mut f_lo = f(lo);
    let mut hi = seed_jde + window_days;
    let mut t = lo;
    while t <= seed_jde + window_days {
        let f_t = f(t);
        if f_lo * f_t < 0.0 {
            hi = t;
            break;
        }
        lo = t;
        f_lo = f_t;
        t += step;
    }
    let mut a = lo;
    let mut b = hi;
    let mut f_a = f(a);
    for _ in 0..60 {
        let mid = (a + b) / 2.0;
        let f_mid = f(mid);
        if f_a * f_mid <= 0.0 {
            b = mid;
        } else {
            a = mid;
            f_a = f_mid;
        }
    }
    (a + b) / 2.0
}

/// The Julian Ephemeris Day of the solar term at `target_deg` degrees,
/// searching near `year`-`seed_month`-`seed_day` (a plain calendar date a
/// few days from the true crossing is enough of a seed).
fn solar_term_jde(year: i32, seed_month: u32, seed_day: f64, target_deg: f64) -> Option<f64> {
    let year_i16 = i16::try_from(year).ok()?;
    let month_u8 = u8::try_from(seed_month).ok()?;
    let seed_date = astro::time::Date {
        year: year_i16,
        month: month_u8,
        decimal_day: seed_day,
        cal_type: astro::time::CalType::Gregorian,
    };
    let seed_jd = astro::time::julian_day(&seed_date);
    let delta_t = astro::time::delta_t(year, month_u8);
    let seed_jde = astro::time::julian_ephemeris_day(seed_jd, delta_t);
    Some(bisect_solar_longitude(seed_jde, target_deg, 8.0))
}

fn winter_solstice_jde(year: i32) -> Option<f64> {
    solar_term_jde(year, 12, 21.0, 270.0)
}

/// Converts a Julian (Ephemeris) Day to an `astro::time::Date`. Works
/// regardless of whether `jde` represents Terrestrial or Universal Time
/// -- this is a pure calendrical (Julian Day -> calendar date)
/// conversion, the TT/UT distinction only matters when relating the
/// result back to a real-world clock reading, which callers handle
/// separately (see [`jde_to_cst_date`]).
fn astro_date_from_jde(jde: f64) -> Option<astro::time::Date> {
    let (year, month, decimal_day) = astro::time::date_frm_julian_day(jde).ok()?;
    Some(astro::time::Date { year, month, decimal_day, cal_type: astro::time::CalType::Gregorian })
}

/// The Julian Ephemeris Day of the new moon (lunar conjunction) closest
/// to `seed_jde`.
///
/// # A genuine `astro`-crate bug, worked around here
///
/// Per Meeus (*Astronomical Algorithms*, ch. 49), the lunation index `K`
/// used to locate "the phase closest to a date" must be *rounded* to the
/// nearest integer. `astro::lunar::time_of_phase`'s actual source (read
/// directly from `lunar.rs`) instead does:
/// ```text
/// let mut K = 12.3685 * (time::decimal_year(&date) - 2000.0);
/// K = (K as i64) as f64;
/// ```
/// `as i64` truncates towards zero, which is *not* the same as rounding
/// to nearest. For seed dates after 2000 (`K >= 0`) this happens to
/// coincide with `K.floor()`; for seed dates before 2000 (`K < 0`, e.g.
/// all of 1948) it instead coincides with `K.ceil()` -- the opposite
/// bias. The practical effect: for pre-2000 dates, seeding with a date
/// that's *before* a target new moon can still resolve to the *next*
/// one instead, once the seed's raw `K` crosses an integer boundary.
/// Confirmed both by reading `astro`'s source and empirically, via a
/// `--nocapture` scan of seeds through November-December 1948: results
/// jumped from the correct 1948-12-01 new moon to the wrong 1948-12-30
/// one between seeds seven days apart, tracking exactly where the
/// *truncated* (not rounded) `K` crossed an integer.
///
/// The fix: replicate `astro`'s own `K` formula here, round it
/// *correctly* (`K.round()`), then hand `time_of_phase` a synthetic seed
/// date engineered so that when it re-derives `K` from that seed and
/// truncates, it lands back on our correctly-rounded value. The
/// synthetic seed need not represent any real calendar date --
/// `decimal_year` only depends on `year + (day-of-year fraction)`, so
/// any `(year, month = 1, decimal_day)` combination that reproduces the
/// intended fractional year works, and using month 1 sidesteps that
/// formula's leap-day adjustment (which only applies for month > 2).
fn nearest_new_moon_jde(seed_jde: f64) -> Option<f64> {
    let seed_date = astro_date_from_jde(seed_jde)?;
    let decimal_year = astro::time::decimal_year(&seed_date);
    let k_target = (12.3685 * (decimal_year - 2000.0)).round();

    // Bias by 0.1 (out of a full unit spacing of 1.0 between adjacent
    // lunation indices) towards the side that guarantees `astro`'s
    // truncate-towards-zero reproduces `k_target` exactly, regardless of
    // floating-point jitter in the round trip through `decimal_year`.
    let k_biased = if k_target >= 0.0 { k_target + 0.1 } else { k_target - 0.1 };
    let synth_decimal_year = 2000.0 + k_biased / 12.3685;

    let year = synth_decimal_year.floor();
    let frac = synth_decimal_year - year;
    let year_i16 = i16::try_from(year as i64).ok()?;
    let days_in_year = if is_gregorian_leap_year(year_i16) { 366.0 } else { 365.0 };
    let synth_date = astro::time::Date {
        year: year_i16,
        month: 1,
        decimal_day: frac * days_in_year,
        cal_type: astro::time::CalType::Gregorian,
    };
    Some(astro::lunar::time_of_phase(&synth_date, &astro::lunar::Phase::New))
}

/// The Julian Ephemeris Day of the latest new moon whose *instant* is at
/// or before `target_jde`.
fn new_moon_on_or_before(target_jde: f64) -> Option<f64> {
    let mut guess = target_jde;
    let mut nm_jde = nearest_new_moon_jde(guess)?;
    let mut guard = 0;
    while nm_jde > target_jde {
        guess = nm_jde - SYNODIC_MONTH_DAYS;
        nm_jde = nearest_new_moon_jde(guess)?;
        guard += 1;
        if guard > 30 {
            return None;
        }
    }
    // Catch-up loop: `nearest_new_moon_jde(target_jde)` can land one
    // synodic month early, so step forward while the *next* new moon is
    // still at or before `target_jde`. This should only ever need 0-2
    // iterations; a hard cap guards against looping forever if
    // `nearest_new_moon_jde` ever fails to make forward progress (e.g.
    // a still-unknown edge case in the K-rounding fix above).
    let mut catch_up_guard = 0;
    loop {
        let next_jde = nearest_new_moon_jde(nm_jde + SYNODIC_MONTH_DAYS)?;
        if next_jde <= target_jde {
            nm_jde = next_jde;
        } else {
            break;
        }
        catch_up_guard += 1;
        if catch_up_guard > 30 {
            return None;
        }
    }
    Some(nm_jde)
}

/// The Julian Ephemeris Day of the first new moon whose instant is
/// strictly after `jde`.
fn next_new_moon_after(jde: f64) -> Option<f64> {
    let mut guess = jde + SYNODIC_MONTH_DAYS;
    let mut nm_jde = nearest_new_moon_jde(guess)?;
    let mut guard = 0;
    while nm_jde <= jde {
        guess = nm_jde + SYNODIC_MONTH_DAYS;
        nm_jde = nearest_new_moon_jde(guess)?;
        guard += 1;
        if guard > 30 {
            return None;
        }
    }
    // Catch-up loop, mirroring `new_moon_on_or_before`'s: bounded so a
    // still-unknown edge case in `nearest_new_moon_jde` can't hang this
    // function instead of just returning `None`.
    let mut catch_up_guard = 0;
    loop {
        let prev_jde = nearest_new_moon_jde(nm_jde - SYNODIC_MONTH_DAYS)?;
        if prev_jde > jde {
            nm_jde = prev_jde;
        } else {
            break;
        }
        catch_up_guard += 1;
        if catch_up_guard > 30 {
            return None;
        }
    }
    Some(nm_jde)
}

/// Converts a Julian Ephemeris Day (Terrestrial Time) to the China
/// Standard Time (UTC+8) calendar date containing that instant.
fn jde_to_cst_date(jde: f64) -> Option<NaiveDate> {
    let (y, m, _) = astro::time::date_frm_julian_day(jde).ok()?;
    let delta_t = astro::time::delta_t(i32::from(y), m);
    let ut_jde = jde - delta_t / 86_400.0;
    let cst_jde = ut_jde + 8.0 / 24.0;
    let (y2, m2, decimal_day) = astro::time::date_frm_julian_day(cst_jde).ok()?;
    NaiveDate::from_ymd_opt(i32::from(y2), u32::from(m2), decimal_day.trunc() as u32)
}

/// Civil-date-aware version of [`new_moon_on_or_before`]: a new moon
/// whose precise *instant* is after `target_jde` can still be "on or
/// before" it in CST civil-date terms, if both land on the same calendar
/// day. This really happens -- e.g. 1984, where the December solstice
/// fell at 00:22 CST and that same day's new moon at 19:46 CST, both on
/// December 22. The Chinese calendar's rules (first day of a month = the
/// civil day of the new moon; winter solstice always in month 11) are
/// stated in terms of civil days, not raw instants, so this comparison
/// must be too -- using the instant-based version alone silently drops
/// the December-22-starting month from month 11 in years like 1984.
fn new_moon_on_or_before_civil(target_jde: f64) -> Option<f64> {
    let target_civil = jde_to_cst_date(target_jde)?;
    let mut m = new_moon_on_or_before(target_jde)?;
    // Bounded for the same reason as the catch-up loops in
    // `new_moon_on_or_before`/`next_new_moon_after`: this should only
    // ever need 0-1 iterations in practice.
    let mut guard = 0;
    loop {
        let nxt = next_new_moon_after(m)?;
        if jde_to_cst_date(nxt)? <= target_civil {
            m = nxt;
        } else {
            break;
        }
        guard += 1;
        if guard > 30 {
            return None;
        }
    }
    Some(m)
}

/// Civil (CST) dates of every major solar term ("zhongqi") from the known
/// zhongqi instant `ws0_jde` (a December solstice, 270 degrees) forward
/// through past `end_jde`, stepping ~30.44 days at a time with a wide
/// (+/-12 day) bisection window -- safely bracketing the true ~29.4-31.4
/// day real-world spacing between consecutive major terms.
fn zhongqi_civil_dates_from(ws0_jde: f64, end_jde: f64) -> Option<Vec<NaiveDate>> {
    let mut results = vec![jde_to_cst_date(ws0_jde)?];
    let mut jde = ws0_jde;
    let mut target = 270.0_f64;
    let mut guard = 0;
    while jde < end_jde + 3.0 {
        target = (target + 30.0) % 360.0;
        let seed = jde + TROPICAL_YEAR_DAYS / 12.0;
        jde = bisect_solar_longitude(seed, target, 12.0);
        results.push(jde_to_cst_date(jde)?);
        guard += 1;
        if guard > 20 {
            break;
        }
    }
    results.sort_unstable();
    results.dedup();
    Some(results)
}

thread_local! {
    // Memoizes `sui_months_uncached`, which is expensive (several
    // solar-longitude bisections, each evaluating a full VSOP87-style
    // solar position series dozens of times) and gets called on the
    // *same* `year_for_solstice` repeatedly in normal use:
    // `chinese_year_months(y)` needs `sui_months(y-1)` and
    // `sui_months(y)`, so `chinese_year_months(y)` and
    // `chinese_year_months(y+1)` both need `sui_months(y)`, and
    // `to_chinese_ymd`'s year-search loop can call `chinese_new_year`
    // (hence `chinese_year_months`, hence `sui_months`) for the same
    // year several times over. Without this cache, converting a wide
    // range of dates was measured at ~58s in `--release`; a thread-local
    // cache (safe here since `sui_months` is a pure function of its
    // argument, so no cross-thread synchronization is needed) removes
    // essentially all of that redundant work.
    static SUI_MONTHS_CACHE: RefCell<HashMap<i32, Option<Vec<MonthEntry>>>> = RefCell::new(HashMap::new());
}

/// All lunar months in the "suì" (歲) running from the December solstice
/// of `year_for_solstice` to the December solstice of
/// `year_for_solstice + 1`: month 11 and month 12 of Chinese Year
/// `year_for_solstice`, followed by months 1 through 10 of Chinese Year
/// `year_for_solstice + 1` (with a leap month inserted somewhere in this
/// sequence if the suì has 13 months).
///
/// Memoized (see [`SUI_MONTHS_CACHE`]) -- the actual computation is in
/// [`sui_months_uncached`].
fn sui_months(year_for_solstice: i32) -> Option<Vec<MonthEntry>> {
    if let Some(cached) = SUI_MONTHS_CACHE.with(|c| c.borrow().get(&year_for_solstice).cloned()) {
        return cached;
    }
    let result = sui_months_uncached(year_for_solstice);
    SUI_MONTHS_CACHE.with(|c| c.borrow_mut().insert(year_for_solstice, result.clone()));
    result
}

fn sui_months_uncached(year_for_solstice: i32) -> Option<Vec<MonthEntry>> {
    let ws0 = winter_solstice_jde(year_for_solstice)?;
    let ws1 = winter_solstice_jde(year_for_solstice + 1)?;
    let m_minus1 = new_moon_on_or_before_civil(ws0)?;
    let m11 = new_moon_on_or_before_civil(ws1)?;

    let mut moons = vec![m_minus1];
    let mut guard = 0;
    while *moons.last().unwrap() < m11 - 1.0 {
        let nxt = next_new_moon_after(*moons.last().unwrap())?;
        moons.push(nxt);
        guard += 1;
        // A suì has 12 or 13 months; 40 is a generous safety margin, not
        // a value we expect to ever actually reach in practice.
        if guard > 40 {
            return None;
        }
    }
    if (*moons.last().unwrap() - m11).abs() > 1.0 {
        let last = moons.len() - 1;
        moons[last] = m11;
    }
    let num_months = moons.len() - 1;
    if num_months != 12 && num_months != 13 {
        return None;
    }

    let civil_dates: Option<Vec<NaiveDate>> = moons.iter().map(|&m| jde_to_cst_date(m)).collect();
    let civil_dates = civil_dates?;

    let mut leap_index: Option<usize> = None;
    if num_months == 13 {
        let zq_dates = zhongqi_civil_dates_from(ws0, m11)?;
        for (i, window) in civil_dates.windows(2).enumerate() {
            let contains = zq_dates.iter().any(|&z| window[0] <= z && z < window[1]);
            if !contains {
                leap_index = Some(i);
                break;
            }
        }
    }

    let mut result = Vec::with_capacity(num_months);
    let mut current: u32 = 11;
    let mut prev_number: Option<u32> = None;
    for i in 0..num_months {
        if leap_index == Some(i) {
            result.push((prev_number?, true, civil_dates[i]));
        } else {
            result.push((current, false, civil_dates[i]));
            prev_number = Some(current);
            current = if current < 12 { current + 1 } else { 1 };
        }
    }
    Some(result)
}

/// All 12 (or 13, in a leap year) months of Chinese Year `year`, sorted
/// chronologically: months 1-10 come from the suì ending at the December
/// solstice of `year` itself, months 11-12 (which occur near the *end*
/// of Chinese Year `year`, in Gregorian December of `year`/January of
/// `year + 1`) come from the following suì.
fn chinese_year_months(year: i32) -> Option<Vec<MonthEntry>> {
    let sui_a = sui_months(year - 1)?;
    let sui_b = sui_months(year)?;

    let mut months: Vec<MonthEntry> = sui_a.into_iter().filter(|m| (1..=10).contains(&m.0)).collect();
    for m in sui_b {
        if m.0 == 11 || m.0 == 12 {
            months.push(m);
        } else {
            break;
        }
    }
    months.sort_by_key(|m| m.2);
    Some(months)
}

// ---- Public-facing entry points (thin wrappers called from `naive/date.rs`) ----

pub(crate) fn chinese_new_year(year: i32) -> Option<NaiveDate> {
    let months = chinese_year_months(year)?;
    months.iter().find(|m| m.0 == 1 && !m.1).map(|m| m.2)
}

pub(crate) fn to_chinese_ymd(date: NaiveDate) -> Option<(i32, u32, bool, u32)> {
    let mut year = date.year();
    let mut guard = 0;
    loop {
        let ny = chinese_new_year(year)?;
        if date < ny {
            year -= 1;
        } else {
            let next_ny = chinese_new_year(year + 1)?;
            if date >= next_ny {
                year += 1;
            } else {
                break;
            }
        }
        guard += 1;
        if guard > 20 {
            return None;
        }
    }
    let months = chinese_year_months(year)?;
    let (month, is_leap, start) = months.iter().rev().find(|m| m.2 <= date).copied()?;
    let day = date.num_days_from_ce() - start.num_days_from_ce() + 1;
    Some((year, month, is_leap, u32::try_from(day).ok()?))
}

pub(crate) fn from_chinese_ymd(year: i32, month: u32, is_leap: bool, day: u32) -> Option<NaiveDate> {
    if day < 1 || day > 30 {
        return None;
    }
    let months = chinese_year_months(year)?;
    let idx = months.iter().position(|m| m.0 == month && m.1 == is_leap)?;
    let start = months[idx].2;
    let month_length: i32 = if idx + 1 < months.len() {
        months[idx + 1].2.num_days_from_ce() - start.num_days_from_ce()
    } else {
        let next_ny = chinese_new_year(year + 1)?;
        next_ny.num_days_from_ce() - start.num_days_from_ce()
    };
    if i32::try_from(day).ok()? > month_length {
        return None;
    }
    start.checked_add_signed(Duration::days(i64::from(day) - 1))
}

pub(crate) fn qingming(year: i32) -> Option<NaiveDate> {
    let jde = solar_term_jde(year, 4, 5.0, 15.0)?;
    jde_to_cst_date(jde)
}

// ---- Diagnostic regression tests -----------------------------------
//
// Added after `to_chinese_ymd_and_from_chinese_ymd_round_trip_a_wide_range_of_dates`
// (in `naive/date.rs`) failed on its very first date, 1950-01-01, in
// Fabrice's `cargo test` run. Tracing it down (via successive rounds of
// `cargo test <name> -- --nocapture`, since diagnosing this needed a
// real Rust compiler, which this assistant doesn't have) led all the way
// to a genuine bug in the `astro` crate's own `lunar::time_of_phase`
// (truncating a lunation index instead of rounding it -- see
// [`nearest_new_moon_jde`]'s doc comment for the full story), now fixed
// in `nearest_new_moon_jde`. Kept as permanent regression tests: 1948 is
// a 13-month leap suì (leap month 7), the exact kind of case where the
// bug bit.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_moon_on_or_before_ws0_1948_regression() {
        let ws0 = winter_solstice_jde(1948).expect("winter_solstice_jde(1948) returned None");
        assert_eq!(jde_to_cst_date(ws0), NaiveDate::from_ymd_opt(1948, 12, 22));

        let nm_jde = new_moon_on_or_before(ws0).expect("new_moon_on_or_before(ws0) returned None");
        assert_eq!(jde_to_cst_date(nm_jde), NaiveDate::from_ymd_opt(1948, 12, 1));
    }

    #[test]
    fn nearest_new_moon_jde_1948_wide_seed_scan() {
        // Scans `nearest_new_moon_jde` across a wide range of seeds
        // spanning all of November and December 1948 (run with
        // `--nocapture` to see the raw trace). Before the K-rounding fix
        // in `nearest_new_moon_jde`, this showed the result flipping
        // from the correct 1948-12-01 new moon to the wrong 1948-12-30
        // one between seeds only two days apart (1948-11-25 vs
        // 1948-11-27) -- the exact discontinuity the fix targets. Kept
        // as a permanent regression guard against that class of bug.
        let ws0 = winter_solstice_jde(1948).expect("winter_solstice_jde(1948) returned None");
        eprintln!("ws0 jde = {ws0} ({:?})", jde_to_cst_date(ws0));

        // Nov 1, 1948 through Jan 5, 1949, every 2 days.
        let start = ws0 - 51.0; // ~ Nov 1
        let mut seed = start;
        while seed < ws0 + 15.0 {
            let result = nearest_new_moon_jde(seed);
            eprintln!(
                "seed={seed} ({:?})  ->  nearest_new_moon_jde = {result:?} ({:?})",
                jde_to_cst_date(seed),
                result.and_then(jde_to_cst_date)
            );
            seed += 2.0;
        }
    }

    #[test]
    fn sui_months_1948_step_by_step_regression() {
        // Reproduces `sui_months(1948)` one internal step at a time,
        // checking the CST *civil date* of each intermediate value
        // against a Python (`pymeeus`) reference computed the same way.
        // This is the exact chain of calls that surfaced the
        // `new_moon_on_or_before`/`next_new_moon_after` seeding bug (see
        // `new_moon_on_or_before_ws0_1948_regression`) -- kept as a
        // thorough regression test for the whole 1948 leap-suì
        // computation (13 months, leap month 7) now that it's fixed.
        let ws0 = winter_solstice_jde(1948).expect("winter_solstice_jde(1948) returned None");
        assert_eq!(jde_to_cst_date(ws0), NaiveDate::from_ymd_opt(1948, 12, 22), "ws0 civil date");

        let ws1 = winter_solstice_jde(1949).expect("winter_solstice_jde(1949) returned None");
        assert_eq!(jde_to_cst_date(ws1), NaiveDate::from_ymd_opt(1949, 12, 22), "ws1 civil date");

        let m_minus1 = new_moon_on_or_before_civil(ws0).expect("new_moon_on_or_before_civil(ws0) returned None");
        assert_eq!(jde_to_cst_date(m_minus1), NaiveDate::from_ymd_opt(1948, 12, 1), "m_minus1 civil date");

        let m11 = new_moon_on_or_before_civil(ws1).expect("new_moon_on_or_before_civil(ws1) returned None");
        assert_eq!(jde_to_cst_date(m11), NaiveDate::from_ymd_opt(1949, 12, 20), "m11 civil date");

        // Reference civil dates for moons[0..=13] (14 entries, 13
        // complete months), from the Python prototype.
        let expected_moon_dates = [
            (1948, 12, 1),
            (1948, 12, 30),
            (1949, 1, 29),
            (1949, 2, 28),
            (1949, 3, 29),
            (1949, 4, 28),
            (1949, 5, 28),
            (1949, 6, 26),
            (1949, 7, 26),
            (1949, 8, 24),
            (1949, 9, 22),
            (1949, 10, 22),
            (1949, 11, 20),
            (1949, 12, 20),
        ];

        let mut moons = vec![m_minus1];
        let mut guard = 0;
        while *moons.last().unwrap() < m11 - 1.0 {
            let nxt = next_new_moon_after(*moons.last().unwrap())
                .unwrap_or_else(|| panic!("next_new_moon_after failed at moons[{}]", moons.len() - 1));
            moons.push(nxt);
            guard += 1;
            assert!(guard <= 40, "moons-building loop exceeded 40 iterations, moons so far: {}", moons.len());
        }

        assert_eq!(moons.len(), expected_moon_dates.len(), "wrong number of moons collected: {moons:?}");
        for (i, (&jde, &(y, m, d))) in moons.iter().zip(expected_moon_dates.iter()).enumerate() {
            assert_eq!(jde_to_cst_date(jde), NaiveDate::from_ymd_opt(y, m, d), "moons[{i}]");
        }

        let zq_dates = zhongqi_civil_dates_from(ws0, m11).expect("zhongqi_civil_dates_from returned None");
        let expected_zq_dates = [
            (1948, 12, 22),
            (1949, 1, 20),
            (1949, 2, 19),
            (1949, 3, 21),
            (1949, 4, 20),
            (1949, 5, 21),
            (1949, 6, 22),
            (1949, 7, 23),
            (1949, 8, 23),
            (1949, 9, 23),
            (1949, 10, 24),
            (1949, 11, 22),
            (1949, 12, 22),
            (1950, 1, 20),
        ];
        assert_eq!(zq_dates.len(), expected_zq_dates.len(), "wrong number of zhongqi collected: {zq_dates:?}");
        for (i, (&d, &(y, m, day))) in zq_dates.iter().zip(expected_zq_dates.iter()).enumerate() {
            assert_eq!(d, NaiveDate::from_ymd_opt(y, m, day).unwrap(), "zhongqi[{i}]");
        }

        // Finally, the full function under test.
        let result = sui_months(1948);
        assert!(result.is_some(), "sui_months(1948) returned None even though every intermediate value matched the Python reference");
    }

    #[test]
    fn sui_months_1948_is_a_thirteen_month_leap_sui_with_leap_month_7() {
        let months = sui_months(1948).expect("sui_months(1948) returned None");
        assert_eq!(months.len(), 13, "expected 13 months, got {months:?}");
        let leap: Vec<_> = months.iter().filter(|m| m.1).collect();
        assert_eq!(leap.len(), 1, "expected exactly one leap month, got {leap:?}");
        assert_eq!(leap[0].0, 7, "expected leap month 7, got {:?}", leap[0]);
        assert_eq!(leap[0].2, NaiveDate::from_ymd_opt(1949, 8, 24).unwrap());
    }

    #[test]
    fn chinese_new_year_1949_and_1950_match_reference_dates() {
        assert_eq!(
            chinese_new_year(1949),
            NaiveDate::from_ymd_opt(1949, 1, 29),
            "chinese_new_year(1949) mismatch or None"
        );
        assert_eq!(
            chinese_new_year(1950),
            NaiveDate::from_ymd_opt(1950, 2, 17),
            "chinese_new_year(1950) mismatch or None"
        );
    }

    #[test]
    fn chinese_year_months_1949_has_thirteen_months_with_leap_7_and_month_11_on_dec_20() {
        // Chinese Year 1949 has a leap 7th month (闰七月, starting
        // 1949-08-24), confirmed against the Hong Kong Observatory's
        // published Gregorian-Lunar conversion table for 1949
        // (hko.gov.hk/en/gts/time/calendar/text/files/T1949e.txt), which
        // lists two consecutive "7th Lunar month" entries: 1949-07-26
        // and 1949-08-24. This test previously asserted 12 months (a
        // mistake made when it was first written, not a code bug --
        // 1948's leap suì genuinely does place its leap month within
        // Chinese Year 1949's 1-10 range, giving 1949 thirteen months
        // total).
        let months = chinese_year_months(1949).expect("chinese_year_months(1949) returned None");
        assert_eq!(months.len(), 13, "expected 13 months, got {months:?}");
        let leap: Vec<_> = months.iter().filter(|m| m.1).collect();
        assert_eq!(leap.len(), 1, "expected exactly one leap month, got {leap:?}");
        assert_eq!(leap[0].0, 7, "expected leap month 7, got {:?}", leap[0]);
        assert_eq!(leap[0].2, NaiveDate::from_ymd_opt(1949, 8, 24).unwrap());
        let m11 = months.iter().find(|m| m.0 == 11).expect("month 11 missing");
        assert_eq!(m11.2, NaiveDate::from_ymd_opt(1949, 12, 20).unwrap());
        let m12 = months.iter().find(|m| m.0 == 12).expect("month 12 missing");
        assert_eq!(m12.2, NaiveDate::from_ymd_opt(1950, 1, 18).unwrap());
    }

    #[test]
    fn to_chinese_ymd_1950_01_01_matches_reference() {
        let d = NaiveDate::from_ymd_opt(1950, 1, 1).unwrap();
        let result = to_chinese_ymd(d);
        assert_eq!(result, Some((1949, 11, false, 13)), "to_chinese_ymd(1950-01-01) = {result:?}");
    }
}
