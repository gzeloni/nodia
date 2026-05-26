# Lists & Maps

Lists are ordered, zero-indexed collections of values. Maps are ordered,
string-keyed collections.

## Lists

### Literals

Inline:

```nodia
val tags = ["compiler", "formatter", "streams"]
```

Multi-line (canonical when long):

```nodia
val tags = [
  "compiler",
  "formatter",
  "streams",
]
```

Trailing commas are allowed and produced by the formatter for multi-line
literals.

### Indexing

Zero-based. Negative indexes count from the end:

```bash
./target/release/nodia eval '
val tags = ["a", "b", "c"]
emit tags[0]
emit tags[-1]
'
```

```text
a
c
```

Out-of-bounds access is a runtime error (`E2000`).

### Iteration

```bash
./target/release/nodia eval '
for tag in ["compiler", "formatter"] {
  emit upper(tag)
}
'
```

```text
COMPILER
FORMATTER
```

### Building Lists

List helpers are **non-mutating** — they return new lists:

```bash
./target/release/nodia eval '
var values = []
values = push(values, "a")
values = push(values, "b")
emit values
'
```

```text
[a, b]
```

See [Collections builtins](../stdlib/collections.md) for the full list of
helpers (`push`, `pop`, `first`, `last`, `slice`, `reverse`, `sort`, `unique`,
`len`).

## Maps

### Literals

Inline:

```nodia
val user = {name: "Ana", role: "dev"}
```

Canonical (multi-line for non-empty maps):

```nodia
val user = {
  name: "Ana",
  role: "dev",
}
```

Keys can be identifiers, reserved words in key position, or strings. String
keys are required when the key contains characters outside the identifier
grammar:

```bash
./target/release/nodia eval '
val data = {
  from: "api",
  name: "Ana",
  "full name": "Ana Maria",
}
emit data["from"]
emit data["full name"]
'
```

```text
api
Ana Maria
```

### Field And Index Access

```bash
./target/release/nodia eval '
val user = {name: "Ana", role: "dev"}
emit user.name
emit user["role"]
'
```

```text
Ana
dev
```

Field access on a non-map value, or on a missing key, is a runtime error.
The checker rejects field access on **known** literal-map shapes when the field
is statically absent (`E4105`).

### Field And Index Assignment

Mutable map and list bindings support in-place updates:

```bash
./target/release/nodia eval '
var user = {}
user.name = "Ana"
user["role"] = "dev"
emit user
'
```

```text
{name: Ana, role: dev}
```

Map assignment inserts or replaces the final key. List assignment replaces an
existing index. The root binding must be a `var` (or a mutable binding imported
through `use`).

### Iteration

`for ... in map` iterates over keys (sorted deterministically):

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

You can also destructure key/value pairs directly:

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

### Builtins For Maps

* `len(map)` — number of entries.
* `keys(map)` — list of keys.
* `values(map)` — list of values.
* `entries(map)` — list of `{key, value}` items.
* `contains(map, key)` — `true` if the key is present.

```bash
./target/release/nodia eval '
val u = {name: "Ana", role: "dev"}
emit keys(u)
emit values(u)
emit entries(u)
emit len(u)
emit contains(u, "name")
'
```

```text
[name, role]
[Ana, dev]
[{key: name, value: Ana}, {key: role, value: dev}]
2
true
```

## Mutability

* `val list = [...]` binds the list immutably — you cannot reassign the
  binding itself. The list contents are still values; helpers like `push`
  return new lists rather than mutating in place.
* `var list = [...]` lets you reassign the binding (e.g. to the result of
  `push`) or update an existing list index.
* The same rule applies to maps. `var map = {}` allows `map[key] = value` and
  `map.field = value`, while `val map = {}` rejects those updates.
