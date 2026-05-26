# Strings & Interpolation

Nodia supports three string literal forms and runtime `{...}` interpolation.

## Literal Forms

### Double Quotes

```nodia
emit "hello"
```

### Single Quotes

```nodia
emit 'hello'
```

Single- and double-quoted strings are equivalent in v0.6 — they accept the
same escape sequences and use the same interpolation rules.

### Triple Quotes

```nodia
val config = """
APP_NAME=nodia
APP_ENV=prod
"""

emit config
```

Triple-quoted strings preserve their literal contents until the next `"""`
delimiter, including newlines.

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

Unknown escapes resolve to the escaped character itself in v0.6 (e.g. `\x`
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

Strings interpolate `{expr}` at runtime when evaluated:

```bash
./target/release/nodia eval '
val name = "Ana"
emit "Hello, {capitalize(name)}"
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

### Literal Braces

Use `{{` and `}}` to emit literal braces:

```bash
./target/release/nodia eval 'emit "{{value}}"'
```

```text
{value}
```

### Where Interpolation Runs

Interpolation is a runtime feature of string values, not a parse-time AST
node. The string keeps its source form until it is evaluated and converted to
text. This applies equally to literals, triple-quoted strings, and strings
returned from functions or builtins, as long as they are evaluated as strings.

### Errors

The checker rejects malformed interpolation:

* unterminated `{...}` → `E4106`
* empty interpolation `{}` → `E4106`

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
