# Values & Truthiness

Nodia is dynamically typed at runtime. Every value has one of a small,
fixed set of kinds.

## Value Kinds

| Kind       | Example                                              |
| ---------- | ---------------------------------------------------- |
| `null`     | `null`                                               |
| `bool`     | `true`, `false`                                      |
| `int`      | `42`, `-3`                                           |
| `float`    | `3.14`, `0.0`                                        |
| `string`   | `"hello"`, `'hello'`, `"""triple"""`                 |
| `list`     | `[1, 2, 3]`                                          |
| `map`      | `{name: "Ana", role: "dev"}`                         |
| `stream`   | `stdin`, `stdout`, `stderr`, `open("f.txt", "read")` |
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

Decimal literals with digits on both sides of `.`:

```nodia
val ratio = 0.5
val total = 10.0
```

The trailing-dot form `10.` is **not** part of the language.

### Strings

See [Strings & Interpolation](strings.md).

### Lists And Maps

See [Lists & Maps](collections.md).

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
| `list`     | false if empty (`[]`) |
| `map`      | false if empty (`{}`) |
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

Lists and maps compare element-wise / key-wise. Functions, streams, regexes,
and `use` references compare by identity in v0.6.
