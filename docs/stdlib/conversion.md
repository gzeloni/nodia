# Conversion Builtins

Nodia keeps conversions explicit. Equality is strict and there are no
implicit coercions across kinds — use these builtins when you need to move
between kinds.

## `string(value)`

Converts any value to its text form. The result is the same string `emit`
uses when given that value:

```bash
./target/release/nodia eval '
emit string(42)
emit string(3.14)
emit string(true)
emit string(null)
emit string([1, 2])
emit string({a: 1})
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
./target/release/nodia eval 'emit string(regex { one_or_more digit })'
```

```text
\d+
```

## `bool(value)`

Folds a value to its truthiness:

```bash
./target/release/nodia eval '
emit bool(null)
emit bool(0)
emit bool("")
emit bool([])
emit bool(1)
emit bool("text")
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

Returns the character count for strings, the element count for lists, or the
entry count for maps. See [Collections](collections.md#length).
