# Format Builtins (REMOVED from stdlib)

> **Deprecated**: The `format` module has been removed from the Rust standard
> library. It will be reimplemented in Nodia itself in the next major version.
> This documentation is kept for reference only.

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

For `%s`, precision truncates grapheme clusters from the rendered string form.
String width and `pad(...)` width are also grapheme
based, so visible characters with combining marks stay intact. `%%` emits a
literal percent sign.

```bash
./target/release/nodia eval '
use format
emit format.format("%05d %.2f %-6s", [7, 3.5, "ok"])
'
```

```text
00007 3.50 ok    
```

Grapheme-aware string formatting:

```bash
./target/release/nodia eval '
use format
emit format.format("[%2s][%.1s]", ["é", "éx"])
'
```

```text
[ é][é]
```

## `pad(value, width, align)` / `pad(value, width, align, pad)`

Pads the string form of a value to the requested width. Use `format.left` or
`format.right` as the alignment mode:

```bash
./target/release/nodia eval '
use format
emit format.pad("42", 5, format.left)
emit format.pad("42", 5, format.left, "0")
'
```

```text
   42
00042
```

Right padding:

```bash
./target/release/nodia eval '
use format
emit format.pad("ok", 5, format.right, ".")
'
```

```text
ok...
```

When the `pad` string is longer than one character, Nodia repeats and truncates
it to fit the requested width exactly. Width is still measured in grapheme
clusters, not scalar values.

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
