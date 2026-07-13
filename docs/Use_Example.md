# Usage Examples

This document is a hands-on companion to `docs/API_Reference.md`: instead of an exhaustive, example-free listing, it shows runnable code. Every snippet uses only public `time_compute` API and compiles as written (assuming the shown `use` imports and the crate's default features, except where a snippet is explicitly marked as needing a Cargo feature).

The document has two parts:

- **Part 1** covers ordinary, day-to-day usage of the chrono-compatible core: building dates and times, formatting, arithmetic, time zones.
- **Part 2** is entirely about the `time_compute`-only extensions -- the calendars and festivals with no chrono equivalent. It is organized first by religion/cultural tradition, then by country (mixing several traditions into a realistic public-holiday list), then a quick theme-based index for looking a function up by what kind of thing it computes.

---

# Part 1 -- Basic usage

## 1. Creating dates and times

```rust
use time_compute::{NaiveDate, NaiveTime, NaiveDateTime};

let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
let time = NaiveTime::from_hms_opt(9, 30, 0).unwrap();

let dt: NaiveDateTime = date.and_time(time);

// A shortcut for the same result:
let dt2 = NaiveDate::from_ymd_opt(2026, 7, 13)
    .unwrap()
    .and_hms_opt(9, 30, 0)
    .unwrap();

assert_eq!(dt, dt2);
```

Every constructor that can fail (an invalid day, a nonexistent February 29, ...) returns `Option`, following the `_opt` naming convention throughout the crate; there is no panicking form to reach for by mistake.

## 2. The current date and time

```rust
use time_compute::{Utc, Local};

let now_utc = Utc::now();
let now_local = Local::now();

println!("UTC:   {now_utc}");
println!("Local: {now_local}");
```

`Local::now()` is the one place in the crate that consults the operating system's IANA time zone database (via the `tzdb`/`tz-rs` dependencies).

## 3. Time zones and `DateTime<Tz>`

```rust
use time_compute::{TimeZone, Utc, FixedOffset};

// `Utc` and `FixedOffset` implement `TimeZone`; `with_ymd_and_hms` is a
// trait method, called on a value of the zone (Utc is zero-sized).
let meeting = Utc.with_ymd_and_hms(2026, 7, 13, 14, 0, 0).unwrap();

let paris_summer_time = FixedOffset::east_opt(2 * 3600).unwrap(); // UTC+2 (CEST)
let meeting_in_paris = meeting.with_timezone(&paris_summer_time);

println!("{meeting}");         // 2026-07-13 14:00:00 UTC
println!("{meeting_in_paris}"); // 2026-07-13 16:00:00 +02:00
```

## 4. Formatting and parsing

```rust
use time_compute::NaiveDate;

let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();

println!("{}", date.format("%A %d %B %Y")); // "Monday 13 July 2026"
println!("{}", date.format("%Y-%m-%d"));    // "2026-07-13"

let parsed = NaiveDate::parse_from_str("13/07/2026", "%d/%m/%Y").unwrap();
assert_eq!(parsed, date);
```

```rust
use time_compute::DateTime;

// RFC 3339/2822 have dedicated parsers, since they need a time zone.
let dt = DateTime::parse_from_rfc3339("2026-07-13T14:00:00+02:00").unwrap();
println!("{}", dt.to_rfc2822());
```

## 5. Arithmetic: `TimeDelta`, `Days`, `Months`

```rust
use time_compute::{NaiveDate, TimeDelta, Days, Months};

let date = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();

let next_week = date + Days::new(7);        // 2026-02-07
let next_month = date + Months::new(1);     // 2026-02-28 (clamped: no Feb 31)
let ten_days_later = date + TimeDelta::days(10);

let christmas = NaiveDate::from_ymd_opt(2026, 12, 25).unwrap();
let gap = christmas.signed_duration_since(date);
println!("{} days between the two dates", gap.num_days());
```

`Months` respects calendar-month length (clamping the day of month when needed); `TimeDelta`/`Days` are plain fixed-length spans. Use `checked_add_*`/`checked_sub_*` instead of the operators when overflow must be handled rather than panic.

## 6. Reading date/time components

```rust
use time_compute::{NaiveDate, Datelike, Weekday};

let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();

assert_eq!(date.year(), 2026);
assert_eq!(date.month(), 7);
assert_eq!(date.weekday(), Weekday::Mon);
assert_eq!(date.iso_week().week(), 29);
```

## 7. Weeks and iteration

```rust
use time_compute::{NaiveDate, Weekday};

let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();

let week = date.week(Weekday::Mon);
println!("This week runs from {} to {}", week.first_day(), week.last_day());

for day in date.iter_days().take(5) {
    println!("{day}");
}
```

## 8. Rounding

```rust
use time_compute::{NaiveDate, TimeDelta, DurationRound, SubsecRound};

// Rounding to an arbitrary span (here, the nearest 15 minutes)
let dt = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap()
    .and_hms_opt(9, 41, 27).unwrap();
let rounded = dt.duration_round(TimeDelta::minutes(15)).unwrap();
assert_eq!(rounded, NaiveDate::from_ymd_opt(2026, 7, 13).unwrap().and_hms_opt(9, 45, 0).unwrap());

// Rounding/truncating sub-second precision
let precise = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap()
    .and_hms_milli_opt(9, 41, 27, 456).unwrap();
let truncated = precise.trunc_subsecs(0);
assert_eq!(truncated, NaiveDate::from_ymd_opt(2026, 7, 13).unwrap().and_hms_opt(9, 41, 27).unwrap());
```

## 9. Optional `serde` support (feature `serde`)

```rust
use time_compute::NaiveDate;

let date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
let json = serde_json::to_string(&date).unwrap();      // "\"2026-07-13\""
let back: NaiveDate = serde_json::from_str(&json).unwrap();
assert_eq!(back, date);
```

---

# Part 2 -- `time_compute`-only extensions

Everything below has no chrono equivalent (see `docs/API_Reference.md`, Part 2, and `docs/About_dependencies.md` for which of these draw on the `astro` dependency for real astronomical computation, versus which are pure integer arithmetic).

## A. Age calculation

```rust
use time_compute::{NaiveDate, TimeZone, Utc};

let date_of_birth = NaiveDate::from_ymd_opt(1990, 6, 15).unwrap();
let today = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
println!("{} years old", date_of_birth.age(today).unwrap());

let birth_instant = Utc.with_ymd_and_hms(1990, 6, 15, 8, 30, 0).unwrap();
println!("{:?} years old, as an instant", birth_instant.age(Utc::now()));
```

## B. By religion / cultural tradition

### B.1 Christianity

```rust
use time_compute::NaiveDate;

let easter = NaiveDate::easter(2027).unwrap();
let orthodox_easter = NaiveDate::orthodox_easter(2027).unwrap();
let mardi_gras = NaiveDate::mardi_gras(2027).unwrap();
let ash_wednesday = NaiveDate::ash_wednesday(2027).unwrap();
let palm_sunday = NaiveDate::palm_sunday(2027).unwrap();
let ascension = NaiveDate::ascension(2027).unwrap();
let pentecost = NaiveDate::pentecost(2027).unwrap();

println!("Easter (Western): {easter}");
println!("Easter (Orthodox): {orthodox_easter}");
```

### B.2 Judaism

```rust
use time_compute::NaiveDate;

// Direct Hebrew-calendar <-> Gregorian conversion (civil month numbering: 1 = Tishrei)
let rosh_hashanah_5787 = NaiveDate::from_hebrew_ymd(5787, 1, 1).unwrap();
let (hebrew_year, hebrew_month, hebrew_day) = rosh_hashanah_5787.to_hebrew_ymd();

// Gregorian-year-indexed festival helpers
let passover = NaiveDate::passover(2026).unwrap();
let rosh_hashanah = NaiveDate::rosh_hashanah(2026).unwrap();
let yom_kippur = NaiveDate::yom_kippur(2026).unwrap();
let sukkot = NaiveDate::sukkot(2026).unwrap();
let hanukkah = NaiveDate::hanukkah(2026).unwrap();
let purim = NaiveDate::purim(2027).unwrap();
let shavuot = NaiveDate::shavuot(2026).unwrap();
```

### B.3 Islam

```rust
use time_compute::NaiveDate;

let new_year = NaiveDate::hijri_new_year(1448).unwrap();
let ramadan_start = NaiveDate::ramadan_start(1448).unwrap();
let eid_al_fitr = NaiveDate::eid_al_fitr(1448).unwrap();
let eid_al_adha = NaiveDate::eid_al_adha(1448).unwrap();

// Hijri <-> Gregorian conversion (tabular/civil calendar)
let some_day = NaiveDate::from_hijri_ymd(1448, 9, 1).unwrap();
let (hijri_year, hijri_month, hijri_day) = some_day.to_hijri_ymd();
```

Real-world religious observance is based on moon sighting (or, in some countries, the Umm al-Qura astronomical calendar) and routinely differs from this tabular approximation by a day or two -- see `docs/API_Reference.md`, Part 2 section 4.

### B.4 Buddhism

```rust
use time_compute::NaiveDate;

// Thai/Buddhist lunisolar reckoning
let magha_bucha = NaiveDate::magha_bucha(2026).unwrap();
let visakha_bucha = NaiveDate::visakha_bucha(2026).unwrap();   // Vesak
let asalha_bucha = NaiveDate::asalha_bucha(2026).unwrap();
let khao_phansa = NaiveDate::khao_phansa(2026).unwrap();       // start of Buddhist Lent
let awk_phansa = NaiveDate::awk_phansa(2026).unwrap();         // end of Buddhist Lent

// Japan fixes the Buddha's birthday to a Gregorian-calendar date instead
let hana_matsuri = NaiveDate::hana_matsuri(2026).unwrap();
```

### B.5 Chinese folk religion / lunisolar calendar

```rust
use time_compute::NaiveDate;

let chinese_new_year = NaiveDate::chinese_new_year(2026).unwrap();
let duanwu = NaiveDate::duanwu(2026).unwrap();     // Dragon Boat Festival
let zhongqiu = NaiveDate::zhongqiu(2026).unwrap(); // Mid-Autumn Festival
let qingming = NaiveDate::qingming(2026).unwrap(); // Tomb-Sweeping Day (solar term, not lunar)

// Chinese calendar <-> Gregorian conversion
let (c_year, c_month, is_leap, c_day) = chinese_new_year.to_chinese_ymd().unwrap();
let round_trip = NaiveDate::from_chinese_ymd(c_year, c_month, is_leap, c_day).unwrap();
assert_eq!(round_trip, chinese_new_year);
```

### B.6 Shinto / Japanese civil and folk festivals

```rust
use time_compute::{NaiveDate, JapaneseEra};

// Fixed-date festivals (no calendar conversion involved)
let shogatsu = NaiveDate::shogatsu(2026).unwrap();               // New Year
let tanabata = NaiveDate::tanabata(2026).unwrap();               // star festival
let obon_start = NaiveDate::obon_start(2026).unwrap();
let shichi_go_san = NaiveDate::shichi_go_san(2026).unwrap();     // children's rite of passage

// Solar-term festivals (real astronomical computation, via the `astro` dependency)
let shunbun_no_hi = NaiveDate::shunbun_no_hi(2026).unwrap();     // Vernal Equinox Day
let shuubun_no_hi = NaiveDate::shuubun_no_hi(2026).unwrap();     // Autumnal Equinox Day
let setsubun = NaiveDate::setsubun(2026).unwrap();

// The era system
let today = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
let (era, era_year) = today.japanese_era().unwrap();
println!("{era} {era_year}"); // "Reiwa 8"

let heisei_start = NaiveDate::from_japanese_era_ymd(JapaneseEra::Heisei, 1, 1, 8).unwrap();
```

### B.7 Māori tradition

```rust
use time_compute::NaiveDate;

// A fixed lookup table (2022-2052), not a formula -- see docs/API_Reference.md
let matariki_2026 = NaiveDate::matariki(2026).unwrap();
println!("Matariki 2026: {matariki_2026}");

assert_eq!(NaiveDate::matariki(2021), None); // outside the published range
```

## C. By country -- sample public-holiday calendars

Real holiday calendars usually mix a civil calendar with one or more religious traditions. These functions combine cleanly for that purpose.

### C.1 France

```rust
use time_compute::NaiveDate;

fn french_public_holidays(year: i32) -> Vec<(&'static str, NaiveDate)> {
    let easter = NaiveDate::easter(year).unwrap();
    let pentecost = NaiveDate::pentecost(year).unwrap();
    vec![
        ("Jour de l'an", NaiveDate::from_ymd_opt(year, 1, 1).unwrap()),
        ("Lundi de Paques", easter.succ_opt().unwrap()),
        ("Fete du Travail", NaiveDate::from_ymd_opt(year, 5, 1).unwrap()),
        ("Victoire 1945", NaiveDate::from_ymd_opt(year, 5, 8).unwrap()),
        ("Ascension", NaiveDate::ascension(year).unwrap()),
        ("Lundi de Pentecote", pentecost.succ_opt().unwrap()),
        ("Fete nationale", NaiveDate::from_ymd_opt(year, 7, 14).unwrap()),
        ("Assomption", NaiveDate::from_ymd_opt(year, 8, 15).unwrap()),
        ("Toussaint", NaiveDate::from_ymd_opt(year, 11, 1).unwrap()),
        ("Armistice", NaiveDate::from_ymd_opt(year, 11, 11).unwrap()),
        ("Noel", NaiveDate::from_ymd_opt(year, 12, 25).unwrap()),
    ]
}
```

### C.2 Israel

```rust
use time_compute::NaiveDate;

fn israeli_public_holidays(year: i32) -> Vec<(&'static str, NaiveDate)> {
    vec![
        ("Rosh Hashanah", NaiveDate::rosh_hashanah(year).unwrap()),
        ("Yom Kippur", NaiveDate::yom_kippur(year).unwrap()),
        ("Sukkot", NaiveDate::sukkot(year).unwrap()),
        ("Hanukkah", NaiveDate::hanukkah(year).unwrap()),
        ("Purim", NaiveDate::purim(year).unwrap()),
        ("Passover", NaiveDate::passover(year).unwrap()),
        ("Shavuot", NaiveDate::shavuot(year).unwrap()),
    ]
}
```

### C.3 Muslim-majority countries (tabular approximation)

```rust
use time_compute::NaiveDate;

fn tabular_islamic_holidays(hijri_year: i32) -> Vec<(&'static str, NaiveDate)> {
    vec![
        ("Islamic New Year", NaiveDate::hijri_new_year(hijri_year).unwrap()),
        ("Start of Ramadan", NaiveDate::ramadan_start(hijri_year).unwrap()),
        ("Eid al-Fitr", NaiveDate::eid_al_fitr(hijri_year).unwrap()),
        ("Eid al-Adha", NaiveDate::eid_al_adha(hijri_year).unwrap()),
    ]
}
// Note (see section B.3 above): treat these as reproducible reference dates,
// not as the officially announced observance date.
```

### C.4 Thailand

```rust
use time_compute::NaiveDate;

fn thai_buddhist_holidays(year: i32) -> Vec<(&'static str, NaiveDate)> {
    vec![
        ("Makha Bucha", NaiveDate::magha_bucha(year).unwrap()),
        ("Visakha Bucha", NaiveDate::visakha_bucha(year).unwrap()),
        ("Asalha Bucha", NaiveDate::asalha_bucha(year).unwrap()),
        ("Khao Phansa", NaiveDate::khao_phansa(year).unwrap()),
        ("Awk Phansa", NaiveDate::awk_phansa(year).unwrap()),
    ]
}
```

### C.5 China

```rust
use time_compute::NaiveDate;

fn chinese_festivals(year: i32) -> Vec<(&'static str, NaiveDate)> {
    vec![
        ("Chinese New Year", NaiveDate::chinese_new_year(year).unwrap()),
        ("Qingming", NaiveDate::qingming(year).unwrap()),
        ("Duanwu (Dragon Boat)", NaiveDate::duanwu(year).unwrap()),
        ("Zhongqiu (Mid-Autumn)", NaiveDate::zhongqiu(year).unwrap()),
    ]
}
```

### C.6 Japan

```rust
use time_compute::NaiveDate;

fn japanese_festivals(year: i32) -> Vec<(&'static str, NaiveDate)> {
    vec![
        ("Shogatsu (New Year)", NaiveDate::shogatsu(year).unwrap()),
        ("Shunbun no Hi (Vernal Equinox)", NaiveDate::shunbun_no_hi(year).unwrap()),
        ("Hana Matsuri", NaiveDate::hana_matsuri(year).unwrap()),
        ("Tanabata", NaiveDate::tanabata(year).unwrap()),
        ("Obon", NaiveDate::obon_start(year).unwrap()),
        ("Shuubun no Hi (Autumnal Equinox)", NaiveDate::shuubun_no_hi(year).unwrap()),
        ("Shichi-Go-San", NaiveDate::shichi_go_san(year).unwrap()),
    ]
}
```

### C.7 New Zealand

```rust
use time_compute::NaiveDate;

fn nz_matariki_holiday(year: i32) -> Option<NaiveDate> {
    NaiveDate::matariki(year) // None outside the published 2022-2052 range
}
```

## D. By theme -- quick index

| Theme | Functions | Notes |
|---|---|---|
| Movable feast, computed by a fixed algorithm | `easter`, `orthodox_easter`, `mardi_gras`, `ash_wednesday`, `palm_sunday`, `ascension`, `pentecost`, `passover`, `rosh_hashanah`, `yom_kippur`, `sukkot`, `hanukkah`, `purim`, `shavuot`, `hijri_new_year`, `ramadan_start`, `eid_al_fitr`, `eid_al_adha` | All `const fn`, integer-only arithmetic, no dependency |
| Fixed Gregorian-calendar date | `shogatsu`, `hana_matsuri`, `tanabata`, `obon_start`, `shichi_go_san` | No calendar conversion involved |
| Real astronomical computation (non-`const fn`, floating point) | `shunbun_no_hi`, `shuubun_no_hi`, `setsubun`, `chinese_new_year`, `to_chinese_ymd`, `from_chinese_ymd`, `duanwu`, `zhongqiu`, `qingming` | Uses the `astro` dependency (see `docs/About_dependencies.md`) |
| Calendar-system conversion | `from_hebrew_ymd`/`to_hebrew_ymd`, `from_hijri_ymd`/`to_hijri_ymd`, `from_chinese_ymd`/`to_chinese_ymd`, `japanese_era`/`from_japanese_era_ymd` | Each pair round-trips exactly |
| Fixed lookup table, not a formula | `matariki` | Covers 2022-2052 only, `None` outside that range |
| Age / elapsed time | `NaiveDate::age`, `DateTime::age` | Built on `years_since`; caller always supplies "today" explicitly, since neither type reads the system clock itself |
