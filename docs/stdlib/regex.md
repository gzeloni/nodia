# Regex Builtins

Import this namespace with `use re`.

These builtins accept a **pattern** that can be either a regex value
(produced by `regex { ... }`) or a plain string. When given a string, the
builtin treats it as raw regex text.

For DSL syntax, see [Regex DSL](../language/regex.md).

## `test(text, pattern)`

Returns `true` if the pattern matches **anywhere** in the text:

```bash
./target/release/nodia eval '
emit test("go to https://example.com now", regex {
  "https://"
  one_or_more letter
})
'
```

```text
true
```

## `full_match(text, pattern)`

Returns `true` only when the **entire** text matches:

```bash
./target/release/nodia eval '
emit full_match("abc-42", regex {
  start
  one_or_more letter
  "-"
  one_or_more digit
  end
})
'
```

```text
true
```

A pattern that already contains `start` / `end` and a `full_match` call are
redundant; pick one and stick with it for clarity.

## `find(text, pattern)`

Returns a structured match map for the first occurrence, or `null`.

```bash
./target/release/nodia eval '
val hit = find("go to https://example.com now", regex {
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
})

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
they line up with `len(string)` and `slice(...)`. See
[Text Positions](../reference/text-semantics.md) for the exact `0.6.x`
contract and current limitations.

## `find_all(text, pattern)`

Returns a list of all non-overlapping match maps:

```bash
./target/release/nodia eval '
emit len(find_all("http://a https://b", regex {
  either {
    branch { "http" }
    branch { "https" }
  }
  "://"
  one_or_more letter
}))
'
```

```text
2
```

Each element of the returned list has the same shape as `find(...)`. Negative
or empty results return `[]`.

## `replace(text, pattern, replacement)`

Replaces **all** matches. When `pattern` is a regex, the replacement string
supports placeholders:

| Placeholder    | Meaning              |
| -------------- | -------------------- |
| `$(0)`         | whole match          |
| `$(1)`, `$(2)` | indexed captures     |
| `$(name)`      | named capture        |
| `$$`           | literal `$`          |

```bash
./target/release/nodia eval '
emit replace("ana 42 bruno 77", regex { one_or_more digit }, "#")
'
```

```text
ana # bruno #
```

```bash
./target/release/nodia eval '
emit replace("https://example.com", regex {
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

When both the pattern and the replacement are source literals, `nodia check`
also validates malformed placeholder syntax and impossible capture references
before runtime.

## `replace_all(text, pattern, replacement)`

Explicit alias of `replace(...)`. Same behavior; the name documents intent.

## `split(text, pattern)`

Splits on every match. The pattern can be a literal string or a regex:

```bash
./target/release/nodia eval '
emit split("ana   bruno\tcarla", regex { one_or_more whitespace })
'
```

```text
["ana", "bruno", "carla"]
```

If the pattern can match empty text, `split` keeps the empty edge segments:

```bash
./target/release/nodia eval '
emit split("abc", "")
emit split("xay", regex { zero_or_more "a" })
'
```

```text
["", "a", "b", "c", ""]
["", "x", "y", ""]
```

## `split_regex(text, pattern)`

Explicit alias of `split(...)` when you want the regex intent to be obvious
at the call site.

## Patterns As Strings

Every regex builtin also accepts a plain string pattern, which is treated as
raw regex text:

```bash
./target/release/nodia eval '
emit full_match("abc-42", "^[a-z]+-\\d+$")
'
```

```text
true
```

The Nodia regex DSL is recommended for new code because it stays readable as
patterns grow.
