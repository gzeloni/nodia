# Date & Time Builtins

Nodia now has first-class `date`, `datetime`, and `duration` values.

The implementation is intentionally strict and deterministic:

* `date` is a calendar date with no time-of-day.
* `datetime` is an instant plus a fixed UTC offset.
* `duration` is a precise span stored in nanoseconds.

Nodia currently supports **UTC and fixed offsets** such as `Z`, `+02:00`,
or `-0330`. It does **not** ship an IANA time zone database in the standard
library, so names like `"America/Sao_Paulo"` are not part of the language yet.

## Constructors

| Builtin | Behavior |
| ------- | -------- |
| `date(year, month, day)` | builds a `date` |
| `date({year, month, day})` | map-based `date` constructor |
| `datetime(y, m, d, h, min, s)` | builds a UTC `datetime` |
| `datetime(y, m, d, h, min, s, offset_or_options)` | accepts offset string/int or options map |
| `datetime({year, month, day, hour, minute, second, ...})` | map-based `datetime` constructor |
| `duration({...})` | builds a `duration` from component fields |

Accepted `datetime(...)` options:

* `offset`: string or integer minutes, for example `"Z"`, `"+05:30"`, `-180`
* `nanosecond`: integer `0..999999999`

Accepted `duration(...)` fields:

* `weeks`
* `days`
* `hours`
* `minutes`
* `seconds` (`int` or `float`)
* `milliseconds`
* `microseconds`
* `nanoseconds`

```bash
./target/release/nodia eval '
val d = date(2026, 5, 27)
val dt = datetime({
  year: 2026,
  month: 5,
  day: 27,
  hour: 14,
  minute: 30,
  second: 5,
  offset: "+05:30",
})
val span = duration({days: 2, hours: 3, seconds: 0.5})

emit d
emit dt
emit span
'
```

```text
2026-05-27
2026-05-27T14:30:05+05:30
P2DT3H0.5S
```

## Parse And Format

| Builtin | Behavior |
| ------- | -------- |
| `parse_date(text)` | parses `YYYY-MM-DD` |
| `parse_datetime(text)` | parses ISO/RFC3339-style datetime text |
| `parse_duration(text)` | parses ISO 8601 duration text |
| `isoformat(value)` | emits canonical ISO text for `date`, `datetime`, `duration` |
| `strftime(value, pattern)` | formats `date` or `datetime` with a standard directive subset |

`parse_datetime(...)` accepts:

* `T`, `t`, or a space between date and time
* optional fractional seconds
* optional offsets like `Z`, `+02`, `+0230`, `+02:30`
* no offset means UTC

`parse_duration(...)` accepts ISO 8601 day/time durations such as `P3D`,
`PT4H30M`, `P2DT1.5S`, `-PT15M`. Calendar years/months are intentionally
rejected because they are not fixed-length durations.

Supported `strftime(...)` directives:

| Directive | Meaning |
| --------- | ------- |
| `%Y` | year |
| `%m` | month number |
| `%d` | day of month |
| `%H` | hour |
| `%M` | minute |
| `%S` | second |
| `%f` | microseconds |
| `%N` | nanoseconds |
| `%a`, `%A` | weekday short / long name |
| `%b`, `%B` | month short / long name |
| `%j` | ordinal day of year |
| `%u` | ISO weekday (`1..7`) |
| `%V` | ISO week number |
| `%F` | `%Y-%m-%d` |
| `%T` | `%H:%M:%S` |
| `%z` | offset as `+HHMM` |
| `%:z` | offset as `+HH:MM` |
| `%Z` | `UTC` or `UTC+HH:MM` |
| `%%` | literal `%` |

```bash
./target/release/nodia eval '
val dt = parse_datetime("2026-05-27T14:30:05.12+05:30")
emit isoformat(dt)
emit strftime(dt, "%F %T %:z")
'
```

```text
2026-05-27T14:30:05.12+05:30
2026-05-27 14:30:05 +05:30
```

## Clock And Epoch

| Builtin | Behavior |
| ------- | -------- |
| `now()` | current UTC `datetime` |
| `now(offset)` | current `datetime` rendered in a fixed offset |
| `today()` | current UTC `date` |
| `today(offset)` | current `date` in a fixed offset |
| `from_unix(seconds)` | unix seconds to `datetime` |
| `from_unix(seconds, offset)` | same instant rendered in another fixed offset |
| `from_unix_ms(milliseconds)` | unix milliseconds to `datetime` |
| `unix_seconds(datetime)` | unix timestamp as `int` or `float` |
| `unix_ms(datetime)` | unix milliseconds as `int` or `float` |

`from_unix(...)` accepts integer or floating-point seconds. `from_unix_ms(...)`
accepts integer or floating-point milliseconds.

## Accessors

| Builtin | Behavior |
| ------- | -------- |
| `year(x)` | year from `date` or `datetime` |
| `month(x)` | month from `date` or `datetime` |
| `day(x)` | day from `date` or `datetime` |
| `hour(x)` | hour from `datetime` |
| `minute(x)` | minute from `datetime` |
| `second(x)` | second from `datetime` |
| `nanosecond(x)` | nanosecond from `datetime` |
| `weekday(x)` | ISO weekday (`1 = Monday`) |
| `weekday_name(x)` | full weekday name |
| `month_name(x)` | full month name |
| `ordinal_day(x)` | day-of-year (`1..366`) |
| `iso_week(x)` | map `{year, week}` |
| `offset_minutes(datetime)` | fixed UTC offset in minutes |
| `days_in_month(date_or_datetime)` | days in the current month |
| `days_in_month(year, month)` | days in a specific month |
| `is_leap_year(int_or_date_or_datetime)` | leap-year predicate |
| `date_only(x)` | extracts a `date` from `date` or `datetime` |

## Arithmetic And Comparison

| Builtin | Behavior |
| ------- | -------- |
| `with_offset(datetime, offset)` | same instant, different rendered offset |
| `add_days(x, n)` | shifts `date` or `datetime` by calendar days |
| `add_months(x, n)` | calendar month arithmetic with end-of-month clamping |
| `add_years(x, n)` | calendar year arithmetic with leap-day clamping |
| `add_duration(datetime, duration)` | instant arithmetic |
| `add_duration(duration, duration)` | duration arithmetic |
| `diff_days(a, b)` | difference in whole calendar days |
| `diff_seconds(a, b)` | difference in seconds |
| `diff_duration(a, b)` | difference as `duration` |
| `start_of_day(x)` | midnight `datetime` |
| `end_of_day(x)` | `23:59:59.999999999` `datetime` |

Ordering works with `<`, `<=`, `>`, `>=` for matching temporal kinds:

* `date` compares by calendar day
* `datetime` compares by instant, not by textual representation
* `duration` compares by exact length

`datetime` equality also compares by instant:

```bash
./target/release/nodia eval '
emit parse_datetime("2024-01-01T00:00:00+02:00") == parse_datetime("2023-12-31T22:00:00Z")
'
```

```text
true
```

## JSON

`json.write(...)` encodes:

* `date` as an ISO date string
* `datetime` as an ISO datetime string
* `duration` as an ISO duration string

```bash
./target/release/nodia eval '
use json

emit json.write({
  when: parse_datetime("2024-02-29T12:00:00Z"),
  due: date(2024, 3, 5),
  retry_in: duration({minutes: 15}),
})
'
```

```text
{"due":"2024-03-05","retry_in":"PT15M","when":"2024-02-29T12:00:00Z"}
```
