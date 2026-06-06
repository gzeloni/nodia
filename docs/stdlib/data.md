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

Parses either a JSON string or a UTF-8 byte sequence (`list<int>`) into Nodia
values:

* objects become maps;
* arrays become lists;
* strings stay strings;
* integers become `int`;
* decimal/exponent numbers become `float`;
* `true`, `false`, `null` map directly.

```bash
./target/release/nodia eval '
use json
use text

val doc = json.read(text.encode_utf8("""
{"name":"Ana","meta":{"count":2},"flags":[true,false]}
"""))
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
entries. When the first argument is bytes, `json.read(...)` decodes them as
strict UTF-8 before parsing. Invalid UTF-8 is a runtime error. If your source
can be dirty, sanitize it first with `text.strip_bom(...)`,
`text.normalize_lf(...)`, `text.drop_nul(...)`, or decode lossily with
`text.decode_utf8_lossy(...)` before handing the resulting text to `json.read`.

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
list of strings:

```bash
./target/release/nodia eval '
use csv
use text

emit csv.read(text.encode_utf8("name,role\nAna,dev\n\"Bia, Jr\",ops"))
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
use text

val rows = csv.read(text.encode_utf8("name,role\nAna,dev\n\"Bia, Jr\",ops"), true)
emit rows[0].name
emit rows[1]
'
```

```text
Ana
{name: "Bia, Jr", role: "ops"}
```

Duplicate header names are rejected instead of silently collapsing fields.
When the first argument is bytes, `csv.read(...)` decodes them as strict
UTF-8 before parsing.

### `csv.read(text_or_bytes, {header: true, types: true})`

Use an options map when you want header rows and scalar type coercion:

```bash
./target/release/nodia eval '
use csv

val rows = csv.read("name,age,active\nAna,30,true", {
  header: true,
  types: true,
})
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
