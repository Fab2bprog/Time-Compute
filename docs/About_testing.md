# About Testing

`time_compute` is trusted to do calendar math correctly, not just to compile. This document explains the testing philosophy behind the crate and the two independent, complementary layers that backed its development: a large body of unit tests written directly against external ground truth (permanent, and what continues to guarantee correctness today), and a purpose-built, dev-only differential harness that, during development, compared every core behavior against the real `chrono` crate input by input to catch behavioral divergences before release. That harness has since been removed from the project entirely, along with `chrono` as a dev-dependency -- `chrono` is not, and has never been, a dependency of the published `time_compute` library, in any configuration. This document also gives the precise numbers behind both layers -- how many tests, how many comparisons were run historically, and exactly what each one found -- as a record of how the crate's correctness was established.

## Two different questions, two different tools

Two distinct risks exist in a project like this one, and no single kind of test catches both:

1. **Is the algorithm itself correct?** A calendar algorithm (Easter, the Hebrew *molad*, a Chinese new-moon computation, ...) can be internally consistent and still be *wrong* -- it needs to be checked against an authority outside the code itself: a published festival table, an astronomical observatory, an independently written reference implementation.
2. **Does the chrono-compatible surface actually behave like chrono?** Since the entire point of the frozen core API is drop-in compatibility, the only real test of that claim is running both libraries on the same input and diffing the output.

Unit tests answer question 1, and remain part of the crate today. The differential harness described below answered question 2 during development, as a one-time verification step; it is not part of the crate and never has been a dependency of it. Between the two, every public function was checked against something other than its own logic before release.

## Layer 1 -- unit tests

**Where they live:** every source file ends in a `#[cfg(test)] mod tests { use super::*; ... }` block, testing the code directly above it in the same file. There is no separate `tests/` integration-test tree for this layer -- keeping a function and its tests in the same file has kept the two from drifting apart as the crate grew.

**How many:** **492 `#[test]` functions**, spread across every module in `src/`. The heaviest-tested files are, unsurprisingly, the largest and most calendrically complex ones:

| File | Unit tests |
|---|---|
| `naive/date.rs` | 83 |
| `datetime.rs` | 50 |
| `duration.rs` | 38 |
| `naive/time.rs` | 28 |
| `format/formatting.rs`, `format/parsed.rs`, `format/scan.rs`, `naive/datetime.rs` | 26 each |
| `format/parse.rs` | 21 |
| `format/strftime.rs`, `weekday.rs` | 18 each |
| `month.rs` | 16 |
| `offset/mod.rs` | 14 |
| `calendar.rs`, `round.rs` | 12 each |
| `offset/fixed.rs`, `weekday_set.rs` | 11 each |
| `naive/week.rs` | 9 |
| `buddhist_calendar.rs` | 8 |
| `chinese_calendar.rs`, `naive/iter.rs`, `traits.rs` | 7 each |
| `format/mod.rs`, `offset/local.rs` | 5 each |
| `offset/utc.rs` | 4 |
| `format/locales.rs`, `matariki.rs` | 3 each |

All 492 pass under `cargo test`, confirmed directly by Fabrice.

**What each kind of unit test actually checks against:**

- **Reference-date tests** -- a hand-picked, independently sourced list of known-correct outputs. This is the backbone of every calendar extension: Easter dates cross-checked against published tables; Hebrew holiday dates checked against hebcal.com; Hijri dates checked against known Gregorian equivalents; Japanese era boundaries checked against the dates used in ICU/Unicode CLDR; Chinese new year and leap-month years checked against the Hong Kong Observatory's official published calendar; Thai/Buddhist festival dates checked against the `pythaidate` reference implementation and independently published tables; Matariki dates transcribed and cross-checked against the Museum of New Zealand Te Papa Tongarewa's and RNZ's published tables.
- **Wide-range round-trip properties** -- e.g. converting tens of thousands of consecutive dates to the Chinese calendar and back, or exercising every day of both a leap and a non-leap year through `from_yo_opt`/`ordinal`, and checking the result always reconstructs the original input.
- **Invariant checks** -- properties that must hold regardless of the specific date, such as "the four Thai Buddhist festivals always occur in the same relative month order every year" or "Khao Phansa is always exactly the day after Asalha Bucha."
- **Boundary and edge-case tests** -- behavior exactly at `NaiveDate::MIN`/`MAX`, leap seconds, DST folds/gaps for `Local`, the exact numeric edges of `TimeDelta`'s representable range (several real overflow bugs in `TimeDelta`'s microsecond/nanosecond/millisecond constructors were caught this way, at exactly `i64::MIN`/`i64::MAX`/`-i64::MAX`).

Unit tests are what catch a **wrong algorithm**. They say nothing about whether the result matches chrono's own behavior for the functions meant to mirror it -- that is the differential harness's job.

## Layer 2 -- historical differential testing against real `chrono` (development-only, now removed)

### What it was and how it was built

