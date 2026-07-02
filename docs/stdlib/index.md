# Standard Library

Nodia's standard library is intentionally small. Builtin names are short,
technical, and predictable — the simplicity is in syntax and canonical
formatting, not in overly humanized names.

Stdlib access is namespace-based through `use`:

```nodia
use text
use numbers
use json
use csv

emit text.upper("ana")
emit numbers.abs(-4)
val doc = json.parse(r'{"name":"Ana"}')
val table = csv.stringify([{name: "Ana"}], ["name"])
```

These are namespace calls, not methods on arbitrary values.
There is no implicit stdlib prelude.
`regex` is a language builtin, not a stdlib namespace, so it is used directly
as `regex { ... }` and `regex.find(...)`.

When you want only a few names in local scope, use `pick`:

```nodia
use numbers pick range
use conversion pick string

for i in range(3) {
  emit string(i)
}
```

`pick` without `as` imports only the selected stdlib names directly.
With `as`, the selection stays under the namespace alias.

Available stdlib namespaces:

* `text`
* `numbers`
* `conversion`
* `collections`
* `format`
* `io`
* `scan`
* `system`
* `datetime`

File-backed stdlib modules resolved through `use name`:

* `json`
* `csv`
* `http`
* `log`
* `test`

## Sections

* [Text](text.md) — case, normalization, unit-aware access, trim, split/join, lines, dedent, indent, byte/offset helpers, contains, starts, ends.
* [Numbers](numbers.md) — conversions, math, ranges.
* [Collections](collections.md) — `collections.len`, `collections.keys`,
  `collections.values`, `collections.entries`, `collections.get`,
  `collections.push`, `collections.pop`, `collections.first`,
  `collections.last`, `collections.slice`, `collections.reverse`,
  `collections.sort`, `collections.unique`.
* [Date & Time](datetime.md) — dates, datetimes, durations, parsing,
  formatting, epoch conversion, arithmetic.
* [Data](data.md) — `json.parse`, `json.stringify`, `csv.parse`,
  `csv.stringify`.
* [Format](format.md) — `format.format`, `format.pad`, `format.fixed`.
* [Scan](scan.md) — `scan.cursor`, prefix matching, explicit spans, scanner
  errors, staged parsing helpers.
* [System](system.md) — `system.args`, `system.env`, `system.exit`.
* [Conversion](conversion.md) — `conversion.string`, `conversion.bool`,
  `conversion.int`, `conversion.float`.
* [Regex](regex.md) — builtin `regex { ... }`, `regex.test`, `regex.find`,
  `regex.replace`, `regex.split`.
* [IO](io.md) — file, path, directory, glob, and stream builtins.

## Arity Checking

User functions and imported stdlib callables are checked for arity by
`nodia check`. A mismatch produces `E4107`:

```text
error[E4107]: text.upper() expects 1 argument(s), got 2
```
