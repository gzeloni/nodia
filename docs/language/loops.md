# Loops

Nodia has two loop forms: `for ... in` and `while`. Both support `break` and
`continue`.

## `for ... in`

`for` iterates over lists, strings, and maps.

| Iterable | `for value in ...` | `for (a, b) in ...` |
| -------- | ------------------ | ------------------- |
| list     | each list value    | pair-like items only |
| string   | one-character strings | not supported |
| map      | string keys (deterministic order) | key/value pairs |

### Over A List

```bash
./target/release/nodia eval '
for name in ["ana", "bruno"] {
  emit capitalize(name)
}
'
```

```text
Ana
Bruno
```

### Over A String

```bash
./target/release/nodia eval '
for ch in "nod" {
  emit ch
}
'
```

```text
n
o
d
```

### Over A Map

The iteration variable receives the keys:

```bash
./target/release/nodia eval '
val user = {name: "Ana", role: "dev"}
for key in user {
  emit "{key}={user[key]}"
}
'
```

```text
name=Ana
role=dev
```

Map keys are stored in deterministic sorted order; iteration matches `keys(map)`.

### Over A Map As Pairs

```bash
./target/release/nodia eval '
val user = {name: "Ana", role: "dev"}
for (key, value) in user {
  emit "{key}={value}"
}
'
```

```text
name=Ana
role=dev
```

### With `range`

`range(end)` and `range(start, end)` produce integer lists you can iterate
directly:

```bash
./target/release/nodia eval '
for n in range(4) {
  emit n
}
'
```

```text
0
1
2
3
```

`range(start, end)` works in either direction:

```bash
./target/release/nodia eval '
emit range(5, 2)
'
```

```text
[5, 4, 3]
```

## `while`

```bash
./target/release/nodia eval '
var n = 0
while n < 3 {
  emit n
  n = n + 1
}
'
```

```text
0
1
2
```

### Safety Cap

`while` enforces a runtime safety cap of **100 000 iterations**. Once the
counter exceeds the cap, the runtime raises a runtime error. This prevents
runaway loops from hanging automation pipelines.

```text
error[E2000]: while loop exceeded 100000 iterations
```

If you legitimately need a long loop, structure it around explicit data (for
example, iterating over a `range(...)`).

## `break`

`break` exits the innermost loop:

```bash
./target/release/nodia eval '
for n in range(10) {
  if n == 3 {
    break
  }
  emit n
}
'
```

```text
0
1
2
```

## `continue`

`continue` jumps to the next iteration of the innermost loop:

```bash
./target/release/nodia eval '
for n in range(5) {
  if n == 2 {
    continue
  }
  emit n
}
'
```

```text
0
1
3
4
```

## Control-Flow Placement

`break` and `continue` outside a loop are rejected by the checker (`E4103`).
`return` outside a function is also rejected (`E4103`).
