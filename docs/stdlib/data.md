# Data Builtins

The public data modules consumed through `use json` and `use csv` come from the
Nodia stdlib in the project root. They are regular `.nod` modules, not the
internal Rust bootstrap layer.

## JSON

Import the module first:

```nodia
use json
```

### `json.parse(text)`

Parses JSON text and returns normal Nodia data:

* objects become maps
* arrays become lists
* strings stay strings
* integers become `int`
* decimal/exponent numbers become `float`
* `true`, `false`, `null` map directly

```bash
./target/release/nodia eval '
use json

val doc = json.parse("""
{"name":"Ana","meta":{"count":2},"flags":[true,false]}
""")
emit doc.name
emit doc.meta.count
emit doc.flags
'
```

```text
Ana
2
[true, false]
```

Malformed input raises `E4000` with `context = ["json.parse"]`.
If your source is raw bytes, decode it first:

```nodia
use json
use text
use io

val raw = io.read("payload.json", io.bytes)
val doc = json.parse(text.decode(raw, text.utf8))
```

### `json.stringify(value)`

Serializes a Nodia value into compact JSON text:

```bash
./target/release/nodia eval '
use json

val payload = {
  name: "Ana",
  active: true,
  scores: [1, 2, 3],
}
emit json.stringify(payload)
'
```

```text
{"active":true,"name":"Ana","scores":[1,2,3]}
```

Accepted kinds: `null`, `bool`, `int`, `float`, `string`, `list`, `map`.
Maps are emitted in deterministic lexicographic key order.
Unsupported values raise an error.

## CSV

Import the module first:

```nodia
use csv
```

### `csv.parse(text, false)`

Parses CSV text into a list of rows, where each row is a list of strings:

```bash
./target/release/nodia eval '
use csv

emit csv.parse("name,role\nAna,dev\n\"Bia, Jr\",ops", false)
'
```

```text
[["name", "role"], ["Ana", "dev"], ["Bia, Jr", "ops"]]
```

Quoted fields, escaped quotes, commas, and embedded newlines inside quoted
fields are supported.

### `csv.parse(text, true)`

When the second argument is `true`, the first row becomes the header and the
result becomes a list of maps:

```bash
./target/release/nodia eval '
use csv

val rows = csv.parse("name,role\nAna,dev\n\"Bia, Jr\",ops", true)
emit rows[0].name
emit rows[1]
'
```

```text
Ana
{name: "Bia, Jr", role: "ops"}
```

Malformed input raises a runtime error such as `E8101` or `E8102`.

### `csv.stringify(rows, headers)`

Serializes CSV text.

When `headers` is a list, rows are treated as maps and fields are emitted in
that order:

```bash
./target/release/nodia eval '
use csv

val rows = [
  {name: "Ana", role: "dev"},
  {name: "Bia, Jr", role: "ops"},
]
emit csv.stringify(rows, ["name", "role"])
'
```

```text
"name","role"
Ana,dev
"Bia, Jr",ops
```

When `headers` is `null`, rows are treated as lists:

```bash
./target/release/nodia eval '
use csv

emit csv.stringify([
  ["name", "role"],
  ["Ana", "dev"],
], null)
'
```

```text
name,role
Ana,dev
```
