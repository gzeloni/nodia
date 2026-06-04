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

* [Text](text.md) — case, trim, split/join, lines, dedent, indent, contains, starts, ends.
* [Numbers](numbers.md) — conversions, math, ranges.
* [Collections](collections.md) — `len`, `keys`, `values`, `entries`, `get`,
  `push`, `pop`, `first`, `last`, `slice`, `reverse`, `sort`, `unique`.
* [Date & Time](datetime.md) — dates, datetimes, durations, parsing,
  formatting, epoch conversion, arithmetic.
* [Data](data.md) — `use json`, `use csv`.
* [Format](format.md) — `format`, `pad_left`, `pad_right`, `fixed`.
* [System](system.md) — `args`, `env`, `exit`.
* [Conversion](conversion.md) — `string`, `bool`, `int`, `float`.
* [Regex](regex.md) — `test`, `full_match`, `find`, `find_all`, `replace`,
  `replace_all`, `split`, `split_regex`.
* [IO](io.md) — file, path, directory, glob, and stream builtins.

## Arity Checking

User functions and imported stdlib callables are checked for arity by
`nodia check`. A mismatch produces `E4107`:

```text
error[E4107]: 'upper' expects 1 argument, got 2
```
