# Time-Compute

`time_compute` is a from-scratch, near-zero-dependency Rust date/time library with a public API that mirrors [`chrono`](https://github.com/chronotope/chrono) one-to-one. It exists to answer a simple question: could the same well-defined, mathematically settled problem -- calendar arithmetic -- be rebuilt on a leaner, more precisely scoped, more transparently maintainable foundation, without breaking a single line of code that already depends on `chrono`?

If your project already uses `chrono`, migrating is normally a matter of changing `use chrono::...` to `use time_compute::...` and nothing else. If you're starting fresh, you get the same battle-tested API shape as `chrono`, plus a set of calendar and festival extensions `chrono` doesn't have at all.

## Installation

```toml
[dependencies]
time_compute = "1.0"
```

Or, with an optional feature enabled (see `docs/About_dependencies.md` for what each one does):

```toml
[dependencies]
time_compute = { version = "1.0", features = ["serde"] }
```

## Quick start

```rust
use time_compute::{NaiveDate, Datelike, Weekday};

let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
assert_eq!(date.weekday(), Weekday::Mon);

// A time_compute-only extension: Easter has no chrono equivalent.
let easter_2027 = NaiveDate::easter(2027).unwrap();
println!("Easter 2027: {easter_2027}");
```

See `docs/Use_Example.md` for many more, organized by theme, country, and religious tradition.

## Full API compatibility with chrono

The entire `chrono`-compatible surface of this crate -- `NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Tz>`, `Utc`/`Local`/`FixedOffset`, `TimeDelta`/`Duration`, `Days`, `Months`, `Weekday`, `Month`, the `Datelike`/`Timelike` traits, the full `strftime`-style formatting/parsing engine, RFC 2822/3339 support, rounding, and the optional `serde`/`rkyv`/`arbitrary`/`defmt`/`unstable-locales` features -- matches `chrono`'s own public API: same type names, same method names and signatures, same observable behavior. That surface is **frozen**: once an item matches `chrono`'s behavior, it is never modified again. See `docs/API_Reference.md` for the exhaustive, function-by-function listing.

Compatible does not mean copied. `time_compute` contains **no `chrono` source code** -- every algorithm was implemented from its own primary sources (the underlying mathematics, published standards, astronomical algorithms, independent reference implementations), not from reading `chrono`'s implementation. The two libraries arrive at the same answers because they both correctly solve the same well-defined problem, not because one copies the other. See `docs/About_time_compute.md` for the full reasoning behind that design choice.

## An independent, original work

`time_compute` is not a fork, a patch, a wrapper, or a derivative of `chrono` in any sense. There is no shared code between the two projects: no file, function body, or algorithm in this crate was copied, adapted, or transliterated from `chrono`'s source. Every type, every calendar computation, the entire formatting/parsing engine, and every extension in this crate was designed and written from scratch, from primary sources -- the underlying mathematics, published standards (ISO 8601, national calendar standards), astronomical algorithms, and independent reference implementations -- with no dependency on `chrono`'s codebase, past or present. `chrono` has never appeared anywhere in this crate's own dependency tree, not even during development (see `docs/About_testing.md`: the one tool that ever linked against real `chrono` used it purely as an external, black-box oracle to compare *output* against, the same way an independent test suite would, and that tool has since been removed).

The two projects are entirely separate, independently maintained works. The only thing they share on purpose is the public API's shape -- type names, method names, signatures, and observable behavior -- and that resemblance exists for exactly one reason: so that a project already built on `chrono` can move to `time_compute` by changing an import path, not by rewriting its code. Beyond that one deliberate design choice, `time_compute` is an original work in its own right: its own architecture, its own algorithms, its own tests, its own dependencies, and its own documentation.

## Beyond chrono: what time_compute adds

On top of the frozen core, `time_compute` adds a substantial set of calendar and festival functions with **no `chrono` equivalent at all**, each checked against its own external, named authority (published tables, astronomical references, independent reference implementations -- never the author's own judgment):

- **Age calculation** -- `NaiveDate::age`/`DateTime::age`, an anniversary-aware "years old as of a given date" helper.
- **Christian movable feasts** -- Easter (Western and Orthodox), Mardi Gras, Ash Wednesday, Palm Sunday, Ascension, Pentecost.
- **Hebrew calendar** -- full bidirectional Gregorian <-> Hebrew conversion, plus Passover, Rosh Hashanah, Yom Kippur, Sukkot, Hanukkah, Purim, Shavuot.
- **Hijri (Islamic) calendar** -- the tabular/civil calendar, full bidirectional conversion, plus the Islamic New Year, Ramadan, Eid al-Fitr, Eid al-Adha.
- **Japanese era system and festivals** -- the five modern eras (Meiji through Reiwa) with conversion in both directions, fixed-date festivals (Shogatsu, Hana Matsuri, Tanabata, Obon, Shichi-Go-San), and real astronomically computed solar-term days (Vernal/Autumnal Equinox Day, Setsubun).
- **Chinese lunisolar calendar** -- full bidirectional conversion, Chinese New Year, Dragon Boat Festival, Mid-Autumn Festival, Qingming, all backed by real new-moon and solar-term computation.
- **Thai/Buddhist lunisolar calendar** -- Magha Bucha, Visakha Bucha (Vesak), Asalha Bucha, and the start/end of Buddhist Lent, via the traditional Chulasakarat mean-motion reckoning.
- **Matariki** -- the Maori New Year, New Zealand's official public holiday.

See `docs/Use_Example.md` for runnable examples of every one of these, organized by religion/tradition, by country, and by theme.

## A near-zero, precisely scoped dependency footprint

`time_compute` defaults to zero dependencies, with a short list of exceptions, each individually named and justified rather than pulled in casually: `tzdb`/`tz-rs` (reading the real IANA time zone database for `Local`, entirely in Rust rather than delegating to OS-specific behavior) and `astro` (real astronomical computation -- Meeus' algorithms -- for the handful of functions, like the Japanese/Chinese solar-term calendars, that need the Sun's true position and have no integer closed form). Everything else -- the entire chrono-compatible core, and the great majority of the calendar extensions above -- has no dependency at all. Compare that to `chrono` itself, which unconditionally depends on `num-traits` for its own core regardless of which features are enabled. `docs/About_dependencies.md` documents the exact scope of every single dependency this crate carries, so the "minimal dependencies" claim can be checked line by line rather than taken on faith.

## Tested two different ways

Two independent kinds of testing back this crate's correctness, because they catch two different kinds of mistake:

1. **492 unit tests**, one `#[cfg(test)] mod tests` block per source file, each checked against external ground truth -- published festival tables, astronomical references, independent reference implementations, wide-range round-trip properties, and cross-date invariants. These are permanent and continue to guard every future change.
2. **A differential test harness, run three times during development**, that compared `time_compute`'s output against the real `chrono` crate's output, input by input, across tens of thousands of comparisons. It caught two real bugs in this crate (a truncating-division bug in `iso_year_week`, and a `serde`/`FromStr` separator mismatch in `NaiveDateTime`) and independently surfaced one real, already-acknowledged bug in `chrono` itself. That tool, and `chrono` as a dev-dependency, have since been removed from the project entirely -- `chrono` is not, and has never been, a dependency of the published crate, in any configuration. `docs/About_testing.md` documents the full methodology and the exact numbers from all three runs.

## Built with Claude, designed to stay maintainable with AI

This crate -- its code, its tests, and every document in `docs/` -- was generated entirely through a Cowork collaboration with Claude (Anthropic's Sonnet 5 model), under Fabrice's direction. That is not incidental to the project's design: calendar arithmetic is a rare case of a software domain that is genuinely *settled* -- the Gregorian calendar's rules, ISO 8601, and the astronomical algorithms behind a solar term are fixed mathematics, not a moving target of business requirements. That makes correctness objectively checkable (against a published table, a standard, an independent implementation) rather than a matter of undocumented tribal knowledge, which is exactly the property that makes a codebase easy for an AI -- or any newcomer -- to safely review and extend without first absorbing years of unwritten context. Combined with a near-zero dependency graph and a project convention of writing down *why* each decision was made, `time_compute` is deliberately shaped to remain maintainable, including by AI-assisted development, long after this initial Cowork session. See `docs/About_time_compute.md` for the full argument, including an honest, non-promotional take on how that claim compares to `chrono`'s own decade of battle-tested maturity.

## Documentation

- `docs/Architecture.md` -- how the code is organized, with diagrams.
- `docs/API_Reference.md` -- exhaustive, function-by-function reference (chrono-compatible core, then extensions).
- `docs/Use_Example.md` -- runnable usage examples, core and extensions, organized by religion/country/theme.
- `docs/About_dependencies.md` -- exactly why each dependency exists and what it touches.
- `docs/About_testing.md` -- the full testing methodology and numbers.
- `docs/About_time_compute.md` -- why this crate exists, and the case for its long-term maintainability.

## License

MIT.
