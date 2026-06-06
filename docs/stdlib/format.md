# Format Builtins

These builtins cover the common formatting gap between interpolation and
full text generation.

Import this namespace with `use format`.

## `format(fmt, values)`

`format` accepts a printf-style format string plus a list of values:

* `%s` — string form
* `%d` — integer
* `%f` — float
* `%%` — literal percent sign

Supported modifiers:

* width: `%5d`
* left-align: `%-8s`
* zero-pad: `%05d`
* precision: `%.2f`, `%.3s`

For `%s`, precision truncates characters from the rendered string form. `%%`
emits a literal percent sign.

```bash
./target/release/nodia eval '
use format
emit format.format("%05d %.2f %-6s", [7, 3.5, "ok"])
'
```

```text
00007 3.50 ok    
```

## `pad_left(value, width)` / `pad_left(value, width, pad)`

Pads the string form of a value on the left:

```bash
./target/release/nodia eval '
use format
emit format.pad_left("42", 5)
emit format.pad_left("42", 5, "0")
'
```

```text
   42
00042
```

## `pad_right(value, width)` / `pad_right(value, width, pad)`

Pads on the right:

```bash
./target/release/nodia eval '
use format
emit format.pad_right("ok", 5, ".")
'
```

```text
ok...
```

When the `pad` string is longer than one character, Nodia repeats and truncates
it to fit the requested width exactly.

## `fixed(number, digits)`

Formats a number with an exact number of decimal places:

```bash
./target/release/nodia eval '
use format
emit format.fixed(3.14159, 3)
emit format.fixed(12, 2)
'
```

```text
3.142
12.00
```
