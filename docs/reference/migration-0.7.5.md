# Migration To 0.7.5

`0.7.5` closes the `0.7.x` text-semantics line. This release does not add a
new conceptual model; it freezes the current one, removes stale naming from
the public docs, and makes migration targets explicit.

## What Stabilized

The stable surface for text pipelines is now:

* namespace-based stdlib access through `use`
* explicit `bytes` values instead of `list<int>` byte hacks
* explicit codec selection through `text.encode(...)` / `text.decode(...)`
* explicit unit selection through `text.byte`, `text.scalar`, and
  `text.grapheme`
* explicit normalization and lossy decode choices at the call site

There is no compatibility prelude and no legacy alias layer for removed names.

If you are already on `0.8.1+`, remember that `text.decode(...)`,
`json.read(...)`, `csv.read(...)`, `datetime.parse(...)`, `io.*`, `regex.test(...)`,
and `regex.find(...)` now return `result`. Use `result.raise(...)`,
`result.then(...)`, `result.recover(...)`, or `result.value_or(...)` at the
call site instead of accessing the returned value directly.

## Old To New Names

| Older shape | `0.7.5` shape |
| --- | --- |
| `regex.full_match(text, pattern)` | `regex.test(text, pattern, regex.full)` |
| `text.replace_all(text, from, to)` | `text.replace(text, from, to)` |
| `text.split_regex(text, pattern)` | `text.split(text, pattern)` |
| `text.nfc(text)` | `text.normalize(text, text.nfc)` |
| `text.nfd(text)` | `text.normalize(text, text.nfd)` |
| `text.nfkc(text)` | `text.normalize(text, text.nfkc)` |
| `text.nfkd(text)` | `text.normalize(text, text.nfkd)` |
| `text.byte_len(text)` | `text.len(text, text.byte)` |
| `text.grapheme_len(text)` | `text.len(text, text.grapheme)` |
| `text.scalar(text, index)` | `text.at(text, text.scalar, index)` |
| `text.grapheme(text, index)` | `text.at(text, text.grapheme, index)` |
| `text.byte_slice(text, start, end)` | `text.slice(text, text.byte, start, end)` |
| `text.scalar_slice(text, start, end)` | `text.slice(text, text.scalar, start, end)` |
| `text.grapheme_slice(text, start, end)` | `text.slice(text, text.grapheme, start, end)` |
| `text.byte_offset(text, scalar_offset)` | `text.offset(text, text.scalar, text.byte, scalar_offset)` |
| `text.scalar_offset(text, byte_offset)` | `text.offset(text, text.byte, text.scalar, byte_offset)` |

## Bytes Migration

Before `0.7.x`, raw undecoded data tended to leak through string-only APIs or
through ad-hoc list conventions. In `0.7.5`, raw byte flow is explicit:

* produce bytes with `b"..."` or `text.encode(...)`
* read bytes with `io.read(..., io.bytes)`
* decode bytes with `text.decode(..., text.utf8)` or
  `text.decode(..., text.utf8, text.lossy)`
* hand bytes directly to `json.read(...)`, `csv.read(...)`, file writes, or
  inspect them through `bytes[index]`

If a pipeline may contain malformed UTF-8, decode lossily on purpose. Do not
expect `io.read(...)` or `json.read(bytes)` to silently replace invalid input.

## Compatibility Surfaces That Remain

Some older collection-style text surfaces are still present intentionally:

* `collections.len(string)` still counts scalar values
* `string[index]` still performs scalar indexing
* `collections.get(string, index, default)` and `collections.slice(string, ...)`
  still use the older scalar/clamped behavior
* `collections.slice(bytes, ...)` still provides the older clamped byte slice

These remain for now because they are established collection operations. New
text-sensitive code should prefer `text.len`, `text.at`, `text.slice`, and
`text.offset` when boundary semantics matter.

## `use` Guidance

Prefer namespace imports for larger modules:

```nodia
use text
use json
use result

emit text.normalize("é", text.nfc)
emit result.raise(json.read(r'{"name":"Ana"}')).name
```

Use `pick` only when it actually improves the local code:

```nodia
use numbers pick range
use conversion pick string

for i in range(3) {
  emit string(i)
}
```

Without `as`, stdlib `pick` imports direct names into local scope. With `as`,
the selected names stay under the alias namespace.

## Checker And Diagnostics

The checker now reports surface call names, not hidden internal names. For
example, `text.upper("a", "b")` reports:

```text
error[E4107]: text.upper() expects 1 argument(s), got 2
```

That same rule applies to aliases such as `txt.upper(...)`.
