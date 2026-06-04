# Format Builtins

These builtins cover the common formatting gap between interpolation and
full text generation.

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

```bash
./target/release/nodia eval '
emit format("%05d %.2f %-6s", [7, 3.5, "ok"])
'
```

```text
00007 3.50 ok    
```

## `pad_left(value, width)` / `pad_left(value, width, pad)`

Pads the string form of a value on the left:

```bash
./target/release/nodia eval '
emit pad_left("42", 5)
emit pad_left("42", 5, "0")
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
emit pad_right("ok", 5, ".")
'
```

```text
ok...
```

## `fixed(number, digits)`

Formats a number with an exact number of decimal places:

```bash
./target/release/nodia eval '
emit fixed(3.14159, 3)
emit fixed(12, 2)
'
```

```text
3.142
12.00
```
