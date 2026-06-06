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

## Text Semantics

These helpers make UTF-8 byte boundaries explicit. They are strict: the first
argument must be a string.

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

See [Text Semantics](../reference/text-semantics.md) for the full `0.7.1`
model shared by string indexing, slicing, regex offsets, and chunked reads.
