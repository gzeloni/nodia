# Date & Time Builtins

Nodia now has first-class `date`, `datetime`, and `duration` values.
Import this namespace with `use datetime`.

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
| `datetime.date(year, month, day)` | builds a `date` |
| `datetime.date({year, month, day})` | map-based `date` constructor |
| `datetime.datetime(y, m, d, h, min, s)` | builds a UTC `datetime` |
| `datetime.datetime(y, m, d, h, min, s, offset_or_options)` | accepts offset string/int or options map |
| `datetime.datetime({year, month, day, hour, minute, second, ...})` | map-based `datetime` constructor |
| `datetime.duration({...})` | builds a `duration` from component fields |

Accepted `datetime.datetime(...)` options:

* `offset`: string or integer minutes, for example `"Z"`, `"+05:30"`, `-180`
* `nanosecond`: integer `0..999999999`

Accepted `datetime.duration(...)` fields:

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
use datetime

val d = datetime.date(2026, 5, 27)
val dt = datetime.datetime({
  year: 2026,
  month: 5,
  day: 27,
  hour: 14,
  minute: 30,
  second: 5,
  offset: "+05:30",
})
val span = datetime.duration({days: 2, hours: 3, seconds: 0.5})

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
| `datetime.parse(text, datetime.as_date)` | parses `YYYY-MM-DD` and returns `result` |
| `datetime.parse(text, datetime.as_datetime)` | parses ISO/RFC3339-style datetime text and returns `result` |
| `datetime.parse(text, datetime.as_duration)` | parses ISO 8601 duration text and returns `result` |
| `datetime.isoformat(value)` | emits canonical ISO text for `date`, `datetime`, `duration` |
| `datetime.strftime(value, pattern)` | formats `date` or `datetime` with a standard directive subset |

`datetime.parse(..., datetime.as_datetime)` accepts:

* `T`, `t`, or a space between date and time
* optional fractional seconds
* optional offsets like `Z`, `+02`, `+0230`, `+02:30`
* no offset means UTC

`datetime.parse(..., datetime.as_duration)` accepts ISO 8601 day/time durations such as `P3D`,
`PT4H30M`, `P2DT1.5S`, `-PT15M`. Calendar years/months are intentionally
rejected because they are not fixed-length durations.

Supported `datetime.strftime(...)` directives:

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
use datetime
use result

val dt = result.raise(datetime.parse("2026-05-27T14:30:05.12+05:30", datetime.as_datetime))
emit datetime.isoformat(dt)
emit datetime.strftime(dt, "%F %T %:z")
'
```

```text
2026-05-27T14:30:05.12+05:30
2026-05-27 14:30:05 +05:30
```

## Clock And Epoch

| Builtin | Behavior |
| ------- | -------- |
| `datetime.now()` | current UTC `datetime` |
| `datetime.now(offset)` | current `datetime` rendered in a fixed offset |
| `datetime.today()` | current UTC `date` |
| `datetime.today(offset)` | current `date` in a fixed offset |
| `datetime.from_epoch(value, datetime.seconds)` | unix seconds to `datetime` |
| `datetime.from_epoch(value, datetime.seconds, offset)` | same instant rendered in another fixed offset |
| `datetime.from_epoch(value, datetime.milliseconds)` | unix milliseconds to `datetime` |
| `datetime.epoch(datetime, datetime.seconds)` | unix timestamp as `int` or `float` |
| `datetime.epoch(datetime, datetime.milliseconds)` | unix milliseconds as `int` or `float` |

`datetime.from_epoch(..., datetime.seconds)` accepts integer or floating-point seconds.
`datetime.from_epoch(..., datetime.milliseconds)`
accepts integer or floating-point milliseconds.

## Accessors

| Builtin | Behavior |
| ------- | -------- |
| `datetime.year(x)` | year from `date` or `datetime` |
| `datetime.month(x)` | month from `date` or `datetime` |
| `datetime.day(x)` | day from `date` or `datetime` |
| `datetime.hour(x)` | hour from `datetime` |
| `datetime.minute(x)` | minute from `datetime` |
| `datetime.second(x)` | second from `datetime` |
| `datetime.nanosecond(x)` | nanosecond from `datetime` |
| `datetime.weekday(x)` | ISO weekday (`1 = Monday`) |
| `datetime.weekday_name(x)` | full weekday name |
| `datetime.month_name(x)` | full month name |
| `datetime.ordinal_day(x)` | day-of-year (`1..366`) |
| `datetime.iso_week(x)` | map `{year, week}` |
| `datetime.offset_minutes(datetime)` | fixed UTC offset in minutes |
| `datetime.days_in_month(date_or_datetime)` | days in the current month |
| `datetime.days_in_month(year, month)` | days in a specific month |
| `datetime.is_leap_year(int_or_date_or_datetime)` | leap-year predicate |
| `datetime.date_only(x)` | extracts a `date` from `date` or `datetime` |

## Arithmetic And Comparison

| Builtin | Behavior |
| ------- | -------- |
| `datetime.with_offset(datetime, offset)` | same instant, different rendered offset |
| `datetime.add(x, n, datetime.days)` | shifts `date` or `datetime` by calendar days |
| `datetime.add(x, n, datetime.months)` | calendar month arithmetic with end-of-month clamping |
| `datetime.add(x, n, datetime.years)` | calendar year arithmetic with leap-day clamping |
| `datetime.add(datetime, duration)` | instant arithmetic |
| `datetime.add(duration, duration)` | duration arithmetic |
| `datetime.diff(a, b, datetime.days)` | difference in whole calendar days |
| `datetime.diff(a, b, datetime.seconds)` | difference in seconds |
| `datetime.diff(a, b, datetime.span)` | difference as `duration` |
| `datetime.bound(x, datetime.start)` | midnight `datetime` |
| `datetime.bound(x, datetime.end)` | `23:59:59.999999999` `datetime` |

Ordering works with `<`, `<=`, `>`, `>=` for matching temporal kinds:

* `date` compares by calendar day
* `datetime` compares by instant, not by textual representation
* `duration` compares by exact length

`datetime` equality also compares by instant:

```bash
./target/release/nodia eval '
use datetime
use result

emit result.raise(datetime.parse("2024-01-01T00:00:00+02:00", datetime.as_datetime)) == result.raise(datetime.parse("2023-12-31T22:00:00Z", datetime.as_datetime))
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
use datetime
use result

emit json.write({
  when: result.raise(datetime.parse("2024-02-29T12:00:00Z", datetime.as_datetime)),
  due: datetime.date(2024, 3, 5),
  retry_in: datetime.duration({minutes: 15}),
})
'
```

```text
{"due":"2024-03-05","retry_in":"PT15M","when":"2024-02-29T12:00:00Z"}
```
