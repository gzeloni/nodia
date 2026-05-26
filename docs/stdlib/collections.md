# Collection Builtins

All collection builtins are pure — they return new values instead of mutating
in place.

## Length

### `len(value)`

Length of a string, list, or map.

```bash
./target/release/nodia eval '
emit len("abc")
emit len([1, 2, 3])
emit len({name: "Ana"})
'
```

```text
3
3
1
```

`len` on a string is a character count.

## Map Helpers

### `keys(map)`

Returns the keys in deterministic sorted order:

```bash
./target/release/nodia eval 'emit keys({name: "Ana", role: "dev"})'
```

```text
[name, role]
```

### `values(map)`

Returns the values, ordered by key:

```bash
./target/release/nodia eval 'emit values({name: "Ana", role: "dev"})'
```

```text
[Ana, dev]
```

### `entries(map)`

Returns the entries as `{key, value}` maps, ordered by key:

```bash
./target/release/nodia eval 'emit entries({name: "Ana", role: "dev"})'
```

```text
[{key: name, value: Ana}, {key: role, value: dev}]
```

### `contains(map, key)`

See [`contains`](text.md#tests) — works on strings, lists, and maps.

## List Helpers

### `push(list, value)`

Returns a new list with `value` appended:

```bash
./target/release/nodia eval 'emit push([1, 2], 3)'
```

```text
[1, 2, 3]
```

### `pop(list)`

Returns a new list with its last value removed. On an empty list, returns
an empty list:

```bash
./target/release/nodia eval '
emit pop([1, 2, 3])
emit pop([])
'
```

```text
[1, 2]
[]
```

### `first(list)` / `last(list)`

Return the first / last value, or `null` if the list is empty:

```bash
./target/release/nodia eval '
emit first(["a", "b"])
emit last(["a", "b"])
emit first([])
'
```

```text
a
b
null
```

### `slice(value, start, end)`

Slices a list or string by index. Negative indexes count from the end:

```bash
./target/release/nodia eval '
emit slice(["a", "b", "c", "d"], 1, 3)
emit slice(["a", "b", "c", "d"], -3, -1)
emit slice("nodia", 1, 4)
'
```

```text
[b, c]
[b, c]
odi
```

`start` is inclusive, `end` is exclusive. Out-of-bounds bounds are clamped
gracefully rather than raising — but you should still pass sensible bounds.

### `reverse(value)`

Reverses a list or string:

```bash
./target/release/nodia eval '
emit reverse([1, 2, 3])
emit reverse("abc")
'
```

```text
[3, 2, 1]
cba
```

### `sort(list)`

Sorts a list deterministically. Numeric and string lists sort with a natural
order:

```bash
./target/release/nodia eval '
emit sort([3, 1, 2])
emit sort(["c", "a", "b"])
'
```

```text
[1, 2, 3]
[a, b, c]
```

### `unique(list)`

Removes duplicates while preserving the original order:

```bash
./target/release/nodia eval 'emit unique(["a", "b", "a", "c", "b"])'
```

```text
[a, b, c]
```
