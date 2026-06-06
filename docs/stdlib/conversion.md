# Conversion Builtins

Nodia keeps conversions explicit. Equality is strict and there are no
implicit coercions across kinds — use these builtins when you need to move
between kinds.

Import this namespace with `use conversion`.

## `string(value)`

Converts any value to its text form. The result is the same string `emit`
uses when given that value:

```bash
./target/release/nodia eval '
use conversion
emit conversion.string(42)
emit conversion.string(3.14)
emit conversion.string(true)
emit conversion.string(null)
emit conversion.string([1, 2])
emit conversion.string({a: 1})
'
```

```text
42
3.14
true
null
[1, 2]
{a: 1}
```

`string(regex { ... })` returns the rendered regex text:

```bash
./target/release/nodia eval 'use conversion
emit conversion.string(regex { one_or_more digit })'
```

```text
\d+
```

## `bool(value)`

Folds a value to its truthiness:

```bash
./target/release/nodia eval '
use conversion
emit conversion.bool(null)
emit conversion.bool(0)
emit conversion.bool("")
emit conversion.bool([])
emit conversion.bool(1)
emit conversion.bool("text")
'
```

```text
false
false
false
false
true
true
```

Truthiness rules are the same ones `if` and `while` use; see
[Truthiness](../language/values.md#truthiness).

## `int(value)` / `float(value)`

See [Numbers](numbers.md#conversions).

## `len(value)`

Returns the Unicode scalar count for strings, the element count for lists, or the
entry count for maps. See [Collections](collections.md#length).
