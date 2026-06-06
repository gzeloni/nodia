# Text Semantics

This page defines the official `0.7.3` text model in Nodia.

## Core Model

| Concept | Meaning | Public surface |
| --- | --- | --- |
| text | immutable UTF-8 string value | string literals, interpolation results, text IO, regex inputs |
| byte | UTF-8 storage unit | `text.byte_len(text)`, `text.byte_slice(...)`, `io.read(stream, size)` byte budgets |
| byte sequence | explicit undecoded bytes represented as `list<int>` | `text.encode_utf8(...)`, `text.decode_utf8(...)`, `io.read_bytes(...)`, `system.exec(...).stdout` |
| scalar value | one Unicode scalar value | `collections.len(text)`, `text[index]`, `text.scalar(...)`, `text.scalar_slice(...)`, regex `start` / `end` |
| grapheme cluster | one extended grapheme cluster | `text.grapheme_len(text)`, `text.grapheme(...)`, `text.grapheme_slice(...)` |
| character | informal human term only | use `byte`, `scalar value`, or `grapheme cluster` when precision matters |

Nodia `0.7.3` keeps strings as UTF-8 text, but byte-oriented pipelines are now
explicit instead of smuggling lossy decoding through unrelated APIs.

## Indexes And Offsets

Nodia keeps two distinct terms deliberately:

| Term | Meaning |
| --- | --- |
| index | selects one element in a unit sequence |
| offset | names a boundary between elements |

That means:

* `text.scalar(text, i)` and `text.grapheme(text, i)` take indexes.
* `text.byte_slice(text, start, end)`, `text.scalar_slice(...)`,
  `text.grapheme_slice(...)`, regex `start` / `end`, `text.byte_offset(...)`,
  and `text.scalar_offset(...)` all use offsets.
* Offsets may equal the length of the sequence because they name cut points,
  not elements.

## Official Rules

| Surface | Rule |
| --- | --- |
| `collections.len(string)` | counts Unicode scalar values |
| `string[index]` | legacy scalar indexing surface; zero-based; negative indexes count from the end; returns a one-scalar string |
| `collections.get(string, index, default)` | legacy safe scalar indexing surface; negative indexes count from the end |
| `collections.slice(string, start, end)` | legacy scalar slice; negative bounds count from the end; bounds are clamped |
| `text.scalar(text, scalar_index)` | explicit scalar indexing; index must be non-negative and in range |
| `text.scalar_slice(text, start_scalar_offset, end_scalar_offset)` | explicit scalar slice; offsets must be non-negative, in range, and ordered |
| `text.byte_slice(text, start_byte_offset, end_byte_offset)` | explicit byte slice; offsets must be UTF-8 boundaries, in range, and ordered |
| `text.grapheme_len(text)` | counts extended grapheme clusters |
| `text.grapheme(text, grapheme_index)` | explicit grapheme indexing; index must be non-negative and in range |
| `text.grapheme_slice(text, start_grapheme_offset, end_grapheme_offset)` | explicit grapheme slice; offsets must be non-negative, in range, and ordered |
| `re.find(...).start` / `re.find(...).end` | scalar offsets aligned with `collections.len(string)` and `text.scalar_slice(...)` |
| `io.read(stream, size)` | `size` is a byte budget; the returned text may read slightly past that budget to finish one UTF-8 scalar value |
| `text.byte_len(text)` | returns the UTF-8 byte length |
| `text.byte_offset(text, scalar_offset)` | converts a scalar boundary into a UTF-8 byte offset |
| `text.scalar_offset(text, byte_offset)` | converts a UTF-8 byte boundary into a scalar offset; invalid boundaries fail at runtime |
| `text.encode_utf8(text)` | returns a UTF-8 byte sequence as `list<int>` |
| `text.decode_utf8(bytes)` | decodes `list<int>` as UTF-8; invalid bytes fail at runtime |
| `text.decode_utf8_lossy(bytes)` | decodes `list<int>` as UTF-8 with replacement characters for invalid sequences |
| `text.normalize_lf(text)` | normalizes every line ending to `\n` |
| `text.normalize_crlf(text)` | normalizes every line ending to `\r\n` |
| `text.strip_bom(text)` | removes one leading BOM when present |
| `text.drop_nul(text)` | removes every `U+0000` code point |
| `text.nfc(text)` / `text.nfd(text)` | canonical Unicode normalization helpers |
| `text.nfkc(text)` / `text.nfkd(text)` | compatibility Unicode normalization helpers |
| `text.casefold(text)` | Unicode default case folding for explicit caseless operations |

