# Values & Truthiness

Nodia is dynamically typed at runtime. Every value has one of a small,
fixed set of kinds.

## Value Kinds

| Kind       | Example                                              |
| ---------- | ---------------------------------------------------- |
| `null`     | `null`                                               |
| `bool`     | `true`, `false`                                      |
| `int`      | `42`, `-3`                                           |
| `float`    | `3.14`, `0.0`, `1e10`                                |
| `string`   | `"hello"`, `'hello'`, `r"hello"`, `"""triple"""`     |
| `bytes`    | `b"hello"`, `b"\xff\0"`                              |
| `list`     | `[1, 2, 3]`                                          |
| `map`      | `{name: "Ana", role: "dev"}`                         |
| `result`   | `result.ok("Ana")`, `result.err("E8000", "missing row")` |
| `date`     | `datetime.date(2026, 5, 27)`                         |
| `datetime` | `result.raise(datetime.parse("2026-05-27T14:30:05Z", datetime.as_datetime))` |
| `duration` | `datetime.duration({hours: 2, minutes: 30})`         |
| `stream`   | `io.stdin`, `io.stdout`, `io.stderr`, `result.raise(io.open("f.txt", "read"))` |
| `function` | `func greet(name) { ... }`                           |
| `regex`    | `regex { one_or_more digit }`                        |
| `use`      | result of a `use` declaration                        |

### Integers

Signed 64-bit integers. The minus sign is parsed as a unary operator, not as
part of the literal:

```nodia
val count = 42
val offset = -3
```

### Floats

Decimal literals with digits on both sides of `.` and optional scientific
notation:

```nodia
val ratio = 0.5
val total = 10.0
val huge = 1e10
val tiny = 1.5e-3
```

The trailing-dot form `10.` is **not** part of the language.

### Strings

See [Strings & Interpolation](strings.md).

### Bytes

`bytes` is the explicit raw-byte kind. Use it when a pipeline must delay or
avoid UTF-8 decoding entirely:

```nodia
val raw = b"\xef\xbb\xbfhi\0"
```

Byte literals do not interpolate. They support the standard single-line
escapes plus `\0` and `\xNN`. Non-ASCII characters inside a bytes literal are
encoded as UTF-8 bytes.

`bytes[index]` returns an `int` in `0..255`, and `collections.slice(bytes, ...)`
returns another `bytes` value. Common producers are `text.encode(..., text.utf8)`,
`io.read(..., io.bytes)`, and `system.exec(...).stdout`.

### Lists And Maps

See [Lists & Maps](collections.md).

### Results

`result` is the recoverable pipeline value introduced in `0.8.0`.

Import `use result` and construct explicit success/failure values:

```nodia
use result

val ok = result.ok("Ana")
val bad = result.err("E8000", "missing row")
```

Use `result.is_ok(...)`, `result.is_err(...)`, `result.value(...)`,
`result.value_or(...)`, and `result.error(...)` to inspect them.
`result.then(...)` and `result.recover(...)` transform success/error branches.
`result.raise(...)` converts a recoverable error back into a fatal runtime
failure.

### Dates, Datetimes, And Durations

These are first-class runtime values created through standard library
constructors and parsers. See [Date & Time Builtins](../stdlib/datetime.md).

## Truthiness

`if`, `while`, `and`, `or`, and `not` evaluate values for truthiness. The
rules are:

| Value      | Truthy?               |
| ---------- | --------------------- |
| `null`     | always false          |
| `bool`     | itself                |
| `int`      | false if `0`          |
| `float`    | false if `0.0`        |
| `string`   | false if empty (`""`) |
| `bytes`    | false if empty (`b""`) |
| `list`     | false if empty (`[]`) |
| `map`      | false if empty (`{}`) |
| `result`   | true for `ok(...)`, false for `err(...)` |
| `date`     | always true           |
| `datetime` | always true           |
| `duration` | always true           |
| `stream`   | always true           |
| `function` | always true           |
| `regex`    | always true           |
| `use`      | always true           |

Examples:

```bash
./target/release/nodia eval '
if "" {
  emit "truthy"
} else {
  emit "falsy"
}

if 0 { emit "n truthy" } else { emit "n falsy" }
if [] { emit "l truthy" } else { emit "l falsy" }
'
```

```text
falsy
n falsy
l falsy
```

## Equality

Equality (`==`, `!=`) is **structural** and **strict** — no implicit type
coercion across kinds:

```bash
./target/release/nodia eval '
emit null == null
emit null == false
emit 0 == false
emit "" == false
'
```

```text
true
false
false
false
```

Bytes compare by exact byte sequence. Lists and maps compare element-wise /
key-wise. `date` compares by calendar day, `datetime` compares by instant, and
`duration` compares by exact stored length. Functions, streams, regexes, and
`use` references compare by identity in v0.7.
