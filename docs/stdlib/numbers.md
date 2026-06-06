# Number Builtins

Import this namespace with `use numbers`.

## Conversions

### `int(value)`

* String → integer (parse).
* Float → integer (truncates toward zero).
* Int → integer (identity).

```bash
./target/release/nodia eval '
use numbers
emit numbers.int("42")
emit numbers.int(3.9)
'
```

```text
42
3
```

### `float(value)`

* String → float (parse).
* Int → float.
* Float → float (identity).

```bash
./target/release/nodia eval '
use numbers
emit numbers.float("42")
emit numbers.float(3)
'
```

```text
42.0
3.0
```

## Math

### `abs(n)`

```bash
./target/release/nodia eval 'use numbers
emit numbers.abs(-10)'
```

```text
10
```

### `floor(n)` / `ceil(n)` / `round(n)`

```bash
./target/release/nodia eval '
use numbers
emit numbers.floor(3.9)
emit numbers.ceil(3.1)
emit numbers.round(3.5)
'
```

```text
3
4
4
```

### `sqrt(n)`

```bash
./target/release/nodia eval 'use numbers
emit numbers.sqrt(9)'
```

```text
3.0
```

`sqrt` always returns a `float`.

### `pow(a, b)`

```bash
./target/release/nodia eval 'use numbers
emit numbers.pow(2, 8)'
```

```text
256
```

### `min(a, b)` / `max(a, b)`

```bash
./target/release/nodia eval '
use numbers
emit numbers.min(10, 3)
emit numbers.max(10, 3)
'
```

```text
3
10
```

### `clamp(n, min, max)`

```bash
./target/release/nodia eval '
use numbers
emit numbers.clamp(12, 0, 10)
emit numbers.clamp(-1, 0, 10)
emit numbers.clamp(5, 0, 10)
'
```

```text
10
0
5
```

## Aggregates

### `sum(list)`

```bash
./target/release/nodia eval 'use numbers
emit numbers.sum([1, 2, 3])'
```

```text
6
```

### `avg(list)`

Returns `null` for an empty list:

```bash
./target/release/nodia eval '
use numbers
emit numbers.avg([1, 2, 3])
emit numbers.avg([])
'
```

```text
2.0
null
```

## Ranges

### `range(end)`

Integers from `0` (inclusive) to `end` (exclusive):

```bash
./target/release/nodia eval 'use numbers
emit numbers.range(4)'
```

```text
[0, 1, 2, 3]
```

### `range(start, end)`

Bidirectional — if `start > end`, the range counts down:

```bash
./target/release/nodia eval '
use numbers
emit numbers.range(2, 5)
emit numbers.range(5, 2)
'
```

```text
[2, 3, 4]
[5, 4, 3]
```

In both directions, the `end` value is excluded.
