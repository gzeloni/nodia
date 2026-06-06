# Text Builtins

All text builtins are pure: they return new strings (or new lists for
`split`/`lines`/`words`), never mutate.

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

### `nfc(text)`

Canonical decomposition followed by canonical composition:

```bash
./target/release/nodia eval 'use text
emit text.nfc("é")'
```

```text
é
```

### `nfd(text)`

Canonical decomposition:

```bash
./target/release/nodia eval 'use text
emit text.nfd("é")'
```

```text
é
```

### `nfkc(text)`

Compatibility decomposition followed by canonical composition:

```bash
./target/release/nodia eval 'use text
emit text.nfkc("①")'
```

```text
1
```

## Bytes And Sanitation

Byte sequences are represented as `list<int>` where every element must be in
`0..255`.

### `encode_utf8(text)`

Encodes UTF-8 text into a byte list:

```bash
./target/release/nodia eval 'use text
emit text.encode_utf8("aéb")'
```

```text
[97, 195, 169, 98]
```

### `decode_utf8(bytes)`

Decodes a byte list as UTF-8. Invalid UTF-8 is a runtime error:

```bash
./target/release/nodia eval 'use text
emit text.decode_utf8([97, 195, 169, 98])'
```

```text
aéb
```

Use this with `io.read_bytes(...)` and `system.exec(...).stdout` /
`system.exec(...).stderr` when a pipeline must keep decoding explicit.

### `decode_utf8_lossy(bytes)`

Decodes a byte list as UTF-8, replacing invalid sequences with `�`:

```bash
./target/release/nodia eval 'use text
emit text.decode_utf8_lossy([97, 255, 98])'
```

```text
a�b
```

This is the only lossy UTF-8 decode surface in the language today.

### `normalize_lf(text)`

Normalizes `\r\n` and bare `\r` into `\n`:

```bash
./target/release/nodia eval 'use text
emit text.normalize_lf("a\r\nb\rc\n")'
```

```text
a
b
c
```

### `normalize_crlf(text)`

Normalizes every line ending into `\r\n`:

```bash
./target/release/nodia eval 'use text
emit text.normalize_crlf("a\r\nb\rc\n")'
```

The output contains CRLF line endings even when the source mixed styles.

### `strip_bom(text)`

Removes one leading Unicode BOM when present:

```bash
./target/release/nodia eval 'use text
emit text.strip_bom(text.decode_utf8([239, 187, 191, 104, 105]))'
```

```text
hi
```

### `drop_nul(text)`

Removes every `U+0000` code point:

```bash
./target/release/nodia eval 'use text
emit text.drop_nul(text.decode_utf8([97, 0, 98, 0]))'
```

```text
ab
```

### `nfkd(text)`

Compatibility decomposition:

```bash
./target/release/nodia eval 'use text
emit text.nfkd("①")'
```

```text
1
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

### `split_regex(text, pattern)`

Explicit regex-only alias of `split(...)`. Use when you want the regex intent
to be obvious at the call site.

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

### `replace_all(text, from, to)`

Explicit alias of `replace(...)`. The name makes the whole-text intent
obvious in scripts; the behavior is identical.

## Tests

### `contains(value, needle)`

* For strings: substring check.
  If `needle` is a regex, this becomes a regex match test.
* For lists: element membership.
* For maps: key presence.

```bash
./target/release/nodia eval '
use text
emit text.contains("adamantite", "mant")
emit text.contains("abc42def", regex { one_or_more digit })
emit text.contains(["compiler", "streams"], "streams")
emit text.contains({name: "Ana"}, "name")
'
```

```text
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
emit text.nfc(composed) == text.nfc(decomposed)
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
works on lists and maps — see [Collections](collections.md).

Use `text.grapheme_len(text)` when you need extended grapheme cluster count,
and `text.byte_len(text)` when you need UTF-8 storage length.

## Unit-Aware Access

These helpers are strict and explicit: the first argument must be a string, and
their indexes/offsets are non-negative.

### `scalar(text, scalar_index)`

Returns one Unicode scalar value as a string:

```bash
./target/release/nodia eval 'use text
emit text.scalar("nodia", 1)'
```

```text
o
```

### `grapheme_len(text)`

Counts extended grapheme clusters:

```bash
./target/release/nodia eval 'use text
emit text.grapheme_len("éx")'
```

```text
2
```

### `grapheme(text, grapheme_index)`

Returns one extended grapheme cluster as a string:

```bash
./target/release/nodia eval 'use text
emit text.grapheme("éx", 0)'
```

```text
é
```

### `byte_slice(text, start_byte_offset, end_byte_offset)`

Slices by explicit UTF-8 byte boundaries:

```bash
./target/release/nodia eval 'use text
emit text.byte_slice("aéb", 1, 3)'
```

```text
é
```

### `scalar_slice(text, start_scalar_offset, end_scalar_offset)`

Slices by explicit scalar offsets:

```bash
./target/release/nodia eval 'use text
emit text.scalar_slice("éx", 0, 2)'
```

```text
é
```

### `grapheme_slice(text, start_grapheme_offset, end_grapheme_offset)`

Slices by explicit grapheme-cluster offsets:

```bash
./target/release/nodia eval 'use text
emit text.grapheme_slice("éx", 0, 1)'
```

```text
é
```

These helpers reject invalid indexes, invalid boundaries, and reversed ranges.
For the legacy clamped scalar behavior, keep using `collections.get(...)` and
`collections.slice(...)`.

## Boundary Conversions

### `byte_len(text)`

Returns the UTF-8 byte length:

```bash
./target/release/nodia eval 'use text
emit text.byte_len("é")'
```

```text
2
```

### `byte_offset(text, scalar_offset)`

Converts a scalar offset into a byte offset:

```bash
./target/release/nodia eval 'use text
emit text.byte_offset("aéb", 2)'
```

```text
3
```

`scalar_offset` must be between `0` and `len(text)`.

### `scalar_offset(text, byte_offset)`

Converts a UTF-8 byte boundary into a scalar offset:

```bash
./target/release/nodia eval 'use text
emit text.scalar_offset("aéb", 3)'
```

```text
2
```

If `byte_offset` points into the middle of one UTF-8 sequence, this is a
runtime error.

See [Text Semantics](../reference/text-semantics.md) for the full `0.7.4`
model shared by string indexing, slicing, regex offsets, and chunked reads.
