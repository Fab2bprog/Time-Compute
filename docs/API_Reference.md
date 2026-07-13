# `time_compute` API Reference

This document is an exhaustive, function-by-function reference of `time_compute`'s public API. It explains what each item does and why it exists, without usage examples (see `docs/Use_Example.md` for worked examples, and the crate's rustdoc/doctests for runnable code).

The reference is split in two parts, matching the crate's own design split (see `docs/Architecture.md`):

- **Part 1** covers the API that mirrors `chrono` 1:1 -- same names, same signatures, same behavior, organized by type/module.
- **Part 2** covers the `time_compute`-only extensions: everything with no `chrono` equivalent, each marked in the source with a `# time_compute extension -- not part of chrono` comment.

Deprecated methods (kept only for API parity with older `chrono` releases, each pointing to its non-deprecated replacement) are listed alongside their replacement rather than in a separate section, since that is how a reader will actually encounter them.

---

# Part 1 -- The chrono-compatible API

## 1. `NaiveDate`

A date in the proleptic Gregorian calendar (year, month, day) with no time-of-day or time zone. Internally a `(year: i32, month: u32, day: u32)` triple; all arithmetic routes through `calendar.rs`'s civil-date/day-count conversion.

**Constants**

- `MIN` / `MAX` -- the smallest and largest representable dates. The accepted year range is `-5,000,000..=5,000,000`, deliberately far wider than chrono's own (chrono tops out around +/-262,000/-400,000 years depending on the exact date); every date chrono accepts is accepted here, plus some extreme dates chrono would reject.
- `MIN_DATE` / `MAX_DATE` -- deprecated aliases of `MIN`/`MAX`.

**Constructors** (each has a panicking form, deprecated in favor of the `_opt` form, and a non-panicking `_opt` form returning `Option<NaiveDate>`)

- `from_ymd_opt(year, month, day)` / `from_ymd` (deprecated) -- from year, month (1-12), day of month. `None`/panics on an invalid or non-existent calendar day (bad month, day out of range, February 29 in a non-leap year, year out of bounds).
- `from_yo_opt(year, ordinal)` / `from_yo` (deprecated) -- from year and day-of-year (1-365 or 366).
- `from_isoywd_opt(iso_year, week, weekday)` / `from_isoywd` (deprecated) -- from an ISO 8601 year, week number (1-53) and weekday. Fails if the combination doesn't correspond to a real date (e.g. week 53 in a year with only 52).
- `from_num_days_from_ce_opt(days)` / `from_num_days_from_ce` (deprecated) -- from the day count since the proleptic 0001-01-01 (day 1 = 0001-01-01).
- `from_epoch_days(days)` -- from the day count since the Unix epoch (1970-01-01 = day 0). Returns `None` if out of range.
- `from_weekday_of_month_opt(year, month, weekday, n)` / `from_weekday_of_month` (deprecated) -- the `n`-th (1-indexed) occurrence of `weekday` in the given month, e.g. the 2nd Friday of March.

**Basic accessors and day-count conversions**

- `to_epoch_days()` -- day count since the Unix epoch.
- `num_days_from_ce()` -- day count since the proleptic 0001-01-01 (also available as `Datelike::num_days_from_ce`, a default trait method with the same formula).
- `leap_year()` -- whether this date's year is a leap year.
- `weeks_from(day)` *(crate-private)* -- internal helper backing the `%U`/`%W` format specifiers.

**Successor/predecessor and day/duration arithmetic**

- `succ_opt()` / `succ` (deprecated) -- the next day, `None`/panics at `MAX`.
- `pred_opt()` / `pred` (deprecated) -- the previous day, `None`/panics at `MIN`.
- `checked_add_days(Days)` / `checked_sub_days(Days)` -- add/subtract a whole number of days, `None` on overflow of the representable range.
- `checked_add_months(Months)` / `checked_sub_months(Months)` -- add/subtract whole months; if the original day of the month doesn't exist in the resulting month (e.g. January 31 + 1 month), clamps to that month's last valid day rather than failing.
- `checked_add_signed(Duration)` / `checked_sub_signed(Duration)` -- add/subtract a signed `Duration`/`TimeDelta`, truncated to a whole number of days.
- `signed_duration_since(other)` -- signed `Duration` between two dates (`self - other`), in whole days.
- `abs_diff(rhs)` -- unsigned `Days` between two dates regardless of order.
- `years_since(base)` -- whole years elapsed from `base` to `self` (anniversary-aware), `None` if `self` is before `base`.
- Operators: `Add<Duration>`, `Sub<Duration>`, `AddAssign`/`SubAssign` (panic on overflow -- use the `checked_*` methods for a non-panicking version); `Add<Months>`/`Sub<Months>`; `Add<Days>`/`Sub<Days>`; `Sub<NaiveDate> -> Duration`.

**`Datelike` trait implementation** -- `year`, `month`, `month0`, `day`, `day0`, `ordinal`, `ordinal0`, `weekday`, `iso_week`, and the `with_year`/`with_month`/`with_month0`/`with_day`/`with_day0`/`with_ordinal`/`with_ordinal0` mutators (each returns `Option<NaiveDate>`, `None` if the resulting date would not exist). See section 8 for the trait's own default methods (`year_ce`, `quarter`, `num_days_from_ce`, `num_days_in_month`).

**Combining with a time of day**

- `and_time(NaiveTime)` -- combines with a `NaiveTime` into a `NaiveDateTime`.
- `and_hms_opt` / `and_hms` (deprecated), `and_hms_milli_opt` / `and_hms_milli` (deprecated), `and_hms_micro_opt` / `and_hms_micro` (deprecated), `and_hms_nano_opt` / `and_hms_nano` (deprecated) -- convenience constructors combining this date with an hour/minute/second (optionally with milli-/micro-/nanosecond precision, which may exceed the normal range to represent a leap second).

**Iteration and week**

- `iter_days()` -- an iterator over `NaiveDate`, stepping one day at a time up to and including `MAX` (`NaiveDateDaysIterator`, see section 5).
- `iter_weeks()` -- same, stepping 7 days at a time (`NaiveDateWeeksIterator`).
- `week(start: Weekday)` -- the `NaiveWeek` (section 4) that this date belongs to, anchored on a chosen starting weekday.

**Parsing and formatting**

- `parse_from_str(s, fmt)` -- parses a `NaiveDate` from a string using a `strftime`-style format (section 13).
- `parse_and_remainder(s, fmt)` -- same, also returning the unparsed remainder of the string.
- `format(fmt)` / `format_with_items(items)` -- returns a `DelayedFormat` (section 13), converted to a string only when actually displayed.
- `format_localized(fmt, locale)` / `format_localized_with_items(items, locale)` -- locale-aware variants, gated behind the `unstable-locales` feature.

**Trait implementations** -- `Display`/`Debug` (`YYYY-MM-DD`, with a `+`/`-` sign and no fixed width for years outside `0..=9999`), `Default` (1970-01-01), `FromStr` (parses `%Y-%m-%d`), `Clone`/`Copy`/`PartialEq`/`Eq`/`PartialOrd`/`Ord`/`Hash`; `serde` (as an ISO 8601 string, feature-gated); `rkyv` (as `ArchivedNaiveDate`, feature-gated); `arbitrary` (feature-gated, for fuzzing); `defmt::Format` (feature-gated).

### 1.1 `IsoWeek`

