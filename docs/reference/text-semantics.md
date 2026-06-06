# Text Semantics

This page defines the official `0.7.4` text model in Nodia.

## Core Model

| Concept | Meaning | Public surface |
| --- | --- | --- |
| text | immutable UTF-8 string value | string literals, interpolation results, text IO, regex inputs |
| byte | one UTF-8 storage unit | `text.len(text, text.byte)`, `text.at(text, text.byte, ...)`, `io.read(stream, size)` byte budgets |
| bytes | explicit undecoded byte sequence value | `b"..."`, `text.encode(..., text.utf8)`, `io.read(..., io.bytes)`, `system.exec(...).stdout` |
| scalar value | one Unicode scalar value | `text.len(text, text.scalar)`, `text.at(text, text.scalar, ...)`, regex `start` / `end` |
| grapheme cluster | one extended grapheme cluster | `text.len(text, text.grapheme)`, `text.at(text, text.grapheme, ...)`, `text.slice(..., text.grapheme, ...)` |
| character | informal human term only | use `byte`, `scalar value`, or `grapheme cluster` when precision matters |

Nodia `0.7.4` keeps strings as UTF-8 text, and raw bytes have their own
first-class value kind instead of piggybacking on list syntax.

## Indexes And Offsets

Nodia keeps two distinct terms deliberately:

| Term | Meaning |
| --- | --- |
| index | selects one element in a unit sequence |
| offset | names a boundary between elements |

That means:

* `text.at(text, unit, index)` uses indexes.
* `text.slice(text, unit, start, end)`, regex `start` / `end`, and
  `text.offset(text, from_unit, to_unit, offset)` use offsets.
* Offsets may equal the length of the sequence because they name cut points,
  not elements.

## Official Rules

| Surface | Rule |
| --- | --- |
| `collections.len(string)` | legacy scalar count for strings |
| `collections.len(bytes)` | raw byte count |
| `string[index]` | legacy scalar indexing surface; zero-based; negative indexes count from the end; returns a one-scalar string |
| `bytes[index]` | direct byte indexing surface; zero-based; negative indexes count from the end; returns one `int` in `0..255` |
| `collections.get(string, index, default)` | legacy safe scalar indexing surface; negative indexes count from the end |
| `collections.get(bytes, index, default)` | legacy safe byte indexing surface; negative indexes count from the end |
| `collections.slice(string, start, end)` | legacy scalar slice; negative bounds count from the end; bounds are clamped |
| `collections.slice(bytes, start, end)` | legacy byte slice; negative bounds count from the end; bounds are clamped |
| `collections.contains(bytes, needle)` | checks byte membership when `needle` is `int`, or subsequence membership when `needle` is `bytes` |
| `text.len(text, text.byte)` | counts UTF-8 bytes |
| `text.len(text, text.scalar)` | counts Unicode scalar values |
| `text.len(text, text.grapheme)` | counts extended grapheme clusters |
| `text.at(text, text.byte, index)` | returns one raw UTF-8 byte as `int`; index must be non-negative and in range |
| `text.at(text, text.scalar, index)` | returns one Unicode scalar value as string; index must be non-negative and in range |
| `text.at(text, text.grapheme, index)` | returns one grapheme cluster as string; index must be non-negative and in range |
| `text.slice(text, text.byte, start, end)` | slices text by UTF-8 byte boundaries; offsets must be valid, in range, and ordered |
| `text.slice(text, text.scalar, start, end)` | slices text by scalar offsets; offsets must be in range and ordered |
| `text.slice(text, text.grapheme, start, end)` | slices text by grapheme offsets; offsets must be in range and ordered |
| `text.offset(text, from_unit, to_unit, offset)` | converts a valid boundary from byte, scalar, or grapheme units into another supported unit |
| `re.find(...).start` / `re.find(...).end` | scalar offsets aligned with `text.len(text, text.scalar)` and `text.slice(..., text.scalar, ...)` |
| `io.read(stream, size)` | `size` is a byte budget; returned text may read slightly past that budget to finish one UTF-8 scalar value |
| `text.encode(text, text.utf8)` | returns UTF-8 `bytes` |
| `text.decode(bytes, text.utf8)` | decodes bytes with codec `text.utf8` in strict mode; invalid sequences fail at runtime |
| `text.decode(bytes, text.utf8, text.lossy)` | decodes bytes with codec `text.utf8` in lossy mode, replacing invalid sequences |
| `text.normalize(text, text.lf)` | normalizes every line ending to `\n` |
| `text.normalize(text, text.crlf)` | normalizes every line ending to `\r\n` |
| `text.normalize(text, text.nfc)` / `text.normalize(text, text.nfd)` | canonical Unicode normalization helpers |
| `text.normalize(text, text.nfkc)` / `text.normalize(text, text.nfkd)` | compatibility Unicode normalization helpers |
| `text.strip_bom(text)` | removes one leading BOM when present |
| `text.drop_nul(text)` | removes every `U+0000` code point |
| `text.casefold(text)` | Unicode default case folding for explicit caseless operations |

## Boundaries

Three kinds of boundaries matter in `0.7.4`:

| Boundary kind | Valid range | Meaning |
| --- | --- | --- |
| scalar boundary | `0 .. text.len(text, text.scalar)` | any cut point between Unicode scalar values |
| byte boundary | `0 .. text.len(text, text.byte)` | any cut point that does not split one UTF-8 sequence |
| grapheme boundary | `0 .. text.len(text, text.grapheme)` | any cut point between extended grapheme clusters |

## Exact And Normalized Operations

Nodia does **not** normalize or case-fold text implicitly.