## Boundaries

Three kinds of boundaries matter in `0.7.3`:

| Boundary kind | Valid range | Meaning |
| --- | --- | --- |
| scalar boundary | `0 .. collections.len(text)` | any cut point between Unicode scalar values |
| byte boundary | `0 .. text.byte_len(text)` | any cut point that does not split one UTF-8 sequence |
| grapheme boundary | `0 .. text.grapheme_len(text)` | any cut point between extended grapheme clusters |

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

This keeps string equality, ordering, regex inputs, and slice behavior
predictable. If a pipeline needs canonical or compatibility equivalence, that
step stays visible at the call site.

## Decoding And Sanitation

`0.7.3` makes the decode boundary explicit:

| Situation | Rule |
| --- | --- |
| text readers such as `io.read(...)` / `io.readln(...)` | UTF-8 strict; invalid bytes fail with `E3000` |
| raw byte readers such as `io.read_bytes(...)` | return `list<int>`; no decode happens |
| `system.exec(...)` output | returns raw bytes in `stdout` and `stderr` |
| invalid `text.decode_utf8(...)` input | fatal runtime error today; recoverable decode errors arrive later in `0.8.x` |
| lossy decode | only through explicit `text.decode_utf8_lossy(...)` |

This removes hidden lossy conversions from the runtime. If a script wants
replacement semantics, the lossy choice is now visible in source.

## Worked Examples

Explicit byte/scalar conversion:

```bash
./target/release/nodia eval '
use text
use collections as col

val sample = "aéb"
emit col.len(sample)
emit text.byte_len(sample)
emit text.byte_offset(sample, 2)
emit text.scalar_offset(sample, 3)
emit text.encode_utf8(sample)
emit text.decode_utf8(text.encode_utf8(sample))
'
```

```text
3
4
3
2
[97, 195, 169, 98]
aéb
```

Unit-aware slicing is still explicit:

```bash
./target/release/nodia eval '
use text
use collections as col

val sample = "éx"
emit col.len(sample)
emit text.grapheme_len(sample)
emit text.scalar_slice(sample, 0, 2)
emit text.grapheme_slice(sample, 0, 1)
emit text.byte_slice("aéb", 1, 3)
'
```

```text
3
2
é
é
é
```

Explicit lossy decode and sanitation:

```bash
./target/release/nodia eval '
use text

val raw = [239, 187, 191, 97, 13, 10, 98, 0, 255]
val decoded = text.decode_utf8_lossy(raw)
emit text.normalize_lf(text.drop_nul(text.strip_bom(decoded)))
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
emit text.scalar_slice(sample, 0, hit.start)
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
emit text.byte_slice("é", 1, 2)'
```

```text
error[E2000]: byte_slice() byte offset 1 is not a UTF-8 boundary in text with 2 byte(s)
```

Invalid strict decoding is also rejected explicitly:

```bash
./target/release/nodia eval 'use text
emit text.decode_utf8([97, 255, 98])'
```

```text
error[E2000]: decode_utf8() cannot decode bytes as UTF-8: invalid utf-8 sequence of 1 bytes from index 1
```

## Not In `0.7.3`

These areas are intentionally still out of scope in this release:

| Area | Status |
| --- | --- |
| dedicated bytes type | still not in the language; byte sequences use `list<int>` for now |
| direct byte indexing | intentionally omitted until bytes have a first-class representation |
| grapheme-aware regex offsets | regex results still report scalar offsets |
| recoverable decode errors | not in the language yet; fatal/runtime behavior remains the only structured outcome before `0.8.x` |