A week in the ISO 8601 calendar (ISO year + week number), returned by `Datelike::iso_week()`. The ISO year can differ from the plain calendar year for the handful of days at the very start/end of a year that belong to a week counted under the adjacent year.

- `year()` -- the ISO 8601 year.
- `week()` -- the week number, 1 to 52 or 53.
- `week0()` -- the week number, 0-indexed.
- `Display` -- prints as `YYYY-Www`.
- `Clone`/`Copy`/`PartialEq`/`Eq`/`PartialOrd`/`Ord`/`Hash`/`Debug`; `rkyv`/`defmt` support (feature-gated). No `serde` impl, matching chrono (which has none for `IsoWeek` either).

## 2. `NaiveTime`

A time of day (hour, minute, second, nanosecond) with no associated date or time zone. Internally `(secs: u32` since midnight in `0..86_400`, `frac: u32` nanoseconds in `0..1_000_000_000`, or `1_000_000_000..2_000_000_000` to represent a leap second`)`.

**Constants** -- `MIN` (midnight, the earliest representable time).

**Constructors** (each with a deprecated panicking form and a non-panicking `_opt` form)

- `from_hms_opt` / `from_hms` (deprecated) -- from hour/minute/second (no leap second representable this way).
- `from_hms_milli_opt` / `from_hms_milli` (deprecated) -- with a millisecond component (may exceed 1,000 for a leap second, only when `sec == 59`).
- `from_hms_micro_opt` / `from_hms_micro` (deprecated) -- with a microsecond component (same leap-second allowance).
- `from_hms_nano_opt` / `from_hms_nano` (deprecated) -- with a nanosecond component (may go up to 1,999,999,999 for a leap second).
- `from_num_seconds_from_midnight_opt` / `from_num_seconds_from_midnight` (deprecated) -- from the number of seconds since midnight plus a nanosecond remainder.

**Arithmetic**

- `overflowing_add_signed(Duration)` / `overflowing_sub_signed(Duration)` -- add/subtract a signed duration, wrapping around a single day; also returns the signed number of seconds carried into/out of the day (0 if no day boundary was crossed), so a caller can apply that carry to an accompanying date. Never fails.
- `signed_duration_since(rhs)` -- signed duration between two times (`self - rhs`), always within +/-1 day.
- Operators: `Add`/`Sub<Duration>`, `AddAssign`/`SubAssign<Duration>`; `Add`/`Sub<core::time::Duration>` (wraps, like the signed operators); `Sub<NaiveTime> -> Duration`; `Add`/`Sub<FixedOffset>` (wraps around a day, used internally when combining a time with an offset).

**`Timelike` trait implementation** -- `hour`, `minute`, `second` (never reports 60 even for a leap second -- inspect `nanosecond()` or formatting to see it), `nanosecond`, and the `with_hour`/`with_minute`/`with_second`/`with_nanosecond` mutators. See section 8 for the trait's default methods (`hour12`, `num_seconds_from_midnight`).

**Parsing and formatting**

- `parse_from_str(s, fmt)` / `parse_and_remainder(s, fmt)` -- as for `NaiveDate`.
- `format(fmt)` / `format_with_items(items)` -- as for `NaiveDate` (no `format_localized` variant -- a bare time of day has no locale-dependent representation beyond what the date-aware types already cover).

**Trait implementations** -- `Debug`/`Display` (`HH:MM:SS[.fraction]`, a leap second prints as second `60`), `Default` (midnight), `FromStr` (parses `%H:%M:%S%.f` with seconds and the fractional part optional), the usual `Clone`/`Copy`/`PartialEq`/`Eq`/`PartialOrd`/`Ord`/`Hash`; `serde`/`rkyv`/`arbitrary`/`defmt` support (feature-gated), same pattern as `NaiveDate`.

## 3. `NaiveDateTime`

A date and time of day combined, still without an associated time zone. Internally a `(NaiveDate, NaiveTime)` pair.

**Constants** -- `MIN` / `MAX`; `UNIX_EPOCH` (deprecated, use `DateTime::UNIX_EPOCH` instead); `MIN_DATETIME` / `MAX_DATETIME` (deprecated aliases of `MIN`/`MAX`).

**Construction and components**

- `new(date, time)` -- combines a `NaiveDate` and `NaiveTime` (equivalent to `date.and_time(time)`).
- `date()` / `time()` -- the date/time-of-day components.

**Unix-timestamp interop (all deprecated in favor of the `DateTime<Utc>`-based equivalents, since a naive value has no time zone to make a timestamp meaningful without an explicit UTC assumption)**

- `from_timestamp(secs, nsecs)` -- panics on out-of-range input; superseded by `DateTime::from_timestamp`.
- `timestamp()` -- non-leap seconds since the Unix epoch, assuming UTC.
- `from_timestamp_millis` / `timestamp_millis`, `from_timestamp_micros` / `timestamp_micros`, `from_timestamp_nanos` / `timestamp_nanos` / `timestamp_nanos_opt`, `from_timestamp_opt` -- millisecond/microsecond/nanosecond equivalents.
- `timestamp_subsec_millis` / `timestamp_subsec_micros` / `timestamp_subsec_nanos` -- the sub-second remainder.

**Arithmetic**

- `checked_add_signed(TimeDelta)` / `checked_sub_signed(TimeDelta)` -- add/subtract a signed duration, wrapping across day boundaries; `None` on overflow of the representable date range. Leap-second handling: assumes there is no leap second ever, except when `self` itself represents one.
- `checked_add_months(Months)` / `checked_sub_months(Months)` -- shifts the date part only (clamping the day of month as needed), time of day unchanged.
- `checked_add_days(Days)` / `checked_sub_days(Days)` -- shifts the date part by whole days.
- `signed_duration_since(rhs)` -- signed `TimeDelta` between two datetimes, never overflows.
- `checked_add_offset(FixedOffset)` / `checked_sub_offset(FixedOffset)` -- add/subtract a fixed UTC offset (used to convert between local and UTC readings), preserving any leap second (unlike `checked_add_signed`). `None` if the result would be out of `NaiveDateTime`'s range.
- Operators: `Add`/`Sub<TimeDelta>`, `Add`/`Sub<core::time::Duration>`, `AddAssign`/`SubAssign` for both; `Add<FixedOffset>`; `Add`/`Sub<Months>`; `Add`/`Sub<Days>`; `Sub<NaiveDateTime> -> TimeDelta`. All panic on overflow -- prefer the `checked_*` methods for a non-panicking version.
- `From<NaiveDate>` -- a date at midnight.

**Time zone attachment**

- `and_utc()` -- wraps into a `DateTime<Utc>`.
- `and_local_timezone(tz)` -- wraps into a `MappedLocalTime<DateTime<Tz>>` for an arbitrary `TimeZone` implementation.

**`Datelike`/`Timelike` trait implementations** -- delegate to the wrapped `NaiveDate`/`NaiveTime`.

**Parsing and formatting** -- `parse_from_str`, `parse_and_remainder`, `format`, `format_with_items`, as for `NaiveDate`/`NaiveTime` (parsing/formatting both the date and time parts together).

**Trait implementations** -- `Debug` (`YYYY-MM-DDTHH:MM:SS[.fraction]`), `Display` (same but with a space instead of `T`), `Default` (Unix epoch, no time zone), `FromStr` (parses `%Y-%m-%dT%H:%M:%S%.f`), the usual comparison/hash traits; `serde` (RFC 3339-ish string) and the `crate::naive::serde` submodule described in section 14; `rkyv`/`arbitrary`/`defmt` (feature-gated).