During development, `examples/differential_check.rs` was a small, self-contained Rust binary (not a `#[test]`, so it never ran on a plain `cargo test`) that linked against the real, published `chrono` crate as a `dev-dependency` -- meaning it was never compiled into the published `time_compute` library, only into this one throwaway diagnostic binary, run explicitly with `cargo run --example differential_check`. Once it had served its purpose, both the file and `chrono` as a dev-dependency were removed from the project entirely. `time_compute` has no dependency on `chrono`, direct or transitive, in any configuration -- it never appears in the published crate's dependency tree, and this section is kept only as a historical record of how the crate's chrono-compatible surface was verified before release. (`serde_json` and `serde`'s `derive` feature remain as separate dev-dependencies, unrelated to this tool -- they back two of this crate's own permanent unit tests; see `docs/About_dependencies.md`.)

The mechanism was deliberately simple: for a given functionality, generate an input, run it through both `chrono` and `time_compute`, format both results as a string (via `Display`/`Debug`, or a small ad-hoc formatter where needed), and record whether the two strings were identical. Comparing as strings rather than writing a bespoke equality check per type is what let the harness compare two structurally unrelated (but same-named) types with a single, generic `Report::push` helper.

Randomized inputs came from a tiny, hand-written, **deterministic** PRNG (a splitmix64 generator, seeded with the fixed constant `0xC0FF_EE12_3456_789A`), rather than pulling in a `rand` dependency for what was explicitly throwaway dev tooling. This mattered for reproducibility: the same seed produced the exact same sequence of test inputs on every run, so a divergence found once could always be reproduced.

### Exactly what was exercised, and how much of it

At its final form, the harness ran **25 test functions**, each covering one functionality or a closely related group of them:

