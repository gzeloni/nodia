# Data Builtins

Structured data is where small scripting languages usually either become
useful or fall apart. Nodia now exposes JSON and CSV through stdlib modules,
so this surface can grow without flooding the global namespace.

## JSON

Import the module first:

```nodia
use json
```

### `json.read(text_or_bytes)`

Parses either a JSON string or a UTF-8 byte sequence (`bytes`) and returns a
`result` whose success value is normal Nodia data:

* objects become maps;
* arrays become lists;
* strings stay strings;
* integers become `int`;
* decimal/exponent numbers become `float`;
* `true`, `false`, `null` map directly.

```bash
./target/release/nodia eval '
use json
use result
use text

val doc = result.raise(json.read(text.encode("""
{"name":"Ana","meta":{"count":2},"flags":[true,false]}
""", text.utf8)))
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

When you embed JSON directly inside source, prefer `r'...'` or triple-quoted
strings. `r"..."` usually is not a good JSON delimiter because the first `"`
inside the JSON closes the raw string.
Duplicate object keys are rejected instead of silently overwriting earlier
entries. When the first argument is bytes, `json.read(...)` decodes them with
`text.decode(..., text.utf8)` before parsing. Invalid UTF-8 or malformed JSON
produce `err(...)`. Recoverable JSON failures also expose `context` and, when
applicable, a nested `span` through `result.error(...)`. If your source can be
dirty, sanitize it first with
`text.strip_bom(...)`,
`text.normalize(..., text.lf)`, `text.drop_nul(...)`, or decode lossily with
`text.decode(..., text.utf8, text.lossy)` before handing the resulting text to
`json.read`.

### `json.write(value)`

Serializes a Nodia value into compact JSON text:

```bash
./target/release/nodia eval '
use json

val payload = {
  name: "Ana",
  active: true,
  scores: [1, 2, 3],
}
emit json.write(payload)
'
```

```text
{"active":true,"name":"Ana","scores":[1,2,3]}
```

Accepted kinds: `null`, `bool`, `int`, `float`, `string`, `list`, `map`,
`date`, `datetime`, `duration`.
Temporal values are emitted as ISO strings. Functions, streams, regex values,
and `use` bindings are rejected.
Map keys are emitted in deterministic lexicographic order, not insertion order.

### `json.write(value, 2)`

With a second integer argument, JSON output is pretty-printed with that many
spaces per indent level. Omitting the argument, or passing `0`, keeps compact
output:

```bash
./target/release/nodia eval '
use json

emit json.write({
  name: "Ana",
  scores: [1, 2],
}, 2)
'
```

```text
{
  "name": "Ana",
  "scores": [
    1,
    2
  ]
}
```

### `json.write(value, {indent: 2})`

The second argument can also be an options map:

```bash
./target/release/nodia eval '
use json

emit json.write({
  name: "Ana",
  scores: [1, 2],
}, {indent: 2})
'
```

This behaves the same as `json.write(value, 2)`.
Only the `indent` option is accepted; unknown option keys are rejected.

## CSV

Import the module first:

```nodia
use csv
```

### `csv.read(text_or_bytes)`

Parses either CSV text or UTF-8 bytes into a list of rows, where each row is a
list of strings. The call returns a `result`.

```bash
./target/release/nodia eval '
use csv
use result
use text

emit result.raise(csv.read(text.encode("name,role\nAna,dev\n\"Bia, Jr\",ops", text.utf8)))
'
```

```text
[["name", "role"], ["Ana", "dev"], ["Bia, Jr", "ops"]]
```

Quoted fields, escaped quotes, commas, and embedded newlines inside quoted
fields are supported.

### `csv.read(text_or_bytes, true)`

With a second `true` argument, the first row is treated as the header and the
result becomes a list of maps:

```bash
./target/release/nodia eval '
use csv
use result
use text

val rows = result.raise(csv.read(text.encode("name,role\nAna,dev\n\"Bia, Jr\",ops", text.utf8), true))
emit rows[0].name
emit rows[1]
'
```

```text
Ana
{name: "Bia, Jr", role: "ops"}
```

Duplicate header names are rejected instead of silently collapsing fields.
When the first argument is bytes, `csv.read(...)` decodes them with
`text.decode(..., text.utf8)` before parsing. Bad UTF-8 or malformed CSV
produce `err(...)`. Recoverable CSV failures also expose `context` and nested
`span` details through `result.error(...)`.

### `csv.read(text_or_bytes, {header: true, types: true})`

Use an options map when you want header rows and scalar type coercion:

```bash
./target/release/nodia eval '
use csv
use result

val rows = result.raise(csv.read("name,age,active\nAna,30,true", {
  header: true,
  types: true,
}))
emit rows[0].age + 2
emit rows[0].active
'
```

```text
32
true
```

Only `header` and `types` are accepted in the options map; unknown keys are
rejected.

### `csv.write(rows)`

Serializes CSV from either:

* a list of list rows; or
* a list of maps, in which case Nodia writes a header row first.

```bash
./target/release/nodia eval '
use csv

val rows = [
  {name: "Ana", role: "dev"},
  {name: "Bia, Jr", role: "ops"},
]
emit csv.write(rows)
'
```

```text
name,role
Ana,dev
"Bia, Jr",ops
```

For map rows, the header is the deterministic lexicographic union of every key
present in the input rows. Missing values serialize as empty fields.