## 4. `NaiveWeek`

The calendar week a given `NaiveDate` belongs to, anchored on a chosen starting weekday. Obtained via `NaiveDate::week(start)`.

- `first_day()` / `checked_first_day()` -- the first day of the week (panics vs. returns `None` if that day would fall outside `NaiveDate`'s representable range).
- `last_day()` / `checked_last_day()` -- the last day of the week, same panic/`None` distinction.
- `days()` / `checked_days()` -- the full 7-day range as a `RangeInclusive<NaiveDate>`.
- `PartialEq`/`Eq`/`Hash` -- based on the week's first day, not the originating date used to look it up (two dates in the same week compare equal).

## 5. Iterators over `NaiveDate`

Created by `NaiveDate::iter_days()`/`iter_weeks()`.

- `NaiveDateDaysIterator` -- advances one day at a time. `Iterator`, `DoubleEndedIterator` (steps backward via `pred_opt`), `ExactSizeIterator`, `FusedIterator`. Note: an iterator whose current value is already `NaiveDate::MAX` yields nothing at all (not even `MAX` itself), a quirk inherited from the `?`-based short-circuit in `next()` and confirmed to mirror chrono's own iterator.
- `NaiveDateWeeksIterator` -- same, advancing 7 days at a time.

## 6. `DateTime<Tz>`

A date and time of day together with a time zone, generic over any `TimeZone` implementation (`Utc`, `FixedOffset`, or `Local`). Internally a `(NaiveDateTime` in UTC, `Tz::Offset)` pair.

**Constants** -- `DateTime::<Utc>::MIN_UTC` / `MAX_UTC`; `DateTime::<Utc>::UNIX_EPOCH`; `MIN_DATETIME` / `MAX_DATETIME` (deprecated aliases of `MIN_UTC`/`MAX_UTC`).

**Low-level construction** (prefer `TimeZone::from_local_datetime`/`with_ymd_and_hms`/`timestamp_opt` or `NaiveDateTime::and_local_timezone` for regular use)

- `from_naive_utc_and_offset(datetime, offset)` -- builds directly from a UTC `NaiveDateTime` and an offset.
- `from_utc(datetime, offset)` (deprecated) -- same, older name.
- `from_local(datetime, offset)` (deprecated) -- builds from a *local* `NaiveDateTime` and an offset; panics if converting to UTC overflows.

**`DateTime<Utc>`-specific constructors**

- `from_timestamp_secs(secs)` -- from a Unix timestamp in whole seconds.
- `from_timestamp(secs, nsecs)` -- from a Unix timestamp plus a nanosecond remainder (which may exceed 1,000,000,000 for a leap second, only when `secs % 60 == 59`).
- `from_timestamp_millis` / `from_timestamp_micros` -- millisecond/microsecond equivalents.
- `from_timestamp_nanos` -- nanosecond equivalent; never fails (an `i64` of nanoseconds always fits).

**`DateTime<FixedOffset>`-specific parsing**

- `parse_from_rfc2822(s)` -- parses an RFC 2822 date-and-time string (e.g. `Tue, 1 Jul 2003 10:52:37 +0200`).
- `parse_from_rfc3339(s)` -- parses an RFC 3339/ISO 8601 date-and-time string.
- `parse_from_str(s, fmt)` / `parse_and_remainder(s, fmt)` -- parses using a user format string; unlike `NaiveDateTime::parse_from_str`, this *requires* a time zone in the input.

**Components and timestamps**

- `date_naive()` -- the local date component (panics if the offset pushes it outside `NaiveDate`'s range).
- `time()` -- the local time-of-day component.
- `timestamp()` -- non-leap seconds since the Unix epoch.
- `timestamp_millis()` / `timestamp_micros()` -- millisecond/microsecond equivalents.
- `timestamp_nanos()` (deprecated, panics beyond the ~584-year range an `i64` of nanoseconds can span) / `timestamp_nanos_opt()` -- nanosecond equivalent.
- `timestamp_subsec_millis()` / `timestamp_subsec_micros()` / `timestamp_subsec_nanos()` -- the sub-second remainder.
- `offset()` -- the offset from UTC in effect at this instant.
- `timezone()` -- reconstructs the associated `TimeZone` value from the stored offset.

**Conversions**

- `with_timezone(&tz)` -- changes the associated time zone, preserving the instant (not the wall-clock reading).
- `fixed_offset()` -- drops the generic time zone in favor of a plain `FixedOffset`, keeping the current offset.
- `to_utc()` -- converts to `DateTime<Utc>`, dropping the offset/time zone.
- `naive_utc()` -- a view of the underlying UTC `NaiveDateTime` (no offset applied).
- `naive_local()` -- a view with the offset applied (panics if that pushes the value outside `NaiveDateTime`'s range).

**Arithmetic**

- `checked_add_signed(TimeDelta)` / `checked_sub_signed(TimeDelta)` -- `None` on overflow.
- `checked_add_months(Months)` / `checked_sub_months(Months)` -- clamps to the last valid day of the resulting month; `None` if the resulting date is out of range or the local time at that date does not exist or is ambiguous (a DST transition).
- `checked_add_days(Days)` / `checked_sub_days(Days)` -- same ambiguity/range caveats as the month variants.
- `signed_duration_since(rhs)` -- signed `TimeDelta` since another `DateTime` (accepts by value or reference, even across different `Tz` types).
- `years_since(base)` -- whole years elapsed, anniversary-aware.
- `with_time(time)` -- replaces the time-of-day part, keeping the date; returns `MappedLocalTime::None` if that would push the value out of range.
- Operators: `Add`/`Sub<TimeDelta>`, `Add`/`Sub<core::time::Duration>`, `AddAssign`/`SubAssign` for both, `Add<FixedOffset>` (shifts the instant while the offset field is unchanged), `Add`/`Sub<Months>`, `Add`/`Sub<Days>`, `Sub<DateTime<Tz>> -> TimeDelta` (by value or by reference). All panic on overflow/ambiguity -- prefer the `checked_*` methods.

**`Datelike`/`Timelike` trait implementations** -- delegate to the local (offset-applied) reading; the `with_*` mutators return `None` (via `MappedLocalTime` collapsed to `Option`) when the resulting local time doesn't exist or is ambiguous, or falls outside the representable UTC range.

**Comparisons** -- `PartialEq`/`Eq` across any two `Tz`/`Tz2` (compares the underlying UTC instant, ignoring the time zone); `PartialOrd`/`Ord` likewise; `Hash`; `Copy` whenever `Tz::Offset: Copy` (`Tz` itself isn't stored).

**RFC-standard string output** (only when `Tz::Offset: Display`)

- `to_rfc2822()` -- e.g. `Tue, 1 Jul 2003 10:52:37 +0200`; panics if the year is outside 0..=9999 (RFC 2822's representable range).
- `to_rfc3339()` -- e.g. `1996-12-19T16:39:57-08:00`.
- `to_rfc3339_opts(secform, use_z)` -- like `to_rfc3339`, with explicit sub-second formatting (`SecondsFormat`, section 13) and an option to print `Z` instead of `+00:00` for UTC.

**Parsing and formatting** -- `format`, `format_with_items`, `format_localized`, `format_localized_with_items` (locale variants feature-gated), as for the naive types, plus the time zone's offset in the output.

**Interop and conversions**

- `From<std::time::SystemTime>` for `DateTime<Utc>`/`DateTime<Local>`; `From<DateTime<Tz>>` for `std::time::SystemTime`.
- `From<DateTime<Utc>>`/`From<DateTime<FixedOffset>>`/`From<DateTime<Local>>` for each of the other two time zones (six conversions total), all implemented via `with_timezone`.
- `Default` for `DateTime<Utc>`, `DateTime<Local>`, `DateTime<FixedOffset>` (Unix epoch, in each respective zone).
- `FromStr` for `DateTime<Utc>`/`DateTime<Local>` -- parses a relaxed RFC 3339 string (via the `DateTime<FixedOffset>` parser), then converts.
- `Debug`/`Display` (`Display` only when `Tz::Offset: Display`).

**`time_compute` extension living here** -- see Part 2, section 1 (`age`).

## 7. Time zones (`offset` module)

### 7.1 `TimeZone` trait

Computes the offset(s) between UTC and local time for a given zone; the primary way to construct `DateTime<Tz>` values.

- `type Offset: Offset` -- the associated offset type cached inside date/time values.
- `with_ymd_and_hms(year, month, day, hour, min, sec)` -- builds a `DateTime` from calendar components; `MappedLocalTime::None` on invalid input.
- `timestamp(secs, nsecs)` (deprecated, panics) / `timestamp_opt(secs, nsecs)` -- from a Unix timestamp plus nanosecond remainder.
- `timestamp_millis(millis)` (deprecated, panics) / `timestamp_millis_opt(millis)` -- millisecond equivalent.
- `timestamp_nanos(nanos)` -- nanosecond equivalent; never fails.
- `timestamp_micros(micros)` -- microsecond equivalent, `MappedLocalTime::None` on out-of-range input.
- `from_offset(&Self::Offset)` -- reconstructs the time zone from one of its offsets.
- `offset_from_local_date(&NaiveDate)` / `offset_from_local_datetime(&NaiveDateTime)` -- the offset(s) applicable to a given *local* date/datetime (may be ambiguous or nonexistent -- hence `MappedLocalTime`).
- `from_local_datetime(&NaiveDateTime)` -- converts a local `NaiveDateTime` to a timezone-aware `DateTime`, given the above.
- `offset_from_utc_date(&NaiveDate)` / `offset_from_utc_datetime(&NaiveDateTime)` -- the offset for a given *UTC* date/datetime; cannot fail (the UTC timeline has no gaps or folds).
- `from_utc_datetime(&NaiveDateTime)` -- converts a UTC `NaiveDateTime` to the local time.

### 7.2 `Offset` trait

- `fix()` -- the fixed offset from UTC that this value represents, as a `FixedOffset`.

### 7.3 `MappedLocalTime<T>` (alias: `LocalResult<T>`)

The result of mapping a local time to a concrete instant: unambiguous (`Single`), ambiguous (`Ambiguous(earliest, latest)`, e.g. during a DST fold), or nonexistent (`None`, e.g. during a DST gap, or on any other error).

- `single()` -- `Some` only for the unambiguous case.
- `earliest()` / `latest()` -- the earliest/latest candidate, for `Single` or `Ambiguous` (`None` for the `None` variant).
- `map(f)` -- transforms the contained value(s) with a function.
- `unwrap()` -- returns the single value or panics (best used with zones where the mapping can't practically fail, like `Utc`).

### 7.4 `Utc`

The UTC time zone; a zero-sized type also used as its own `Offset`.

- `now()` -- the current date and time in UTC (reads the system clock).
- `TimeZone`/`Offset` implementations are all trivial (UTC has no offset variation).
- `Debug` prints `Z`, `Display` prints `UTC`.

### 7.5 `FixedOffset`

A time zone with a fixed offset from UTC, from -23:59:59 to +23:59:59.

- `east(secs)` (deprecated, panics) / `east_opt(secs)` -- an offset for the Eastern Hemisphere (positive = ahead of UTC); `None`/panics if `secs` is not strictly within +/-86,400 seconds (24h).
- `west(secs)` (deprecated, panics) / `west_opt(secs)` -- same for the Western Hemisphere (sign flipped).
- `local_minus_utc()` -- seconds to add to UTC to get the local time.
- `utc_minus_local()` -- the inverse.
- `TimeZone`/`Offset` implementations are trivial (a fixed offset never varies by date).
- `Debug`/`Display` -- `+HH:MM`, `-HH:MM`, or `+HH:MM:SS` when the offset has a non-zero seconds component.
- `FromStr` -- parses strings like `+09:00`, `-0400`, `+02` (colon optional, any amount of surrounding whitespace).

### 7.6 `Local`

The system's local time zone; the one place in the crate depending on an external time zone database (`tzdb`/`tz-rs`, reading the IANA database).

- `now()` -- the current date and time in the local zone.
- `TimeZone` implementation resolves offsets via the system's configured zone, falling back to UTC if it cannot be determined (matching chrono's own documented fallback policy); correctly distinguishes a DST fold (ambiguous) from a gap (`None`) by checking whether the returned wall-clock time actually matches what was requested.

## 8. `Datelike` / `Timelike` traits

Read/write access to the components of any date-bearing (`Datelike`) or time-bearing (`Timelike`) type -- implemented by `NaiveDate`/`NaiveDateTime`/`DateTime<Tz>` (`Datelike`) and `NaiveTime`/`NaiveDateTime`/`DateTime<Tz>` (`Timelike`).

**`Datelike`** -- `year`, `year_ce()` (absolute year number from 1, paired with a CE/BCE flag; default method), `quarter()` (1-4, default method), `month`, `month0`, `day`, `day0`, `ordinal`, `ordinal0`, `weekday`, `iso_week`, the `with_*` mutators (abstract, implemented per type), `num_days_from_ce()` (default method, proleptic day count), `num_days_in_month()` (default method, length of the month this date falls in).

**`Timelike`** -- `hour`, `hour12()` (12-hour reading paired with an AM/PM flag; default method), `minute`, `second` (never 60, even for a leap second), `nanosecond` (`1,000,000,000..2,000,000,000` represents a leap second), the `with_*` mutators, `num_seconds_from_midnight()` (default method).

## 9. `Weekday` / `WeekdaySet`

### 9.1 `Weekday`

An enum, Monday through Sunday (ISO 8601 week convention).

- `succ()` / `pred()` -- the next/previous day, wrapping around the week.
- `num_days_from_monday()` / `num_days_from_sunday()` -- 0-indexed day number from the given start.
- `number_from_monday()` / `number_from_sunday()` -- 1-indexed equivalents.
- `days_since(other)` -- days since the previous (or same) occurrence of `other`, always in `0..7`.
- `Display` -- prints the 3-letter English abbreviation (e.g. `Mon`), matching the `%a` format specifier.
- `FromStr` -- accepts short or long English names, case-insensitively; returns `ParseWeekdayError` otherwise.
- `serde` support: serializes as the 3-letter name, deserializes short or long names.

### 9.2 `WeekdaySet`

A compact, `Copy` set of `Weekday`s packed into a single byte.

- `EMPTY` / `ALL` -- the empty set and the full 7-day set.
- `from_array([Weekday; C])` -- builds a set from an array (duplicates fine).
- `single(weekday)` -- a set containing exactly one day.
- `single_day()` -- the single day in the set, or `None` if empty or holding more than one.
- `insert(day)` / `remove(day)` -- mutate the set, reporting whether membership actually changed.
- `is_subset(other)`, `intersection(other)`, `union(other)`, `symmetric_difference(other)`, `difference(other)` -- standard set operations.
- `first()` / `last()` -- the earliest/latest day in the set (Monday-first ordering).
- `iter(start)` -- iterates the set's days starting at `start` and wrapping around the week (`WeekdaySetIter`, a `DoubleEndedIterator` + `ExactSizeIterator` + `FusedIterator`; not itself nameable outside the crate, matching chrono).
- `contains(day)`, `is_empty()`, `len()`.
- `Debug` -- prints the raw 7-bit mask (e.g. `WeekdaySet(0000001)`).
- `Display` -- prints a bracketed, comma-separated, Monday-first list (e.g. `[Mon, Fri, Sun]`).
- `FromIterator<Weekday>`.

## 10. `Month`

The month of the year as a standalone enum (`January`=0 through `December`=11), distinct from the plain `u32` returned by `Datelike::month()`.

- `succ()` / `pred()` -- next/previous month, wrapping across the year boundary.
- `number_from_month()` -- 1-indexed month number.
- `name()` -- full English name (e.g. `"January"`).
- `num_days(year)` -- the number of days in this month for a given year (accounts for leap years in February; `None` only if `year` is out of `NaiveDate`'s range).
- `TryFrom<u8>` -- builds a `Month` from a 1-12 number, `OutOfRange` error otherwise.
- `FromStr` -- accepts the short (3-letter) or full English name, case-insensitive; `ParseMonthError` otherwise.
- `serde` support: serializes as the full name, deserializes short or long names.

## 11. Durations (`duration` module)

### 11.1 `TimeDelta` (alias: `Duration`)

A signed span of time with nanosecond precision. Internal representation: `secs` carries the sign, `nanos` is always a non-negative remainder in `0..1_000_000_000`.

- `MIN` / `MAX` -- the representable range is restricted to +/- `i64::MAX` milliseconds (so `MIN` is not perfectly symmetric with `MAX` once sub-millisecond precision is considered), matching chrono exactly.
- `zero()` / `is_zero()`.
- `new(secs, nanos)` -- from whole seconds and a nanosecond remainder; `None` if out of range.
- `try_weeks`/`weeks`, `try_days`/`days`, `try_hours`/`hours`, `try_minutes`/`minutes`, `try_seconds`/`seconds` -- each pair is a fallible (`Option`) and panicking constructor from the named unit.
- `try_milliseconds`/`milliseconds` -- fallible/panicking; `milliseconds`/`try_milliseconds(-i64::MAX)` is exactly `MIN` and must succeed (a real overflow bug in the internal multiplication was found and fixed here via wrapping arithmetic).
- `microseconds`, `nanoseconds` -- infallible (the full `i64` range of micro-/nanoseconds always fits in the representable range).
- `num_weeks()`, `num_days()`, `num_hours()`, `num_minutes()`, `num_seconds()` -- whole-unit counts, truncated toward zero.
- `as_seconds_f64()` / `as_seconds_f32()` -- fractional-second representation.
- `num_milliseconds()` -- total milliseconds (never overflows, thanks to the constructors' bounds).
- `subsec_millis()` -- the fractional-second part in milliseconds.
- `num_microseconds()` / `num_nanoseconds()` -- total micro-/nanoseconds, `None` on overflow.
- `subsec_micros()` / `subsec_nanos()` -- fractional-second part, sign-matched to the whole duration (unlike the internal always-non-negative `nanos` field).
- `checked_add(rhs)` / `checked_sub(rhs)` -- `None` on overflow instead of panicking.
- `checked_mul(rhs: i32)` -- `None` if the result wouldn't fit in an `i64` number of seconds (checked against `i64`'s range, not `MIN`/`MAX`, matching chrono's own -- narrower -- documented behavior deliberately).
- `checked_div(rhs: i32)` -- `None` if `rhs` is zero.
- `abs()` -- absolute (non-negative) value.
- `from_std(core::time::Duration)` / `to_std()` -- conversion to/from the standard library's unsigned duration type, each returning `Result<_, OutOfRangeError>`.
- Operators: `Neg`, `Add`, `Sub`, `AddAssign`, `SubAssign`, `Mul<i32>`, `Div<i32>` (panics on division by zero), `Sum<&TimeDelta>`/`Sum<TimeDelta>` (for `.sum()` over an iterator).
- `Display` -- ISO 8601 duration format (e.g. `PT1.5S`), with a non-standard leading `-` for negative durations.
- `serde` support: serializes as a `(secs, nanos)` tuple.

### 11.2 `Days`

An increment expressed in whole days, used with `NaiveDate::checked_add_days`/`checked_sub_days` and the `Add`/`Sub<Days>` operators.

- `new(num_days: u64)`.

### 11.3 `Months`

An increment expressed in calendar months (distinct from `TimeDelta`, which has no notion of variable-length months), used with `checked_add_months`/`checked_sub_months`.

- `new(num_months: u32)`.
- `as_u32()`.

### 11.4 `OutOfRangeError`

Returned by `TimeDelta::from_std`/`to_std` when the source value doesn't fit the target type. Implements `std::error::Error` and `Display`.

## 12. Rounding (`round` module)

### 12.1 `SubsecRound` trait

Rounds or truncates a value's sub-second precision to a maximum number of fractional digits. Blanket-implemented for any `Timelike + Add<TimeDelta, Output = Self> + Sub<TimeDelta, Output = Self>` (i.e. `NaiveTime`, `NaiveDateTime`, `DateTime<Tz>`).

- `round_subsecs(digits)` -- rounds (halfway values round away from zero); unchanged at 9+ digits.
- `trunc_subsecs(digits)` -- truncates; unchanged at 9+ digits.

### 12.2 `DurationRound` trait

Rounds or truncates a date/time by an arbitrary `TimeDelta` span (e.g. the nearest 15 minutes). Implemented for `NaiveDateTime` and `DateTime<Tz>`. Fails (via the associated `Err` type) whenever the span or the value's nanosecond timestamp doesn't fit in an `i64`, or the span is zero or negative.

- `duration_round(duration)` -- rounds to the nearest multiple (halfway rounds up).
- `duration_trunc(duration)` -- truncates down to the previous multiple.
- `duration_round_up(duration)` -- rounds up to the next multiple (or stays put if already an exact multiple).

### 12.3 `RoundingError`

The error type for `DurationRound`. Variants: `DurationExceedsTimestamp` (kept for chrono parity; no longer actually produced), `DurationExceedsLimit` (the span doesn't fit in an `i64` of nanoseconds, or is zero/negative), `TimestampExceedsLimit` (the value's own nanosecond timestamp doesn't fit). Implements `Display`/`Debug`.

## 13. Formatting and parsing (`format` module)

The shared engine behind every type's `format`/`parse_from_str` methods, and the `strftime`-style format-string syntax.

### 13.1 The `strftime` specifier table

Format strings use a syntax closely resembling C's `strftime`, documented in full in the `strftime` module. Every specifier below works both for formatting and (except where noted) parsing.

Date specifiers: `%Y` full year (zero-padded to 4 digits, accepts a sign outside 0..=9999), `%C` year/100, `%y` year%100, `%q` quarter (1-4), `%m` month (01-12), `%b`/`%h` abbreviated month name, `%B` full month name, `%d` day of month (01-31), `%e` day of month space-padded, `%a` abbreviated weekday, `%A` full weekday, `%w` weekday (Sun=0), `%u` ISO weekday (Mon=1), `%U` week number (Sunday-first), `%W` week number (Monday-first), `%G`/`%g` ISO week-date year (full/mod 100), `%V` ISO week number, `%j` day of year (001-366), `%D`/`%x` `%m/%d/%y`, `%F` `%Y-%m-%d`, `%v` `%e-%b-%Y`.

Time specifiers: `%H` hour (00-23), `%k` hour space-padded, `%I` hour (01-12), `%l` hour (12h) space-padded, `%P`/`%p` am/pm (lower/upper case), `%M` minute, `%S` second (00-60, accounts for leap seconds), `%f` nanoseconds since the last whole second, `%.f`/`%.3f`/`%.6f`/`%.9f` decimal fraction of a second (auto/3/6/9 digits, with leading dot), `%3f`/`%6f`/`%9f` same without the leading dot, `%R` `%H:%M`, `%T`/`%X` `%H:%M:%S`, `%r` `%I:%M:%S %p`.

Time zone specifiers: `%Z` offset only (no time zone name/abbreviation support), `%z` `+HHMM`, `%:z` `+HH:MM`, `%::z` `+HH:MM:SS`, `%:::z` `+HH` (offset without minutes), `%#z` parsing-only, allows minutes to be missing or present.

Date & time specifiers: `%c` `%a %b %e %H:%M:%S %Y`, `%+` ISO 8601/RFC 3339, `%s` Unix timestamp.

Special specifiers: `%t` tab, `%n` newline, `%%` literal percent.

Padding override modifiers (numeric specifiers only): `%-?` no padding, `%_?` space padding, `%0?` zero padding.

`%x`/`%X`/`%c`/`%r` and month/weekday names are always English/POSIX by default; `unstable-locales` (section 13.6) makes them locale-aware.

### 13.2 `StrftimeItems`

A parsing iterator turning a format string into a sequence of `Item`s.

- `new(s)` -- standard constructor; yields `Item::Error` on an invalid specifier.
- `new_lenient(s)` -- like `new`, but yields `Item::Literal` instead of erroring.
- `new_with_locale(s, locale)` -- adjusts `%x`/`%X`/`%c`/`%r` to the given locale (feature-gated).
- `parse()` -- collects into a `Vec<Item>`, useful to avoid re-parsing the same format string repeatedly.
- `parse_to_owned()` -- same, with no borrowed references into the original format string.

### 13.3 Items, numeric/fixed specifiers, padding

- `Item<'a>` -- the common intermediate representation for both formatting and parsing: `Literal`/`OwnedLiteral`, `Space`/`OwnedSpace`, `Numeric(Numeric, Pad)`, `Fixed(Fixed)`, `Error`.
- `Numeric` -- non-exhaustive enum of numeric field kinds (`Year`, `YearDiv100`, `YearMod100`, `IsoYear` and its Div100/Mod100 variants, `Quarter`, `Month`, `Day`, `WeekFromSun`, `WeekFromMon`, `IsoWeek`, `NumDaysFromSun`, `WeekdayFromMon`, `Ordinal`, `Hour`, `Hour12`, `Minute`, `Second`, `Nanosecond`, `Timestamp`, plus an opaque `Internal` variant for crate-internal use).
- `Fixed` -- non-exhaustive enum of fixed-format field kinds (month/weekday names, AM/PM markers, the `Nanosecond`/`Nanosecond3`/`Nanosecond6`/`Nanosecond9` fractional-second variants, the timezone-name/offset variants, `RFC2822`, `RFC3339`, plus an opaque `Internal` variant).
- `Pad` -- `None`, `Zero`, `Space`.
- `OffsetFormat` / `OffsetPrecision` / `Colons` -- structured description of how to render a UTC offset (precision, colon style, whether to allow `Z` for zero offset, hour padding); used internally by the RFC 3339/2822 writers.

### 13.4 Parsing functions

- `parse(parsed: &mut Parsed, s, items)` -- parses `s` against a sequence of `Item`s into `Parsed`; errors if any input remains unconsumed (`Item::Space` at the end can absorb trailing whitespace). Greedy (prefers the longest valid match for names) and padding-agnostic (ignores the requested `Pad`, so any amount of leading whitespace/zeros is accepted), but obeys each item's fixed parsing width.
- `parse_and_remainder(parsed, s, items)` -- same, returning the unparsed remainder instead of erroring on it.

### 13.5 `Parsed`

An incrementally-built, consistency-checked collection of parsed date/time fields; used internally by every parsing function, and public so custom parsers can reuse the same resolution logic.

- `new()`.
- `set_year`, `set_year_div_100`, `set_year_mod_100`, `set_isoyear`, `set_isoyear_div_100`, `set_isoyear_mod_100`, `set_quarter`, `set_month`, `set_week_from_sun`, `set_week_from_mon`, `set_isoweek`, `set_weekday`, `set_ordinal`, `set_day`, `set_ampm`, `set_hour12`, `set_hour`, `set_minute`, `set_second`, `set_nanosecond`, `set_timestamp`, `set_offset` -- each range-checks the value and, if the field was already set, requires consistency with the new value (`Impossible` error on conflict).
- `to_naive_date()` -- resolves a `NaiveDate` from whichever consistent subset of fields is available (year+month+day, year+ordinal, year+week+weekday, or ISO week date).
- `to_naive_time()` -- resolves a `NaiveTime` from hour(+AM/PM)/minute(/second(/nanosecond)); handles a leap second (`second == 60`).
- `to_naive_datetime_with_offset(offset)` -- resolves a combined `NaiveDateTime`, either from date+time fields or from a single `timestamp` field (the given `offset` is assumed, not cross-checked against any parsed offset field).
- `to_fixed_offset()` -- resolves a `FixedOffset` from the parsed `offset` field.
- `to_datetime()` -- resolves a full `DateTime<FixedOffset>` from date/time/offset fields and/or a timestamp.
- `to_datetime_with_timezone(&tz)` -- same, additionally validating (and disambiguating, where possible) against a specific `TimeZone`.
- Accessors mirroring every `set_*` field (`year()`, `year_div_100()`, ..., `offset()`) -- each returns `Option<T>`, `Some` only if that field was set.

### 13.6 `DelayedFormat` / `SecondsFormat` / `Locale`

- `DelayedFormat<I>` -- a lazily-rendered formatting result (only actually formats when displayed via `Display`/`to_string`/`write_to`); constructed via `new`, `new_with_offset`, `new_with_locale` (feature-gated), `new_with_offset_and_locale` (feature-gated). `write_to(writer)` performs the formatting into any `core::fmt::Write`.
- `SecondsFormat` -- non-exhaustive enum controlling sub-second precision in `to_rfc3339_opts`: `Secs`, `Millis`, `Micros`, `Nanos`, `AutoSi` (auto-selects based on the non-zero digits present).
- `Locale` -- re-export of `pure_rust_locales::Locale` (only present with the `unstable-locales` feature); selects the locale used by `format_localized`/`new_with_locale` and friends. Without the feature, formatting/parsing is always English/POSIX-only.

### 13.7 Errors

- `ParseError` -- opaque wrapper; `.kind()` returns the `ParseErrorKind`. Implements `Display`/`std::error::Error`.
- `ParseErrorKind` -- non-exhaustive: `OutOfRange` (a field's value is out of its permitted range), `Impossible` (fields are mutually inconsistent), `NotEnough` (insufficient fields to build the requested value), `Invalid` (unparseable character sequence), `TooShort` (input ended prematurely), `TooLong` (trailing unparsed input), `BadFormat` (bad/unsupported format string).
- `ParseResult<T>` -- alias for `Result<T, ParseError>`.

## 14. `serde` support (crate feature `serde`)

Available at `time_compute::serde` (re-exported from `datetime::serde`) and `time_compute::naive::serde` (re-exported from `naive::datetime::serde`).

`time_compute::serde` (for `DateTime<Tz>`, which (de)serializes to/from an RFC 3339 string by default): `ts_seconds`, `ts_milliseconds`, `ts_microseconds`, `ts_nanoseconds`, and their `_option` variants -- each a module with `serialize`/`deserialize` functions intended for serde's `#[serde(with = "...")]` field attribute, (de)serializing `DateTime<Utc>` (or `Option<DateTime<Utc>>`) as a Unix timestamp in the named unit instead of a string.

`time_compute::naive::serde` -- the same eight modules (`ts_seconds[_option]` through `ts_nanoseconds[_option]`), but for `NaiveDateTime` instead of `DateTime<Utc>` (internally assumes UTC to compute the timestamp).

Every other public type with a natural string form (`NaiveDate`, `NaiveTime`, `Weekday`, `Month`, `TimeDelta`, `IsoWeek` has none, matching chrono) also implements `serde::Serialize`/`Deserialize` directly (not gated behind a submodule), each documented alongside that type above.

## 15. `rkyv` support (crate features `rkyv`, `rkyv-16`, `rkyv-32`, `rkyv-64`; `rkyv-validation` adds `Archive: CheckBytes`)

Available at `time_compute::rkyv`, re-exporting the `Archived*` counterpart of every type with an `rkyv` derive: `ArchivedDateTime`, `ArchivedTimeDelta` (also aliased `ArchivedDuration`), `ArchivedMonth`, `ArchivedIsoWeek`, `ArchivedNaiveDate`, `ArchivedNaiveDateTime`, `ArchivedNaiveTime`, `ArchivedFixedOffset`, `ArchivedLocal`, `ArchivedUtc`, `ArchivedWeekday`.

---

# Part 2 -- `time_compute` extensions (no chrono equivalent)

Every item below is explicitly marked in the source with a `# time_compute extension -- not part of chrono` doc-comment section. None of it is guaranteed to match any future `chrono` API (there is nothing in chrono to match); all of it is still held to the same quality bar as the core (tests, docs, `#[must_use]`, `const fn` wherever the algorithm allows).

## 1. Age calculation

- `NaiveDate::age(&self, on: Self) -> Option<u32>` -- age in whole years as of `on`, e.g. `date_of_birth.age(today)`. Exactly `on.years_since(*self)` with the argument order that reads naturally for this use case. `NaiveDate` never reads the system clock; the caller passes "today" explicitly.
- `DateTime::age(&self, on: Self) -> Option<u32>` -- same idea for a timezone-aware instant, e.g. `date_of_birth.age(Utc::now())`.

Both are inspired by WLanguage's `Age()` function and were added as a deliberate, explicitly-authorized addition on top of the frozen chrono-compatible surface.

## 2. Christian movable feasts

All computed directly in the proleptic Gregorian calendar, `const fn`, no floating point.

- `NaiveDate::easter(year)` -- western (Catholic/Protestant) Easter Sunday, via the "anonymous Gregorian algorithm" (Meeus/Jones/Butcher), implemented from scratch (chrono itself has no equivalent; its ecosystem relies on the separate `computus` crate).
- `NaiveDate::orthodox_easter(year)` -- Eastern Orthodox Easter: computed via the Julian-calendar Paschal-full-moon reckoning (Meeus's Julian algorithm), then converted to the Gregorian calendar via the standard closed-form Julian/Gregorian drift (`year/100 - year/400 - 2` days).
- `NaiveDate::mardi_gras(year)` -- Fat Tuesday/Shrove Tuesday, 47 days before Easter.
- `NaiveDate::ash_wednesday(year)` -- first day of Lent, 46 days before Easter.
- `NaiveDate::palm_sunday(year)` -- 7 days before Easter.
- `NaiveDate::ascension(year)` -- 39 days after Easter.
- `NaiveDate::pentecost(year)` -- 49 days after Easter.

## 3. Hebrew calendar

A lunisolar calendar computed via the mean lunar conjunction (*molad*) of Tishrei, the 19-year Metonic cycle, and the four traditional postponement rules ("Lo ADU Rosh", "Molad Zaken", "GaTaRaD", "BeTuTeKaFoT"). All-integer arithmetic (no floating point), anchored to one independently verified real date (1 Tishrei 5783 = Monday 26 September 2022) so no ancient epoch conversion is needed -- every other year is reached by pure day counting from that anchor.

- `NaiveDate::from_hebrew_ymd(year, month, day)` -- Hebrew date to Gregorian `NaiveDate` (civil month numbering: 1 = Tishrei).
- `NaiveDate::to_hebrew_ymd(&self)` -- the exact inverse, `(year, month, day)`.
- `NaiveDate::passover(year)` -- 15 Nisan (Pessah), for the Gregorian year given (uses Hebrew year `year + 3760`).
- `NaiveDate::rosh_hashanah(year)` -- 1 Tishrei, the Hebrew New Year (Hebrew year `year + 3761`, since it falls in autumn).
- `NaiveDate::yom_kippur(year)` -- 10 Tishrei, 9 days after Rosh Hashanah.
- `NaiveDate::sukkot(year)` -- 15 Tishrei, 14 days after Rosh Hashanah.
- `NaiveDate::hanukkah(year)` -- 25 Kislev, accounting for the variable length of Cheshvan/Kislev that year.
- `NaiveDate::purim(year)` -- 14 Adar (or 14 Adar II in a leap year), using the winter-appropriate Hebrew year `year + 3760`.
- `NaiveDate::shavuot(year)` -- 6 Sivan.

## 4. Hijri/Islamic calendar (tabular)

Implements the "tabular" (also "civil"/"Kuwaiti algorithm") Islamic calendar: a fixed arithmetical approximation (odd months 30 days, even months 29, except Dhu al-Hijjah which gains a day in leap years; 11 leap years per 30-year cycle at fixed cycle positions), the same scheme used by, among others, Microsoft Windows. Anchored to two independently verified real dates (1 Muharram 1446 AH = Sunday 7 July 2024; 1 Muharram 1447 AH = Thursday 26 June 2025) rather than the 7th-century epoch. Integer-only, `const fn`.

**Important limitation, inherent to any tabular calendar, not specific to this implementation:** real-world religious observance is based on actual moon sighting (or, in some countries, the Umm al-Qura astronomical calendar), which routinely differs from this tabular approximation by a day or two. Use these functions for consistent, reproducible conversions, not to determine an officially announced observance date.

- `NaiveDate::from_hijri_ymd(year, month, day)` -- Hijri date (`year` = AH year, `month` 1-12) to Gregorian `NaiveDate`.
- `NaiveDate::to_hijri_ymd(&self)` -- the exact inverse.
- `NaiveDate::hijri_new_year(hijri_year)` -- 1 Muharram of the given **Hijri** year (not Gregorian -- the Hijri calendar drifts through all Gregorian seasons over roughly 33 years, so there is no fixed Gregorian-to-Hijri year mapping the way there is for the Hebrew calendar).
- `NaiveDate::ramadan_start(hijri_year)` -- 1 Ramadan.
- `NaiveDate::eid_al_fitr(hijri_year)` -- 1 Shawwal, marking the end of Ramadan.
- `NaiveDate::eid_al_adha(hijri_year)` -- 10 Dhu al-Hijjah.

## 5. Japanese calendar and festivals

### 5.1 `JapaneseEra`

An enum covering the five **modern** eras ("one reign, one era name" system formalized in 1868): `Meiji`, `Taisho`, `Showa`, `Heisei`, `Reiwa`. Pre-Meiji era names (over 200, some lasting only months) are out of scope.

- `start_date()` -- the first day of the era (the dates commonly used in software such as ICU/Unicode CLDR for Gregorian conversion).
- `end_date_exclusive()` -- the first day of the *next* era, or `None` for the current, open-ended era (Reiwa today).
- `name()` -- romanized name (e.g. `"Reiwa"`).
- `Display` -- prints `name()`.

### 5.2 Era conversion

- `NaiveDate::japanese_era(&self)` -- the `(JapaneseEra, era_year)` containing this date; `None` for dates before 25 January 1868.
- `NaiveDate::from_japanese_era_ymd(era, era_year, month, day)` -- the exact inverse; `None` if the date falls outside that era's actual span.

### 5.3 Fixed-date festivals (no astronomical calculation needed)

- `NaiveDate::shogatsu(year)` -- Japanese New Year, 1 January.
- `NaiveDate::hana_matsuri(year)` -- the Buddha's birthday observance in Japan, 8 April (unlike most Buddhist traditions, which use a lunar-calendar date, Japan fixed this to the Gregorian calendar).
- `NaiveDate::tanabata(year)` -- the star festival, 7 July (the date used in most of Japan; a minority of regions instead observe a lunar-calendar-equivalent date roughly a month later, not covered here).
- `NaiveDate::obon_start(year)` -- start of the ancestor-veneration festival, 13 August (most of Japan; some regions such as Okinawa follow the traditional lunisolar date instead, not covered here).
- `NaiveDate::shichi_go_san(year)` -- the children's rite-of-passage festival, 15 November.

### 5.4 Solar-term festivals (real astronomical calculation)

The crate's only floating-point, non-`const fn` functions, delegating to the `astro` crate (Meeus' algorithms) for the Sun's true ecliptic longitude -- a deliberate, explicitly-authorized exception to the crate's near-zero-dependency design, since an equinox/solar-term crossing has no closed-form integer solution. Accurate to well within a day for any plausible year; supports years roughly `-32768..=32767` only (bounded by `astro`'s internal `i16` year representation, unlike the rest of the crate's much wider range).

- `NaiveDate::shunbun_no_hi(year)` -- Vernal Equinox Day (春分の日): the JST calendar day containing the moment the Sun's longitude crosses 0 degrees. Also the midpoint of the spring "Higan" Buddhist observance week.
- `NaiveDate::shuubun_no_hi(year)` -- Autumnal Equinox Day (秋分の日): longitude crossing 180 degrees.
- `NaiveDate::setsubun(year)` -- the day before Risshun (立春, solar-term start of spring, longitude crossing 315 degrees), marked by the bean-throwing ritual (mamemaki).

## 6. Chinese lunisolar calendar

Implements the standard modern Chinese calendar algorithm (in use, with minor refinements, since the 1645 calendar reform; formalized in China's national standard GB/T 33661-2017): a lunar month begins on the China-Standard-Time (UTC+8) day of a new moon; the month containing the December solstice is always month 11; a "suì" (year, one "month 11" to the next) has 12 or 13 lunar months, with exactly one leap ("intercalary") month -- the first one after month 11 containing no "zhongqi" (major solar term) -- when it has 13. Backed by real astronomical computation via the `astro` crate (same dependency and precision notes as the Japanese solar-term functions above), with a `thread_local!` memoization cache for the underlying year-structure computation (`sui_months`) to keep repeated queries fast.

- `NaiveDate::chinese_new_year(year)` -- New Year's Day (first day of month 1) of the Chinese year whose New Year falls in Gregorian year `year` (the common "civil year" convention most Chinese calendar software uses; the traditional 60-year sexagenary cycle name is not implemented).
- `NaiveDate::to_chinese_ymd(&self)` -- converts to `(year, month, is_leap, day)` in the Chinese calendar.
- `NaiveDate::from_chinese_ymd(year, month, is_leap, day)` -- the exact inverse; `None` if `month`/`is_leap` doesn't identify a real month of that Chinese year, or `day` exceeds that month's actual length (29 or 30).
- `NaiveDate::duanwu(year)` -- Dragon Boat Festival (端午節), month 5 day 5.
- `NaiveDate::zhongqiu(year)` -- Mid-Autumn Festival (中秋節), month 8 day 15.
- `NaiveDate::qingming(year)` -- Qingming (清明, tomb-sweeping day): unlike `duanwu`/`zhongqiu`, a *solar-term* festival (Sun's longitude crossing 15 degrees, roughly April 4-5), not a Chinese-calendar month/day.

## 7. Thai/Buddhist lunisolar calendar

Implements the traditional Thai ("Chulasakarat") lunisolar reckoning used to fix the four principal Buddhist observances, plus the start/end of the Buddhist Lent. Unlike the Chinese calendar or the Japanese solar-term functions, this is **not** based on real astronomical positions: it is a centuries-old, purely arithmetic mean-motion model (integer counters -- "horakhun", "kammacapon", "avoman", "tithi" -- tracked against a fixed epoch), so this is integer-only, no floating point and no `astro` dependency. Because it tracks mean rates rather than true positions, its dates can differ by a day or two from true astronomical new moons, and even from other published Thai calendars using slightly different historical corrections to the same underlying system.

- `NaiveDate::magha_bucha(year)` -- commemorates the spontaneous gathering of 1,250 of the Buddha's disciples: full moon of the 3rd lunar month (4th, in an intercalary-month year).
- `NaiveDate::visakha_bucha(year)` -- Vesak, the most important Buddhist holy day (the Buddha's birth, enlightenment and death, traditionally held to have all occurred on the same full-moon day): full moon of the 6th lunar month (7th, in an intercalary year).
- `NaiveDate::asalha_bucha(year)` -- commemorates the Buddha's first sermon: full moon of the 8th lunar month (the *second*, repeated, occurrence of month 8 in an intercalary year -- a different shift rule than Magha/Visakha Bucha).
- `NaiveDate::khao_phansa(year)` -- start of the three-lunar-month Buddhist Lent, the day immediately after Asalha Bucha.
- `NaiveDate::awk_phansa(year)` -- end of the Buddhist Lent: full moon of the 11th lunar month. Never itself shifted by an intercalary month (whichever intercalation happens that year has already occurred earlier, at month 8).

## 8. Matariki (Māori New Year)

- `NaiveDate::matariki(year)` -- the date of Matariki, New Zealand's midwinter observance marking the pre-dawn rising of the Matariki star cluster (the Pleiades). Unlike every other function in this crate, **not** computed from any formula: New Zealand's Matariki Advisory Committee sets the date each year (the Friday closest to a several-day lunar phase window that is not itself reducible to a single instant) and it is legislated as a public holiday. This method is a fixed lookup table transcribed from the Museum of New Zealand Te Papa Tongarewa's officially published dates (2022-2052); returns `None` for any year outside that range rather than guessing, since the committee has not published dates beyond 2052 as of this writing.
