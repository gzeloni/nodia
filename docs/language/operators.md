# Operators

Nodia uses word-form logical operators (`and`, `or`, `not`) and standard
symbolic arithmetic / comparison operators. Bitwise operators are available for
`int` values only. There is no operator overloading and no user-defined
operators.

## Arithmetic

```bash
./target/release/nodia eval '
emit 1 + 2
emit 5 - 3
emit 4 * 2
emit 7 % 3
emit 9 / 3
emit 10 / 3
'
```

```text
3
2
8
1
3.0
3.3333333333333335
```

| Operator | Meaning                                            |
| -------- | -------------------------------------------------- |
| `+`      | numeric addition; string concatenation when at least one operand is a string |
| `-`      | numeric subtraction                                |
| `*`      | numeric multiplication                             |
| `/`      | numeric division (always returns float) |
| `%`      | numeric remainder (sign follows the dividend)      |

## Bitwise

Bitwise operators only accept `int` operands:

```bash
./target/release/nodia eval '
emit ~1
emit 5 & 3
emit 5 | 2
emit 5 ^ 1
emit 1 << 3
emit 8 >> 2
emit 1 | 2 & 4
emit (1 | 2) & 7
'
```

```text
-2
1
7
4
8
2
1
3
```

| Operator | Meaning |
| -------- | ------- |
| `~x`     | bitwise complement |
| `&`      | bitwise AND |
| `|`      | bitwise OR |
| `^`      | bitwise XOR |
| `<<`     | left shift |
| `>>`     | right shift |

Shift counts must be between `0` and `63`.

Mixing `int` and `float` produces a `float`:

```bash
./target/release/nodia eval '
emit 2 + 3.5
emit 2.0 + 3
'
```

```text
5.5
5.0
```

## String Concatenation

`+` concatenates when at least one operand is a string:

```bash
./target/release/nodia eval '
emit "x" + 1
emit "a" + "b"
'
```

```text
x1
ab
```

For larger templates, prefer interpolation:

```nodia
emit "x={x} y={y}"
```

## Comparison

```bash
./target/release/nodia eval '
emit 1 < 2
emit 1 <= 1
emit 2 > 1
emit 2 >= 2
'
```

```text
true
true
true
true
```

| Operator | Meaning              |
| -------- | -------------------- |
| `<`      | less than            |
| `<=`     | less than or equal   |
| `>`      | greater than         |
| `>=`     | greater than or equal|

## Equality

```bash
./target/release/nodia eval '
emit "a" == "a"
emit "a" != "b"
emit [1, 2] == [1, 2]
emit {a: 1} == {a: 1}
'
```

```text
true
true
true
true
```

Equality is **strict** — no implicit coercion across kinds:

```bash
./target/release/nodia eval '
emit 5 == 5.0
emit null == false
'
```

```text
false
false
```

## Logical

Use words, not symbols:

```bash
./target/release/nodia eval '
emit true and not false
emit false or true
'
```

```text
true
true
```

`!`, `&&`, and `||` are not accepted. Use `not`, `and`, and `or` instead.

`and` and `or` short-circuit on truthiness (see
[Truthiness](values.md#truthiness)). They return one of the original operands,
not a coerced bool:

```bash
./target/release/nodia eval '
emit "" or "fallback"
emit "value" and "kept"
'
```

```text
fallback
kept
```

## Unary

| Operator | Meaning                                |
| -------- | -------------------------------------- |
| `-x`     | numeric negation                       |
| `not x`  | logical negation, returns `true`/`false` |
| `~x`     | bitwise complement                     |

## Precedence

Lowest to highest:

| Level | Operators                                | Associativity |
| ----- | ---------------------------------------- | ------------- |
| 1     | `or`                                     | left          |
| 2     | `and`                                    | left          |
| 3     | `==`, `!=`                               | left          |
| 4     | `<`, `<=`, `>`, `>=`                     | left          |
| 5     | `|`                                      | left          |
| 6     | `^`                                      | left          |
| 7     | `&`                                      | left          |
| 8     | `<<`, `>>`                               | left          |
| 9     | `+`, `-`                                 | left          |
| 10    | `*`, `/`, `%`                            | left          |
| 11    | unary `-`, `not`, `~`                    | right         |
| 12    | call, field access, index access         | left          |
| 13    | literals, identifiers, grouped expressions | n/a         |

Use parentheses to override precedence:

```bash
./target/release/nodia eval '
emit 1 + 2 * 3
emit (1 + 2) * 3
'
```

```text
7
9
```
