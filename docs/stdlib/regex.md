# Regex Builtins

`regex` is built into the language. Do not import it with `use`.

`test` and `find` accept a **pattern** that can be
either a regex value (produced by `regex { ... }`) or a plain string. A plain
string there is compiled as raw regex text.

If you want to normalize classic regex text into a first-class regex value,
use a bare raw string inside `regex { ... }`:

```nodia
val pat = regex {
  r"(?i)^\d{2}$"
}
```

That keeps `"..."` as literal text in the DSL, while `r"..."` becomes the
clean inverse path from classic regex syntax back into native regex nodes.
That inverse path now covers the broad engine surface directly, including
properties, hard anchors, quoted literals, mid-pattern flag toggles,
subroutines, absent operators, and backtracking verbs.

`replace` and `split` share the text-builtin surface: pass a regex value for
regex behavior, or a plain string for literal text behavior.

Regex execution works over the exact text you pass in. If a pipeline needs
canonical or compatibility equivalence across Unicode forms, normalize first
with `text.normalize(..., text.nfc)` or `text.normalize(..., text.nfkc)`.
Regex remains text-only even after the byte APIs in `0.7.x`: if your input
comes from `io.read(..., io.bytes)` or `system.exec(...)`, decode and sanitize it
explicitly before matching.

For DSL syntax, see [Regex DSL](../language/regex.md).

## `test(text, pattern)`

Returns `true` when the pattern matches **anywhere** in the text:

```bash
./target/release/nodia eval '
emit regex.test("go to https://example.com now", regex {
  "https://"
  one_or_more letter
}))
'
```

```text
true
```

## `test(text, pattern, regex.full)`

Returns `true` only when the **entire** text matches:

```bash
./target/release/nodia eval '
emit regex.test("abc-42", regex {
  start
  one_or_more letter
  "-"
  one_or_more digit
  end
}, regex.full))
'
```

```text
true
```

A pattern that already contains `start` / `end` and a `regex.full` match mode are
redundant; pick one and stick with it for clarity.

## `find(text, pattern)`

Returns a structured match map for the first occurrence, or `null`.

```bash
./target/release/nodia eval '
val hit = regex.find("go to https://example.com now", regex {
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
}))

emit hit.text
emit hit.start
emit hit.end
emit hit.named.scheme
emit hit.named.host
'
```

```text
https://example.com
6
25
https
example.com
```

### Match Shape

```nodia
{
  text: "https://example.com",
  start: 6,
  end: 25,
  groups: ["https", "example.com"],
  named: {
    scheme: "https",
    host: "example.com",
  },
}
```

`start` and `end` are **Unicode scalar value offsets** (not byte offsets), so
they line up with `collections.len(string)` and `collections.slice(...)`. Use
`text.slice(..., text.scalar, ...)` when you want to slice on the same unit,
or `text.offset(...)` when you need to cross the byte/scalar boundary. See
[Text Semantics](../reference/text-semantics.md) for the official `0.7.5`
model.

## `find(text, pattern, regex.all)`

Returns a list of all non-overlapping match maps:

```bash
./target/release/nodia eval '
use collections as col
emit col.len(regex.find("http://a https://b", regex {
  either {
    branch { "http" }
    branch { "https" }
  }
  "://"
  one_or_more letter
}, regex.all)))
'
```

```text
2
```

Each element of the returned list has the same shape as `find(...)`. Negative
or empty results return `[]`.

## `replace(text, pattern, replacement)`

Replaces **all** matches. When `pattern` is a regex value, the replacement
string supports placeholders:

| Placeholder    | Meaning              |
| -------------- | -------------------- |
| `$(0)`         | whole match          |
| `$(1)`, `$(2)` | indexed captures     |
| `$(name)`      | named capture        |
| `$$`           | literal `$`          |

```bash
./target/release/nodia eval '
emit regex.replace("ana 42 bruno 77", regex { one_or_more digit }, "#")
'
```

```text
ana # bruno #
```

```bash
./target/release/nodia eval '
emit regex.replace("https://example.com", regex {
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

A placeholder that names a missing capture is a runtime error.
If a declared capture does not participate in the branch that matched, its
placeholder expands to an empty string.

When the pattern is a regex value and the replacement is a source literal,
`nodia check` also validates malformed placeholder syntax and impossible
capture references before runtime. Plain-string patterns in `replace(...)`
stay literal text, so `$name` is just text there.

Zero-width regex matches also participate in replacement. That means anchors
can insert text without consuming characters:

```bash
./target/release/nodia eval '
emit regex.replace("abc", regex { start }, "<")
emit regex.replace("abc", regex { end }, ">")
'
```

```text
<abc
abc>
```

## `split(text, pattern)`

Splits on every match. The pattern can be a literal string or a regex:

```bash
./target/release/nodia eval '
emit regex.split("ana   bruno\tcarla", regex { one_or_more whitespace })
'
```

```text
["ana", "bruno", "carla"]
```

If the pattern can match empty text, `split` keeps the empty edge segments:

```bash
./target/release/nodia eval '
emit regex.split("abc", "")
emit regex.split("xay", regex { zero_or_more "a" })
'
```

```text
["", "a", "b", "c", ""]
["", "x", "y", ""]
```

## String Patterns In Matching Builtins

`test` and `find` accept plain string patterns,
which are treated as raw regex text:

```bash
./target/release/nodia eval '
emit regex.test("abc-42", "^[a-z]+-\\d+$", regex.full)
'
```

```text
true
```

`replace` and `split` do **not** compile plain strings as raw regex text. Pass
a regex value when you want regex mode there.

The Nodia regex DSL is recommended for new code because it stays readable as
patterns grow.
