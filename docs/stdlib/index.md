# Standard Library

Nodia's standard library is intentionally small. Builtin names are short,
technical, and predictable — the simplicity is in syntax and canonical
formatting, not in overly humanized names.

All builtins are **functions**, not methods. The call style is always
`f(x, y)`, never `x.f(y)`.

## Sections

* [Text](text.md) — case, trim, split/join, lines, dedent, indent, contains, starts, ends.
* [Numbers](numbers.md) — conversions, math, ranges.
* [Collections](collections.md) — `len`, `keys`, `values`, `push`, `pop`,
  `first`, `last`, `slice`, `reverse`, `sort`, `unique`.
* [Conversion](conversion.md) — `string`, `bool`, `int`, `float`.
* [Regex](regex.md) — `test`, `full_match`, `find`, `find_all`, `replace`,
  `replace_all`, `split`, `split_regex`.
* [IO](io.md) — file and stream builtins.

## Legacy Aliases

These older names are still accepted but new code should use the canonical
form:

| Legacy        | Canonical |
| ------------- | --------- |
| `uppercase`   | `upper`   |
| `lowercase`   | `lower`   |
| `starts_with` | `starts`  |
| `ends_with`   | `ends`    |

## Arity Checking

User functions and known builtins are checked for arity by `nodia check`. A
mismatch produces `E4107`:

```text
error[E4107]: 'upper' expects 1 argument, got 2
```
