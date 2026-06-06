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
val doc = json.read(r'{"name":"Ana"}')
val text = csv.write([{name: "Ana"}])
```

These are namespace calls, not methods on arbitrary values.
There is no implicit stdlib prelude.

Available stdlib namespaces:

* `text`
* `numbers`
* `conversion`
* `collections`
* `format`
* `re`
* `io`
* `system`
* `datetime`
* `json`
* `csv`

## Sections

* [Text](text.md) — case, trim, split/join, lines, dedent, indent, byte/offset helpers, contains, starts, ends.
* [Numbers](numbers.md) — conversions, math, ranges.
* [Collections](collections.md) — `collections.len`, `collections.keys`,
  `collections.values`, `collections.entries`, `collections.get`,
  `collections.push`, `collections.pop`, `collections.first`,
  `collections.last`, `collections.slice`, `collections.reverse`,
  `collections.sort`, `collections.unique`.
* [Date & Time](datetime.md) — dates, datetimes, durations, parsing,
  formatting, epoch conversion, arithmetic.
* [Data](data.md) — `use json`, `use csv`.
* [Format](format.md) — `format.format`, `format.pad_left`,
  `format.pad_right`, `format.fixed`.
* [System](system.md) — `system.args`, `system.env`, `system.exit`.
* [Conversion](conversion.md) — `conversion.string`, `conversion.bool`,
  `conversion.int`, `conversion.float`.
* [Regex](regex.md) — `re.test`, `re.full_match`, `re.find`,
  `re.find_all`, `re.replace`, `re.replace_all`, `re.split`,
  `re.split_regex`.
* [IO](io.md) — file, path, directory, glob, and stream builtins.

## Arity Checking

User functions and imported stdlib callables are checked for arity by
`nodia check`. A mismatch produces `E4107`:

```text
error[E4107]: 'upper' expects 1 argument, got 2
```
