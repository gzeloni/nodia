# Data Builtins (REMOVED from stdlib)

> **Deprecated**: The `json`, `yaml`, `toml`, and `csv` modules have been removed
> from the Rust standard library. They will be reimplemented in Nodia itself in
> the next major version. This documentation is kept for reference only.

Structured data is where small scripting languages usually either become
useful or fall apart. Nodia now exposes JSON, YAML, and CSV through stdlib
modules, so this surface can grow without flooding the global namespace.

## JSON

Import the module first:

```nodia
use json
```

### `json.read(text_or_bytes)`

Parses either a JSON string or a UTF-8 byte sequence (`bytes`) and returns
normal Nodia data:

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

val doc = json.read(text.encode("""
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
throw. Caught JSON failures expose `context` and, when applicable, a nested
`span`. If your source can be
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

## YAML

Import the module first:

```nodia
use yaml
```

### `yaml.read(text_or_bytes)`

Parses a practical block-style YAML subset and returns normal Nodia data:

* maps become maps;
* `-` blocks become lists;
* `true`, `false`, `null`, integers, and floats become scalar values;
* quoted or plain scalars become strings when they do not match those literals.

```bash
./target/release/nodia eval '
use yaml

val doc = yaml.read("""
name: Ana
active: true
scores:
  - 7
  - 8
meta:
  city: Rio
  tags: []
""")
emit doc.name
emit doc.meta.city
emit doc.scores
'
```

```text
Ana
Rio
[7, 8]
```

Supported today:

* nested maps and lists by indentation;
* plain scalars;
* single-quoted and double-quoted strings;
* empty `[]` and `{}`;
* `---` / `...` document markers;
* `#` comments outside quoted strings.

Deliberately not supported in this first cut:

* anchors and aliases;
* tags;
* block scalars (`|` / `>`);
* inline map items after `-`, such as `- name: Ana`.

When the first argument is bytes, `yaml.read(...)` decodes them with
`text.decode(..., text.utf8)` before parsing. Invalid UTF-8 or malformed YAML
throw. Caught YAML failures expose `context = ["yaml.read"]` and nested
`span` details.

### `yaml.write(value)`

Serializes a Nodia value into deterministic block-style YAML text using
2-space indentation:

```bash
./target/release/nodia eval '
use yaml

emit yaml.write({
  name: "Ana",
  scores: [1, 2],
  meta: {
    city: "Rio",
  },
})
'
```

```text
meta:
  city: Rio
name: Ana
scores:
  - 1
  - 2
```

Accepted kinds: `null`, `bool`, `int`, `float`, `string`, `list`, `map`,
`date`, `datetime`, `duration`.
Temporal values serialize as ISO strings.
Functions, streams, regex values, scanners, lazy values, bytes, and `use`
bindings are rejected.
Map keys are emitted in deterministic lexicographic order.

## TOML

Import the module first:

```nodia
use toml
```

### `toml.read(text_or_bytes)`

Parses a practical TOML subset and returns normal Nodia data:

* root keys become map fields;
* `[section]` headers become nested maps;
* dotted keys extend nested maps;
* strings, booleans, integers, floats, and arrays are supported.

```bash
./target/release/nodia eval '
use toml

val app = toml.read("""
name = "Ana"
ports = [80, 443]

[meta]
mode = "dev"
""")
emit app.name
emit app.meta.mode
emit app.ports
'
```

```text
Ana
dev
[80, 443]
```

Supported today:

* quoted or bare keys;
* quoted strings;
* booleans, integers, floats;
* arrays of supported scalar values;
* nested tables via `[section]`.

Deliberately not supported in this first cut:

* array tables (`[[item]]`);
* inline tables (`{a = 1}`);
* multiline strings;
* datetime literals as native TOML scalars.

When the first argument is bytes, `toml.read(...)` decodes them with
`text.decode(..., text.utf8)` before parsing. Invalid UTF-8 or malformed TOML
throw. Caught TOML failures expose `context = ["toml.read"]` and nested
`span` details.

### `toml.write(value)`

Serializes a Nodia map into deterministic TOML text:

```bash
./target/release/nodia eval '
use toml

emit toml.write({
  name: "Ana",
  active: true,
  meta: {
    mode: "dev",
  },
})
'
```

```text
active = true
name = "Ana"

[meta]
mode = "dev"
```

Accepted root kind: `map`.
Nested maps serialize as tables. Lists serialize as arrays.
Temporal values serialize as quoted ISO strings.
`null`, functions, streams, regex values, scanners, lazy values, bytes, and
`use` bindings are rejected.

## CSV

Import the module first:

```nodia
use csv
```

### `csv.read(text_or_bytes)`

Parses either CSV text or UTF-8 bytes into a list of rows, where each row is a
list of strings.

```bash
./target/release/nodia eval '
use csv
use text

emit csv.read(text.encode("name,role\nAna,dev\n\"Bia, Jr\",ops", text.utf8))
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

val rows = csv.read(text.encode("name,role\nAna,dev\n\"Bia, Jr\",ops", text.utf8), true)
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
throw. Caught CSV failures expose `context` and nested `span` details.

### `csv.read(text_or_bytes, {header: true, types: true})`

Use an options map when you want header rows and scalar type coercion:

```bash
./target/release/nodia eval '
use csv

val rows = csv.read("name,age,active\nAna,30,true", {
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
