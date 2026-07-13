# About Dependencies

`time_compute` exists to be a from-scratch, near-zero-dependency alternative to `chrono`, with the same API. This document explains exactly what that means in practice: which dependencies the crate actually pulls in, why each one exists, and -- most importantly -- which specific part of the code each one touches. The goal is that a reader can verify for themselves that the "minimal dependencies" claim is not just marketing: it is possible to point at every single dependency and name the one file (or handful of functions) it is responsible for.

## The rule

The default assumption for any new code in this crate is **zero dependencies**. A dependency is only ever added when a task is either impossible to do correctly without it (an external, actively maintained data source or a real scientific computation), or when it is entirely opt-in behind a Cargo feature that a user must deliberately enable. Every exception below was a deliberate, one-at-a-time decision, not a default.

## The core: zero dependencies

The large majority of the crate has no dependency at all, direct or transitive. This includes:

- All proleptic Gregorian calendar math (`calendar.rs`): the civil-date/day-count conversion, leap years, ISO 8601 week numbers, weekday arithmetic.
- The three naive (time-zone-less) types: `NaiveDate`, `NaiveTime`, `NaiveDateTime`, and their supporting types (`NaiveWeek`, `IsoWeek`, the day/week iterators).
- Durations and calendar increments: `TimeDelta`/`Duration`, `Days`, `Months`.
- `Weekday`, `WeekdaySet`, `Month`, the `Datelike`/`Timelike` traits.
- Rounding/truncating (`round.rs`).
- Two of the three time zone implementations: `Utc` and `FixedOffset`.
- The entire `strftime`-style formatting and parsing engine (`format/` module): the specifier table, the `Item`/`Parsed`/`DelayedFormat` machinery, RFC 2822/3339 reading and writing.
- Every `time_compute`-only calendar extension **except** two (see below): the Christian movable feasts (Easter and its relatives), the Hebrew calendar, the Hijri/Islamic tabular calendar, the Japanese era system and its fixed-date festivals, the Thai/Buddhist lunisolar calendar (`buddhist_calendar.rs`), and Matariki (`matariki.rs`, a static lookup table). All of these are pure integer (or, for the fixed calendar dates, no) arithmetic.

If a dependency were removed from `Cargo.toml` entirely, everything listed above would still compile and work exactly as it does today -- because none of it touches a dependency in the first place.

## The two unconditional dependencies

Two dependencies are compiled in even with every optional Cargo feature disabled, because the functionality they back genuinely cannot be done correctly from scratch (or, in one case, at all reasonably).

### `tzdb` and `tz-rs` -- the `Local` time zone only

**What they are:** `tzdb` bundles a copy of the IANA Time Zone Database (the "Olson database" that every operating system and serious time library relies on); `tz-rs` reads that data and does the actual UTC-offset/DST-fold/DST-gap resolution for a given instant and time zone.

**Why they're unavoidable:** the historical and present-day UTC offset rules for every real-world time zone (when DST starts/ends in a given country, whether a zone's fixed offset ever changed, ...) are political and administrative facts, not something derivable from any formula. The only correct way to know that "Europe/Paris was UTC+1 on this date, UTC+2 on that one" is to consult an actual, maintained database of those facts. Writing and maintaining that data by hand inside this crate would mean reproducing the IANA database's own multi-decade maintenance effort, badly.

**Where they're used:** exactly one file, `src/offset/local.rs`, which implements the `Local` time zone (the system's local time zone, as returned by `Local::now()` and used by `TimeZone::offset_from_local_datetime`/`offset_from_utc_datetime` for `Local`). Nothing else in the crate references either crate. `Utc` and `FixedOffset` -- the other two time zone types -- have no dependency at all, since a fixed or UTC offset needs no lookup table.

### `astro` -- real astronomical computation, for two specific feature areas

**What it is:** an implementation of Jean Meeus's classical astronomical algorithms (solar and lunar position, equinox/solstice timing, and related computations), including the VSOP87 solar theory.

**Why it's unavoidable (for the functions that use it):** a handful of calendar features are defined by the Sun or Moon's *actual, physical position* at a given moment, not by any arithmetic cycle: a solar term is the instant the Sun's ecliptic longitude crosses a specific degree value; a new moon is the instant of an actual lunar conjunction. There is no closed-form integer formula for either -- reproducing them from scratch would mean re-deriving VSOP87-grade solar/lunar theory, which is exactly the kind of real scientific computation this crate's zero-dependency default is not meant to reinvent.

**Where it's used, precisely:**
- `src/chinese_calendar.rs` -- the private engine backing the Chinese lunisolar calendar (`NaiveDate::chinese_new_year`, `to_chinese_ymd`, `from_chinese_ymd`, `duanwu`, `zhongqiu`, `qingming`). Needs real new-moon timings and the December-solstice/zhongqi solar terms to place lunar months and the leap month correctly.
- Three functions in `src/naive/date.rs`: `NaiveDate::shunbun_no_hi` (Vernal Equinox Day), `shuubun_no_hi` (Autumnal Equinox Day), and `setsubun` (day before Risshun) -- Japan's solar-term public holidays/observances, each defined by a specific solar-longitude crossing.