`NaiveDate` (validity, all `Datelike` accessors, `succ_opt`/`pred_opt`, `checked_add/sub_days/months`, `signed_duration_since`), `NaiveTime` (all `Timelike` accessors, `hour12`, arithmetic with `TimeDelta`), `NaiveDateTime` (`Display`, `and_utc().timestamp()`, arithmetic), `DateTime<Utc>`/`DateTime<FixedOffset>` and formatting (`Display`, RFC 2822/3339 output, 15 representative `strftime` format strings, a format-then-parse round trip), `Weekday`, `Month`, `Month::num_days` (deliberately including 1900/2000/2004/-400/-401 to exercise the Gregorian century leap-year exception), `TimeDelta` accessors, `FixedOffset`, `Local` (compared only at fixed UTC instants, never via two independent `now()` calls, since two clock reads a few microseconds apart aren't meaningfully comparable), `NaiveDate`/`NaiveTime` mutators (`with_year`/`with_month`/.../`with_hour`/.../`with_nanosecond`, including intentionally out-of-range replacement values to exercise the rejection path too), a systematic ISO-week sweep (below), the `round` module (`SubsecRound`, `DurationRound`, including two deliberately invalid spans), additional string parsing (`parse_from_str` round trips, `DateTime::parse_from_rfc2822`/`parse_from_rfc3339`), `DateTime<Utc>` arithmetic, `WeekdaySet`, `serde` serialization (`NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Utc>`, `TimeDelta`, plus every `Weekday` and `Month` value exhaustively), comparison operators (`<`, `>`, `==`), direct `TimeDelta` arithmetic operators, deliberately malformed parse inputs (bad month/day/hour/minute, empty/truncated/garbage strings -- only `is_err()` agreement is checked, not the exact error text), boundary years near chrono's own exact internal limits, `NaiveWeek`, timestamp variants (`timestamp_millis`/`timestamp_micros`/`timestamp_nanos_opt` and their round trip), and `DateTime<FixedOffset>` arithmetic.

Most of these functions loop over a fixed number of randomized inputs, each producing several comparisons per successful input (a validity check, then one comparison per accessor/method exercised on that input). The iteration counts themselves are fixed, named constants: 500 draws each for `NaiveDate` and `NaiveTime`, 300 each for `NaiveDateTime`, `DateTime<Utc>`-and-formatting, the mutator tests, the `round` module, additional parsing, `DateTime` arithmetic, `WeekdaySet`, comparisons, `TimeDelta` arithmetic, `NaiveWeek`, timestamp variants, and `FixedOffset` datetime arithmetic; 250 for `TimeDelta`; 200 for `FixedOffset` and (when enabled) `serde`.

**The systematic ISO-week sweep** (`test_iso_week_systematic`) is the one part of the harness that is deliberately *not* random. It exhaustively checks the first four days of January and the last four days of December for every year from -3,000 to 3,000 -- 6,001 consecutive years, 8 dates each, exactly **48,008 comparisons** from this one function alone. The reasoning: the Gregorian calendar's weekday/leap-year pattern repeats exactly every 400 years (146,097 days, a whole multiple of 7), so this range covers that repetition cycle roughly fifteen times over in each direction, guaranteeing every one of the seven possible "what weekday does January 1st fall on" alignments is hit many times over -- rather than hoping a random draw happens to land on the rare alignment that actually matters (which is exactly what had let a real bug slip through 300 earlier random `NaiveDate` draws the first time around).

### The real runs, and what they found

The harness was actually executed by Fabrice three times over the course of development, each time after a round of extensions:

1. **First real run, initial version of the harness:** **12,862 comparisons**, **1 divergence**: `NaiveDate::iso_week()` for `y=35814 m=1 d=2` -- chrono returned `35813-W52`, this crate returned `35814-W01`. Manual verification (an independent Zeller's-congruence check, anchored against the known fact that January 1, 2000 was a Saturday) confirmed chrono was right. **Root cause**: `src/calendar.rs`'s `iso_year_week` used Rust's default `/` operator, which truncates toward zero, on a numerator that can be negative (`(z - iso_week1_monday(year)) / 7`) -- for a negative numerator such as `-1`, truncating division gives `0` instead of the mathematically correct `-1`, silently breaking the "does this date actually belong to the previous ISO year's last week" check for any date in the first few days of January of a year whose January 1st falls on a Friday, Saturday, or Sunday (roughly 3 years out of 7) -- a broad-reaching bug that a single 300-draw random sample had only managed to trip once, by chance. **Fixed** by replacing `/` with `.div_euclid(7)` (floor division). Re-run after the fix: **0 divergences on the same 12,862 comparisons.**

2. **Second real run, after extending the harness with more accessors, mutators, the systematic ISO-week sweep, and several other categories (bringing the total comparison volume into the tens of thousands, ~48,000 from the ISO-week sweep alone):** **2 divergences**, both the same case: `secs=41967621435 nsecs=235443862 offset=-86378` (a `FixedOffset` of `-23:59:38`, 22 seconds short of the absolute `-24:00:00` limit `FixedOffset` cannot represent). Investigation traced this to a real bug in `chrono` 0.4.45 (the version actually published on crates.io, confirmed via `Cargo.lock`): its RFC 3339/`%:z` offset formatting rounds seconds to the nearest minute without ever checking whether that rounding pushes the result to exactly `-24:00`, so chrono fails to even re-parse its own formatted output in this edge case. `time_compute`'s own `OffsetFormat` code already contains the fix chrono itself has written on its own development branch (not yet published) -- confirmed by comparing against `chrono`'s in-progress source. **Conclusion: no change needed in this crate** -- it already behaves strictly better than the currently published chrono (always producing valid, re-parseable output). This is documented as a permanent, accepted, deliberate divergence, not a bug to fix here.

3. **Third real run, after a further extension bringing the harness to its final 25 test functions** (adding `serde` comparisons, comparison operators, direct `TimeDelta` operators, invalid-input parsing, boundary years, `NaiveWeek`, timestamp variants, and `FixedOffset` datetime arithmetic): **202 divergences**. 2 were the already-known, already-accepted `FixedOffset`/`-24:00` case above. The other **200 were every single `serde` comparison for `NaiveDateTime`** (`SERDE_ITERATIONS = 200`, a 1:1 match confirming the bug was systemic, not incidental). **Root cause**: `src/naive/datetime.rs`'s `Serialize` implementation for `NaiveDateTime` serialized via `Display` (which separates the date and time with a space: `"2024-03-04 09:38:16..."`), but `Deserialize`/`FromStr` require a literal `"T"` at that position -- so every serialize-then-deserialize round trip of a `time_compute` `NaiveDateTime` failed. Chrono has no such bug because its own `Serialize` goes through `Debug` (which already uses `"T"`), consistent with its own `FromStr`. Checked for the same risk elsewhere (`NaiveDate`, `NaiveTime`, `Weekday`) and found none, since in each of those types `Debug` and `Display` already agree on their separator -- confirmed by the report itself showing zero divergences on those types' own `serde` comparisons. **Fixed** by serializing `NaiveDateTime` through `Debug`'s formatting instead of `Display`'s. Re-run after the fix: only the 2 known, accepted `FixedOffset` divergences remained; the 200 `NaiveDateTime` serde divergences were gone.

### Where that left things

At its final, 25-function form, the differential harness ran a total volume of comparisons in the **tens of thousands** (with the systematic ISO-week sweep alone contributing 48,008 of them), covering essentially the entire chrono-compatible surface of the crate: every naive type's accessors and mutators, `DateTime<Tz>` across all three time zone types, arithmetic, formatting, parsing (valid and deliberately invalid), rounding, comparisons, `serde`, and `WeekdaySet`. It found and fixed two real bugs in this crate (`iso_year_week`'s truncating division, and `NaiveDateTime`'s `serde`/`FromStr` separator mismatch), surfaced one real, already-acknowledged bug in `chrono` itself (kept as a documented, permanent, deliberate divergence rather than "fixed" here), and, on its final run, reported no other divergence.

That clean final run was the harness's exit condition: once it had confirmed the chrono-compatible surface matched real chrono's behavior with no remaining divergence, it had served its purpose. The file (`examples/differential_check.rs`) and `chrono` as a dev-dependency have since been removed from the project entirely, on Fabrice's explicit order. Nothing in the published `time_compute` crate depends on `chrono`, then or now -- this section is retained purely as a historical record of how the crate's correctness was established. It never said anything about the calendar extensions in Part 2 of `docs/API_Reference.md` (Hebrew, Hijri, Chinese, Thai/Buddhist, Japanese, Matariki): chrono has no equivalent behavior there to compare against, which is exactly why those rely entirely on the unit-test layer described above.
