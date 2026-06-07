# Strings & Interpolation

Nodia supports interpolated strings, raw strings, and raw triple-quoted
blocks.

## Literal Forms

### Double Quotes

```nodia
emit "hello"
```

### Single Quotes

```nodia
emit 'hello'
```

Single- and double-quoted strings are equivalent in v0.7.5 — they accept the
same escape sequences and use the same interpolation rules.

### Byte Literals

Use `b"..."` or `b'...'` for raw byte buffers:

```nodia
emit b"ABC"
emit b"\xff\0"
```

Byte literals are not strings: they do not interpolate, and they produce a
`bytes` value instead of `string`. They support `\n`, `\r`, `\t`, `\0`,
`\\`, `\"`, `\'`, and `\xNN`. Non-ASCII characters inside the literal are
encoded as UTF-8 bytes.

### Raw Strings

Use `r"..."` or `r'...'` when you want literal braces, backslashes, or JSON
snippets without interpolation:

```nodia
emit r'{"name":"Ana","tpl":"hello {world}"}'
emit r"\n stays backslash-n"
```

Raw strings do not process escapes and do not interpolate `{...}`.

For inline JSON, prefer `r'...'` or `"""..."""`. `r"..."` closes on the next
double quote, so it is usually the wrong delimiter for JSON text.

### Triple Quotes

```nodia
val config = """
APP_NAME=nodia
APP_ENV=prod
"""

emit config
```

Triple-quoted strings preserve their literal contents until the next `"""`
delimiter, including newlines. Like raw strings, they do not process escapes
and do not interpolate `{...}`.

## Escapes

Single-line strings recognize the following escape sequences:

| Escape | Meaning         |
| ------ | --------------- |
| `\n`   | newline         |
| `\r`   | carriage return |
| `\t`   | tab             |
| `\"`   | double quote    |
| `\'`   | single quote    |
| `\\`   | backslash       |

Unknown escapes resolve to the escaped character itself in v0.7 (e.g. `\x`
becomes `x`).

```bash
./target/release/nodia eval 'emit "line 1\nline 2"'
```

```text
line 1
line 2
```

```bash
./target/release/nodia eval 'emit "tab\there"'
```

```text
tab	here
```

## Interpolation

Single- and double-quoted strings interpolate `{expr}` at runtime when
evaluated:

```bash
./target/release/nodia eval '
use text
val name = "Ana"
emit "Hello, {text.capitalize(name)}"
'
```

```text
Hello, Ana
```

The expression can be any valid Nodia expression:

```bash
./target/release/nodia eval '
val a = 2
val b = 3
emit "sum={a + b}"
'
```

```text
sum=5
```

Balanced braces inside the expression are supported, so map literals and regex
blocks work inside interpolation when needed:

```bash
./target/release/nodia eval 'emit "{ {name: \"Ana\"}[\"name\"] }"'
```

```text
Ana
```

### Literal Braces

Use `{{` and `}}` to emit literal braces inside interpolated strings:

```bash
./target/release/nodia eval 'emit "{{value}}"'
```

```text
{value}
```

Treat `{{` / `}}` as the stable escape form. A lone `}` currently survives
literally as an implementation quirk and should not be relied on.

### Where Interpolation Runs

Interpolation happens when an interpolated source literal is evaluated. After
that, the resulting `string` value is plain text. Raw strings, triple-quoted
strings, JSON-parsed strings, and other runtime string values are not
re-interpolated.

### Errors

The checker rejects malformed interpolation:

* unterminated `{...}` → `E4106`
* empty interpolation `{}` → `E4106`
* invalid interpolation expressions such as `{name +}` → `E4106`

## Concatenation

`+` concatenates when at least one operand is a string. Numbers and booleans
are converted via their natural display form:

```bash
./target/release/nodia eval '
emit "x" + 1
emit "x" + true
emit "a" + "b"
'
```

```text
x1
xtrue
ab
```

For larger templates, prefer interpolation over `+`.
