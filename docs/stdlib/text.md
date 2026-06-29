# Text Builtins

All text builtins are pure: they return new strings, bytes, or new lists for
`split` / `lines` / `words`, never mutate.

Import this namespace with `use text`.

## Case

### `upper(text)`

```bash
./target/release/nodia eval 'use text
emit text.upper("nodia")'
```

```text
NODIA
```

### `lower(text)`

```bash
./target/release/nodia eval 'use text
emit text.lower("NODIA")'
```

```text
nodia
```

### `capitalize(text)`

Uppercases the first character and lowercases the rest:

```bash
./target/release/nodia eval 'use text
emit text.capitalize("gZELONI")'
```

```text
Gzeloni
```

### `casefold(text)`

Applies Unicode default case folding. Use this for explicit caseless
comparisons after choosing the normalization form you want:

```bash
./target/release/nodia eval 'use text
emit text.casefold("Straße")
emit text.casefold("STRASSE")'
```

```text
strasse
strasse
```

## Normalization

### `normalize(text, form)`

Applies an explicit normalization form. The second argument should be one of:

* `text.lf`
* `text.crlf`
* `text.nfc`
* `text.nfd`
* `text.nfkc`
* `text.nfkd`

Examples:

```bash
./target/release/nodia eval 'use text
emit text.normalize("é", text.nfc)
emit text.normalize("é", text.nfd)
emit text.normalize("①", text.nfkc)
emit text.normalize("a\r\nb\rc\n", text.lf)
emit text.normalize("a\r\nb\rc\n", text.crlf)'
```

```text
é
é
1
a
b
c

a\r\nb\r\nc\r\n
```

## Bytes And Sanitation

`bytes` is a first-class value kind for explicit undecoded data. Use `b"..."`
or `b'...'` when you need a literal byte buffer. These literals do not
interpolate. They support `\n`, `\r`, `\t`, `\0`, `\\`, `\"`, `\'`, and
`\xNN`. Non-ASCII characters inside a bytes literal are encoded as UTF-8
bytes, so use `\xNN` when you need an exact opaque byte sequence.

### `encode(text, codec)`

Encodes text into bytes. Today the supported codec is `text.utf8`:

```bash
./target/release/nodia eval 'use text
emit text.encode("aéb", text.utf8)'
```

```text
b"aéb"
```

### `decode(bytes, codec)` / `decode(bytes, codec, mode)`

Decodes bytes into text and returns a `result`. Today the supported codec is
`text.utf8`.
The optional third argument can be `text.strict` or `text.lossy`. Omitting it
defaults to strict decoding.

```bash
./target/release/nodia eval 'use text
use result
emit result.raise(text.decode(b"a\xc3\xa9b", text.utf8))
emit result.raise(text.decode(b"a\xffb", text.utf8, text.lossy))'
```

```text
aéb
a�b
```

Use this with `io.read(..., io.bytes)` and `system.exec(...).stdout` /
`system.exec(...).stderr` when a pipeline must keep decoding explicit.

### `strip_bom(text)`

Removes one leading Unicode BOM when present:

```bash
./target/release/nodia eval 'use text
use result
emit text.strip_bom(result.raise(text.decode(b"\xef\xbb\xbfhi", text.utf8)))'
```

```text
hi
```

### `drop_nul(text)`

Removes every `U+0000` code point:

```bash
./target/release/nodia eval 'use text
use result
emit text.drop_nul(result.raise(text.decode(b"a\0b\0", text.utf8)))'
```

```text
ab
```

## Whitespace

### `trim(text)`

Removes leading and trailing whitespace:

```bash
./target/release/nodia eval 'use text
emit "[{text.trim(\"  value  \")}]"'
```

```text
[value]
```

### `indent(text, spaces_or_prefix)`

When the second argument is an integer, prefixes that many spaces to every
line. When it is a string, uses that string as the per-line prefix.

```bash
./target/release/nodia eval 'use text
emit text.indent("a\nb", 2)'
```

```text
  a
  b
```

```bash
./target/release/nodia eval 'use text
emit text.indent("a\nb", "> ")'
```

```text
> a
> b
```

### `dedent(text)`

Removes the longest common leading-whitespace prefix from each line:

```bash
./target/release/nodia eval 'use text
val block = """
    a
    b
"""
emit text.dedent(block)'
```

```text

a
b
```

## Split & Join

### `split(text, sep)`

Splits on a literal separator string, or on a regex value:

```bash
./target/release/nodia eval 'use text
emit text.split("a,b,c", ",")'
```

```text
["a", "b", "c"]
```

```bash
./target/release/nodia eval 'use text
emit text.split("ana   bruno\tcarla", regex { one_or_more whitespace })'
```

```text
["ana", "bruno", "carla"]
```

If the separator is an empty string, or a regex that can match empty text,
`split` keeps the empty edge segments:

```bash
./target/release/nodia eval '
use text
emit text.split("abc", "")
emit text.split("xay", regex { zero_or_more "a" })
'
```

```text
["", "a", "b", "c", ""]
["", "x", "y", ""]
```

### `join(list, sep)`

Joins a list of values with a separator:

```bash
./target/release/nodia eval 'use text
emit text.join(["a", "b", "c"], "|")'
```

```text
a|b|c
```

### `lines(text)`

Splits on `\n`:

```bash
./target/release/nodia eval 'use text
emit text.lines("a\nb\nc")'
```

```text
["a", "b", "c"]
```

### `unlines(list)`

Inverse of `lines` — joins with `\n` between items:

```bash
./target/release/nodia eval 'use text
emit text.unlines(["a", "b", "c"])'
```

```text
a
b
c
```

### `words(text)`

Splits on runs of whitespace:

```bash
./target/release/nodia eval 'use text
emit text.words("terra blade true night edge")'
```

```text
["terra", "blade", "true", "night", "edge"]
```

## Substitution

### `replace(text, from, to)`

Replaces **all** occurrences. `from` can be a literal string or a regex:

```bash
./target/release/nodia eval 'use text
emit text.replace("a/b/c", "/", " -> ")'
```

```text
a -> b -> c
```

```bash
./target/release/nodia eval 'use text
emit text.replace("ana 42 bruno 77", regex { one_or_more digit }, "#")'
```

```text
ana # bruno #
```

When `from` is a regex, the replacement string supports Nodia-style
placeholders:

| Placeholder    | Meaning                |
| -------------- | ---------------------- |
| `$(0)`         | whole match            |
| `$(1)`, `$(2)` | indexed captures       |
| `$(name)`      | named capture          |
| `$$`           | literal `$`            |

If a named or indexed capture is declared in the pattern but does not match on
that branch, its placeholder expands to an empty string. Referring to a capture
name that was never declared is still an error.

```bash
./target/release/nodia eval '
use text
emit text.replace("https://example.com", regex {
  named scheme {
    either {
      branch { "http" }
      branch { "https" }
    }
  }
  "://"
  named host {
    one_or_more {
      char_set { letter digit "." "-" }
    }
  }
}, "<$(scheme):$(host)>")
'
```

```text
<https:example.com>
```

## Tests

### `contains(value, needle)`

* For strings: substring check.
  If `needle` is a regex, this becomes a regex match test.
* For bytes: byte membership when `needle` is an `int` in `0..255`,
  or subsequence membership when `needle` is `bytes`.
* For lists: element membership.
* For maps: key presence.

```bash
./target/release/nodia eval '
use text
emit text.contains("adamantite", "mant")
emit text.contains("abc42def", regex { one_or_more digit })
emit text.contains(b"abc42", 98)
emit text.contains(b"abc42", b"c4")
emit text.contains(["compiler", "streams"], "streams")
emit text.contains({name: "Ana"}, "name")
'
```

```text
true
true
true
true
true
true
```

## Comparison Contract

String comparison stays exact unless you normalize explicitly:

* `==` / `!=` compare the exact scalar sequence.
* `text.contains`, `text.starts`, and `text.ends` check exact text.
* `collections.sort` and `collections.unique` keep exact string semantics.

For canonical equivalence:

```bash
./target/release/nodia eval '
use text

val composed = "é"
val decomposed = "é"

emit composed == decomposed
emit text.normalize(composed, text.nfc) == text.normalize(decomposed, text.nfc)
emit text.casefold("Straße") == text.casefold("STRASSE")
'
```

```text
false
true
true
```

For normalization-aware order, compute a key explicitly:

```bash
./target/release/nodia eval '
use text
use collections

func key(value) {
  return text.casefold(text.normalize(value, text.nfc))
}

emit collections.sort(["Z", "é", "é"])
emit collections.sort_by(key, ["Z", "é", "é"])
'
```

```text
["Z", "é", "é"]
["Z", "é", "é"]
```

### `starts(text, prefix)` / `ends(text, suffix)`

For string inputs, `prefix` / `suffix` can be either a literal string or a
regex value:

```bash
./target/release/nodia eval '
use text
emit text.starts("adamantite", "ada")
emit text.ends("adamantite", "ite")
emit text.starts("42x", regex { one_or_more digit })
emit text.ends("x42", regex { one_or_more digit })
'
```

```text
true
true
true
true
```

## Length

`collections.len(text)` returns the Unicode scalar count for strings. It also
works on bytes, lists, and maps — see [Collections](collections.md).

Use `text.len(text, text.grapheme)` when you need extended grapheme cluster
count, and `text.len(text, text.byte)` when you need UTF-8 storage length.

## Unit-Aware Access

These helpers are strict and explicit: the first argument must be a string, and
their indexes/offsets are non-negative.

### `at(text, unit, index)`

Returns one element of the chosen text unit. `unit` may be `text.byte`,
`text.scalar`, or `text.grapheme`.

```bash
./target/release/nodia eval 'use text
emit text.at("nodia", text.scalar, 1)
emit text.at("éx", text.grapheme, 0)
emit text.at("é", text.byte, 0)'
```

```text
o
é
195
```

### `slice(text, unit, start, end)`

Slices text by explicit byte, scalar, or grapheme offsets:

```bash
./target/release/nodia eval 'use text
emit text.slice("aéb", text.byte, 1, 3)
emit text.slice("éx", text.scalar, 0, 2)
emit text.slice("éx", text.grapheme, 0, 1)'
```

```text
é
é
é
```

These helpers reject invalid indexes, invalid boundaries, and reversed ranges.
For the legacy clamped scalar behavior, keep using `collections.get(...)` and
`collections.slice(...)`.

## Boundary Conversions

### `len(text, unit)`

Counts the chosen unit explicitly:

```bash
./target/release/nodia eval 'use text
emit text.len("é", text.byte)
emit text.len("éx", text.scalar)
emit text.len("éx", text.grapheme)'
```

```text
2
3
2
```

### `offset(text, from_unit, to_unit, offset)`

Converts a boundary from one text unit into another:

```bash
./target/release/nodia eval 'use text
emit text.offset("aéb", text.scalar, text.byte, 2)
emit text.offset("aéb", text.byte, text.scalar, 3)
emit text.offset("éx", text.grapheme, text.byte, 1)'
```

```text
3
2
3
```

If the source offset points into the middle of one UTF-8 sequence, this is a
runtime error.

See [Text Semantics](../reference/text-semantics.md) for the full `0.7.5`
model shared by string indexing, slicing, regex offsets, and chunked reads.