| Surface | Rule |
| --- | --- |
| `==` / `!=` on strings | compares the exact scalar sequence |
| `text.contains`, `text.starts`, `text.ends` on strings | check exact text, not canonical equivalence |
| `collections.sort(list)` on strings | sorts by exact scalar sequence |
| `collections.unique(list)` on strings | removes duplicates by exact scalar sequence |
| normalization-aware equality | normalize both sides explicitly, for example `text.normalize(a, text.nfc) == text.normalize(b, text.nfc)` |
| normalized/caseless equality | apply the chosen normalization and then `text.casefold(...)` |
| normalized ordering | compute an explicit key with `collections.sort_by(...)` |

This keeps string equality, ordering, regex inputs, and slice behavior
predictable. If a pipeline needs canonical or compatibility equivalence, that
step stays visible at the call site.

## Decoding And Sanitation

`0.7.4` keeps the decode boundary explicit:

| Situation | Rule |
| --- | --- |
| text readers such as `io.read(...)` / `io.readln(...)` | UTF-8 strict; invalid bytes fail with `E3000` |
| raw byte readers such as `io.read(..., io.bytes)` | return `bytes`; no decode happens |
| `system.exec(...)` output | returns raw bytes in `stdout` and `stderr` |
| invalid `text.decode(..., text.utf8)` input | fatal runtime error today; recoverable decode errors arrive later in `0.8.x` |
| lossy decode | only through explicit `text.decode(..., text.utf8, text.lossy)` |

This removes hidden lossy conversions from the runtime. If a script wants
replacement semantics, the lossy choice is now visible in source.

## Cross-Stdlib Adoption

The rest of the stdlib now follows the same rules where it matters:

| Surface | Rule |
| --- | --- |
| `json.read(text_or_bytes)` | accepts string or `bytes`; byte input is decoded with `text.decode(..., text.utf8)` before parsing |
| `csv.read(text_or_bytes, ...)` | accepts string or `bytes`; byte input is decoded with `text.decode(..., text.utf8)` before parsing |
| `format.format("%...s", ...)` | `%s` precision counts grapheme clusters, not scalar values |
| `format.pad(...)` | width counts grapheme clusters, so visible characters are not split mid-cluster |
| regex builtins | remain text-only; bytes must be decoded before matching |

## Worked Examples

Explicit byte/scalar conversion:

```bash
./target/release/nodia eval '
use text

val sample = "aéb"
emit text.len(sample, text.scalar)
emit text.len(sample, text.byte)
emit text.offset(sample, text.scalar, text.byte, 2)
emit text.offset(sample, text.byte, text.scalar, 3)
emit text.encode(sample, text.utf8)
emit text.decode(text.encode(sample, text.utf8), text.utf8)
'
```

```text
3
4
3
2
b"aéb"
aéb
```

Byte values are proper sequences too:

```bash
./target/release/nodia eval '
use collections as col

var raw = b"a\0\xffb"
emit raw[2]
emit col.get(raw, -1, null)
emit col.slice(raw, 1, 3)
raw[1] = 120
emit raw
'
```

```text
255
98
b"\0\xff"
b"ax\xffb"
```

Unit-aware access is explicit:

```bash
./target/release/nodia eval '
use text

val sample = "éx"
emit text.len(sample, text.scalar)
emit text.len(sample, text.grapheme)
emit text.at(sample, text.scalar, 1)
emit text.at(sample, text.grapheme, 0)
emit text.slice(sample, text.scalar, 0, 2)
emit text.slice(sample, text.grapheme, 0, 1)
emit text.slice("aéb", text.byte, 1, 3)
'
```

```text
3
2
́
é
é
é
é
```

Explicit lossy decode and sanitation:

```bash
./target/release/nodia eval '
use text

val raw = b"\xef\xbb\xbfa\r\nb\0\xff"
val decoded = text.decode(raw, text.utf8, text.lossy)
emit text.normalize(text.drop_nul(text.strip_bom(decoded)), text.lf)
'
```

```text
a
b�
```

Regex offsets remain scalar offsets:

```bash
./target/release/nodia eval '
use text
use re

val sample = "éx"
val hit = re.find(sample, regex { "x" })
emit hit.start
emit hit.end
emit text.slice(sample, text.scalar, 0, hit.start)
'
```

```text
2
3
é
```

Invalid unit boundaries are rejected explicitly:

```bash
./target/release/nodia eval 'use text
emit text.slice("é", text.byte, 1, 2)'
```

```text
error[E2000]: slice() byte offset 1 is not a UTF-8 boundary in text with 2 byte(s)
```

Invalid strict decoding is also rejected explicitly:

```bash
./target/release/nodia eval 'use text
emit text.decode(b"a\xffb", text.utf8)'
```

```text
error[E2000]: decode() cannot decode bytes as UTF-8: invalid utf-8 sequence of 1 bytes from index 1
```

Cross-stdlib byte adoption is explicit:

```bash
./target/release/nodia eval '
use text
use json
use csv

val doc = json.read(text.encode(r'{"name":"Ana","age":30}', text.utf8))
val rows = csv.read(text.encode("name,age\nAna,30", text.utf8), {
  header: true,
  types: true,
})
emit doc.name
emit rows[0].age + 5
'
```

```text
Ana
35
```

Grapheme-aware formatting also follows the new model:

```bash
./target/release/nodia eval '
use format
emit format.format("[%2s][%.1s]", ["é", "éx"])
emit format.pad("é", 2, format.left, ".")
'
```

```text
[ é][é]
.é
```

## Not In `0.7.4`

These areas are intentionally still out of scope in this release:

| Area | Status |
| --- | --- |
| grapheme-aware regex offsets | regex results still report scalar offsets |
| recoverable decode errors | not in the language yet; fatal/runtime behavior remains the only structured outcome before `0.8.x` |
