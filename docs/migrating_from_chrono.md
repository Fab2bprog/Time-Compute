# Migrating from `chrono`

`time_compute`'s core API matches `chrono`'s 1:1 -- same types, same method names, same signatures, same observable behavior (see `docs/API_Reference.md`, and `docs/why_not_chrono.md` for the tens-of-thousands-of-comparisons differential testing behind that claim). For the overwhelming majority of projects, migrating is a five-minute mechanical change, not a rewrite: swap the dependency, swap the import path, compile, done.

## The migration, in three steps

### 1. Swap the dependency

```toml
# Before
[dependencies]
chrono = { version = "0.4", features = ["serde"] }

# After
[dependencies]
time_compute = { version = "1.0", features = ["serde"] }
```

### 2. Swap the import path

Every `use chrono::...` becomes `use time_compute::...`. Nothing else in the line changes:

```rust
// Before
use chrono::{NaiveDate, DateTime, Utc, TimeZone, Datelike};

// After
use time_compute::{NaiveDate, DateTime, Utc, TimeZone, Datelike};
```

A single project-wide find-and-replace of `chrono::` -> `time_compute::` (and `use chrono` -> `use time_compute`) handles this across an entire codebase in one pass, since type names, method names, and call syntax are all unchanged.

### 3. Map any optional features you use

| `chrono` feature | `time_compute` equivalent | Notes |
|---|---|---|
| `serde` | `serde` | Identical purpose and API. |
| `rkyv` / `rkyv-16` / `rkyv-32` / `rkyv-64` / `rkyv-validation` | same names | Same mutual-exclusivity rules as `chrono`'s own `rkyv-*` features. |
| `unstable-locales` | `unstable-locales` | Same name, same `pure-rust-locales` backend. |
| `arbitrary` | `arbitrary` | Identical purpose and API. |
| `defmt` | `defmt` | Identical purpose and API. |
| `std`, `alloc`, `clock`, `now`, `oldtime`, `wasmbind`, `libc`, `winapi`, `core-error` | *(none)* | See "Before you migrate" below. |

Then build:

```sh
cargo build
cargo test
```

Because the frozen surface matches `chrono` behaviorally, not just syntactically, code that compiled and passed its own tests against `chrono` is expected to keep passing against `time_compute` without further changes.

## Before / after: common patterns

**Constructing and formatting a date**

```rust
// Before (chrono)
use chrono::{NaiveDate, Datelike};
let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
println!("{}", date.format("%A %d %B %Y"));

// After (time_compute) -- identical, only the import changed
use time_compute::{NaiveDate, Datelike};
let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
println!("{}", date.format("%A %d %B %Y"));
```

**A time-zone-aware instant**

```rust
// Before (chrono)
use chrono::{TimeZone, Utc};
let meeting = Utc.with_ymd_and_hms(2026, 7, 13, 14, 0, 0).unwrap();

// After (time_compute)
use time_compute::{TimeZone, Utc};
let meeting = Utc.with_ymd_and_hms(2026, 7, 13, 14, 0, 0).unwrap();
```

**Duration arithmetic**

```rust
// Before (chrono)
use chrono::{NaiveDate, Duration};
let next_week = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap() + Duration::days(7);

// After (time_compute)
use time_compute::{NaiveDate, Duration};
let next_week = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap() + Duration::days(7);
```

**`serde` (with the `serde` feature enabled on both sides)**

```rust
// Before (chrono)
#[derive(serde::Serialize, serde::Deserialize)]
struct Event { starts_at: chrono::DateTime<chrono::Utc> }

// After (time_compute)
#[derive(serde::Serialize, serde::Deserialize)]
struct Event { starts_at: time_compute::DateTime<time_compute::Utc> }
```

In every case, the change is confined to the type's namespace. Business logic, trait bounds, generic code written against `Datelike`/`Timelike`/`TimeZone`, and every arithmetic or formatting call site are untouched.

## Before you migrate: one thing to check

`time_compute` currently targets `std` environments only; it does not yet offer `chrono`'s `no_std`/`alloc`-only configuration (`chrono`'s `std`, `alloc`, `clock`, `now`, `oldtime`, `wasmbind`, `libc`, `winapi`, and `core-error` features have no `time_compute` equivalent for that reason). This affects only projects that build `chrono` with `default-features = false` for an embedded or `wasm` target without `std`. If your project uses `chrono` normally (the common case: default features, or `default-features = false` plus `std`-compatible features like `serde`), this does not apply to you and the three steps above are the whole migration.

## What you gain, at no extra migration cost

Once migrated, every extension in `docs/API_Reference.md` (Part 2) is already available -- Christian movable feasts, the Hebrew and Hijri calendars, the Japanese era system and its festivals, the Chinese lunisolar calendar, the Thai/Buddhist calendar, Matariki, and age calculation -- with no additional dependency and no additional setup. See `docs/Use_Example.md` for runnable examples, and `docs/why_not_chrono.md` for why that matters for holiday-, payroll-, and scheduling-driven systems specifically. You do not have to use any of it to benefit from migrating; it is simply there the moment you do.