These are, deliberately, the *only* non-`const fn`, floating-point functions in the entire crate -- a visible marker of exactly how contained this dependency's reach is. Every other calendar extension in the crate (Hebrew, Hijri, Thai/Buddhist, the fixed Japanese festivals) was specifically designed to avoid needing `astro`, using integer mean-motion or tabular arithmetic instead, even where that means being a traditional/legal reckoning rather than a true astronomical one.

## The five opt-in dependencies (disabled by default)

None of these are compiled in unless the consumer of the crate explicitly turns on the matching Cargo feature. Each mirrors an optional `chrono` feature of the same name and purpose, so migrating a project that already enables one of these in `chrono` is a drop-in change.

- **`serde` (feature `serde`)** -- (de)serialization. Adds `Serialize`/`Deserialize` implementations to `NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Tz>`, `Weekday`, `Month`, and `TimeDelta`, plus the `time_compute::serde` and `time_compute::naive::serde` helper modules (`ts_seconds`, `ts_milliseconds`, `ts_microseconds`, `ts_nanoseconds`, and their `_option` variants) for (de)serializing as a Unix timestamp instead of a string. Touches only the `serde_impl` submodules scattered next to each type's definition, plus `datetime.rs`'s and `naive/datetime.rs`'s `serde` modules -- no other code path is affected.
- **`rkyv` / `rkyv-16` / `rkyv-32` / `rkyv-64` (mutually exclusive; `rkyv-validation` adds safety-checked deserialization)** -- zero-copy (de)serialization. Adds `#[derive(Archive, Serialize, Deserialize)]` to the same set of types as `serde` above (plus a few more: `FixedOffset`, `Utc`, `Local`, `IsoWeek`), and re-exports their `Archived*` counterparts from `time_compute::rkyv`. Touches only the `#[cfg_attr(any(feature = "rkyv", ...))]` derive attributes already present on each type -- no separate implementation code.
- **`unstable-locales` (pulls in `pure-rust-locales`)** -- locale-aware formatting. Supplies month/weekday names and the `%x`/`%X`/`%c`/`%r` formats in languages other than English/POSIX. Touches exactly `src/format/locales.rs` (which switches from a hard-coded English-only table to `pure-rust-locales`'s data) and the `format_localized`/`format_localized_with_items`/`StrftimeItems::new_with_locale` methods that consume it. Named `unstable-locales`, matching `chrono`'s own choice of name, because the output can shift with a `pure-rust-locales` version bump as it corrects locale data over time -- this feature does not carry the same API stability guarantee as the rest of the crate.
- **`arbitrary`** -- support for the `arbitrary::Arbitrary` trait, used by fuzz testing to generate random-but-valid instances of a type instead of raw random bytes. Touches only the `impl arbitrary::Arbitrary for ...` blocks next to `NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Tz>`, `FixedOffset`, `TimeDelta`, and `Months`.
- **`defmt`** -- compact, `Debug`-like output formatting designed for embedded/`no_std` targets (where the regular `core::fmt` machinery is too heavyweight for constrained devices). Touches only the `impl defmt::Format for ...` blocks next to nearly every public type.

Each of these five is a one-line `Cargo.toml` addition on the consumer's side, and adding none of them at all is the default -- a plain `cargo add time_compute` (or the equivalent `Cargo.toml` entry with no `features = [...]`) pulls in only the two unconditional dependencies above.

## `serde_json` -- test-only, not part of the published crate

One more dependency appears in `Cargo.toml`, but only under `[dev-dependencies]`: `serde_json`. It is used exclusively inside this crate's own `#[cfg(test)] mod tests` blocks (the `serde_round_trip`-style tests gated behind the `serde` feature) to check that a value survives a JSON round trip. Because it is a dev-dependency, Cargo never compiles it into the published library or pulls it into a consumer's build. `time_compute` has no dependency, direct or transitive, on `chrono` in any configuration -- `chrono` is not part of this crate.

## Summary table

| Dependency | Always compiled? | Exact scope | Purpose |
|---|---|---|---|
| `tzdb` | Yes | `src/offset/local.rs` only | IANA time zone database |
| `tz-rs` | Yes | `src/offset/local.rs` only | Resolves offsets/DST from that database |
| `astro` | Yes | `src/chinese_calendar.rs`; 3 functions in `src/naive/date.rs` | Real solar/lunar position (Meeus' algorithms) |
| `serde` | Only with feature `serde` | `serde_impl` submodules + `*::serde` helper modules | Text/JSON (de)serialization |
| `rkyv` | Only with feature `rkyv`/`rkyv-16`/`rkyv-32`/`rkyv-64` | Derive attributes only | Zero-copy (de)serialization |
| `pure-rust-locales` | Only with feature `unstable-locales` | `src/format/locales.rs` + `format_localized*` methods | Non-English locale data |
| `arbitrary` | Only with feature `arbitrary` | `Arbitrary` impls only | Fuzz-testing input generation |
| `defmt` | Only with feature `defmt` | `defmt::Format` impls only | Embedded/no_std-friendly debug output |
| `serde_json` | Dev-only, never shipped | `#[cfg(test)]` blocks only | JSON round-trip test assertions |

Everything not in this table -- which is to say, the great majority of the crate -- has no dependency at all.
