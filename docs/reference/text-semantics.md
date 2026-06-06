# Text Semantics

This page defines the official `0.7.1` text model in Nodia.

## Core Model

| Concept | Meaning | Public surface |
| --- | --- | --- |
| text | immutable UTF-8 string value | string literals, interpolation results, file reads, regex inputs |
| byte | UTF-8 storage unit | `text.byte_len(text)`, `io.read(stream, size)` byte budgets |
| scalar value | one Unicode scalar value | `collections.len(text)`, `text[index]`, `collections.slice(text, ...)`, regex `start` / `end` |
| character | informal human term only | use `byte` or `scalar value` when precision matters |

Nodia `0.7.1` does **not** redefine strings around grapheme clusters. When the
docs say a text position or offset without further qualification, it means a
**Unicode scalar value offset**.

## Official Rules

| Surface | Rule |
| --- | --- |
| `collections.len(string)` | counts Unicode scalar values |
| `string[index]` | zero-based scalar indexing; negative indexes count from the end; returns a one-scalar string |
| `collections.get(string, index, default)` | same scalar indexing rules as `string[index]`, but returns `default` instead of raising |
| `collections.slice(string, start, end)` | `start` inclusive, `end` exclusive; bounds are scalar offsets; negative bounds count from the end; bounds are clamped |
| `re.find(...).start` / `re.find(...).end` | scalar offsets aligned with `collections.len(string)` and `collections.slice(string, ...)` |
| `io.read(stream, size)` | `size` is a byte budget; the returned text may read slightly past that budget to finish one UTF-8 scalar value |
| `text.byte_len(text)` | returns the UTF-8 byte length |
| `text.byte_offset(text, scalar_offset)` | converts a scalar boundary into a UTF-8 byte offset |
| `text.scalar_offset(text, byte_offset)` | converts a UTF-8 byte boundary into a scalar offset; invalid boundaries fail at runtime |
| `text.nfc(text)` / `text.nfd(text)` | canonical Unicode normalization helpers |
| `text.nfkc(text)` / `text.nfkd(text)` | compatibility Unicode normalization helpers |
| `text.casefold(text)` | Unicode default case folding for explicit caseless operations |

## Boundaries

Two kinds of boundaries matter in `0.7.1`:

| Boundary kind | Valid range | Meaning |
| --- | --- | --- |
| scalar boundary | `0 .. collections.len(text)` | any cut point between Unicode scalar values |
| byte boundary | `0 .. text.byte_len(text)` | any cut point that does not split one UTF-8 sequence |

`text.byte_offset(text, scalar_offset)` accepts scalar boundaries.
`text.scalar_offset(text, byte_offset)` accepts byte boundaries and rejects
offsets that land in the middle of one UTF-8 sequence.

## Exact And Normalized Operations

Nodia does **not** normalize or case-fold text implicitly.

| Surface | Rule |
| --- | --- |
| `==` / `!=` on strings | compares the exact scalar sequence |
| `text.contains`, `text.starts`, `text.ends` on strings | check exact text, not canonical equivalence |
| `collections.sort(list)` on strings | sorts by exact scalar sequence |
| `collections.unique(list)` on strings | removes duplicates by exact scalar sequence |
| normalization-aware equality | normalize both sides explicitly, for example `text.nfc(a) == text.nfc(b)` |
| normalized/caseless equality | apply the chosen normalization and then `text.casefold(...)` |
| normalized ordering | compute an explicit key with `collections.sort_by(...)` |

This keeps string equality, ordering, and regex inputs predictable. If a text
pipeline needs canonical or compatibility equivalence, the normalization step
stays visible at the call site.

## Worked Examples

Explicit byte/scalar conversion:

```bash
./target/release/nodia eval '
use text
use collections as col

val text = "aéb"
emit col.len(text)
emit text.byte_len(text)
emit text.byte_offset(text, 2)
emit text.scalar_offset(text, 3)
'
```

```text
3
4
3
2
```

Decomposed text stays scalar-based in core string APIs and regex offsets:

```bash
./target/release/nodia eval '
use collections as col
use re

val text = "éx"
val hit = re.find(text, regex { "x" })
emit hit.start
emit hit.end
emit col.slice(text, 0, hit.start)
'
```

```text
2
3
é
```

Normalization-aware equality is explicit:

```bash
./target/release/nodia eval '
use text

val composed = "é"
val decomposed = "é"

emit composed == decomposed
emit text.nfc(composed) == text.nfc(decomposed)
emit text.casefold("Straße") == text.casefold("STRASSE")
'
```

```text
false
true
true
```

Normalization-aware ordering is also explicit:

```bash
./target/release/nodia eval '
use text
use collections

func key(value) {
  return text.casefold(text.nfc(value))
}

emit collections.sort(["Z", "é", "é"])
emit collections.sort_by(key, ["Z", "é", "é"])
'
```

```text
["Z", "é", "é"]
["Z", "é", "é"]
```

Invalid byte boundaries are rejected explicitly:

```bash
./target/release/nodia eval 'use text
emit text.scalar_offset("é", 1)'
```

```text
error[E2000]: scalar_offset() byte offset 1 does not point to a UTF-8 boundary
```

## Not In `0.7.1`

These areas are intentionally still out of scope in this release:

| Area | Status |
| --- | --- |
| grapheme-aware indexing/slicing | not in the public API yet |
| byte slicing/indexing | not in the public API yet |
| implicit lossy decoding | not allowed; text readers stay UTF-8 strict |
