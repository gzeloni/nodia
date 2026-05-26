# Number Builtins

## Conversions

### `int(value)`

* String → integer (parse).
* Float → integer (truncates toward zero).
* Int → integer (identity).

```bash
./target/release/nodia eval '
emit int("42")
emit int(3.9)
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
emit float("42")
emit float(3)
'
```

```text
42.0
3.0
```

## Math

### `abs(n)`

```bash
./target/release/nodia eval 'emit abs(-10)'
```

```text
10
```

### `floor(n)` / `ceil(n)` / `round(n)`

```bash
./target/release/nodia eval '
emit floor(3.9)
emit ceil(3.1)
emit round(3.5)
'
```

```text
3
4
4
```

### `sqrt(n)`

```bash
./target/release/nodia eval 'emit sqrt(9)'
```

```text
3.0
```

`sqrt` always returns a `float`.

### `pow(a, b)`

```bash
./target/release/nodia eval 'emit pow(2, 8)'
```

```text
256
```

### `min(a, b)` / `max(a, b)`

```bash
./target/release/nodia eval '
emit min(10, 3)
emit max(10, 3)
'
```

```text
3
10
```

### `clamp(n, min, max)`

```bash
./target/release/nodia eval '
emit clamp(12, 0, 10)
emit clamp(-1, 0, 10)
emit clamp(5, 0, 10)
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
./target/release/nodia eval 'emit sum([1, 2, 3])'
```

```text
6
```

### `avg(list)`

Returns `null` for an empty list:

```bash
./target/release/nodia eval '
emit avg([1, 2, 3])
emit avg([])
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
./target/release/nodia eval 'emit range(4)'
```

```text
[0, 1, 2, 3]
```

### `range(start, end)`

Bidirectional — if `start > end`, the range counts down:

```bash
./target/release/nodia eval '
emit range(2, 5)
emit range(5, 2)
'
```

```text
[2, 3, 4]
[5, 4, 3]
```

In both directions, the `end` value is excluded.
