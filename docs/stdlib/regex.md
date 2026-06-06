# Regex Builtins

Import this namespace with `use re`.

`test`, `full_match`, `find`, and `find_all` accept a **pattern** that can be
either a regex value (produced by `regex { ... }`) or a plain string. A plain
string there is compiled as raw regex text.

`replace`, `replace_all`, `split`, and `split_regex` share the text-builtin
surface: pass a regex value for regex behavior, or a plain string for literal
text behavior.

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

When the pattern is a regex value and the replacement is a source literal,
`nodia check` also validates malformed placeholder syntax and impossible
capture references before runtime. Plain-string patterns in `replace(...)`
stay literal text, so `$name` is just text there.

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

## String Patterns In Matching Builtins

`test`, `full_match`, `find`, and `find_all` accept plain string patterns,
which are treated as raw regex text:

```bash
./target/release/nodia eval '
emit full_match("abc-42", "^[a-z]+-\\d+$")
'
```

```text
true
```

`replace`, `replace_all`, `split`, and `split_regex` do **not** compile plain
strings as raw regex text. Pass a regex value when you want regex mode there.

The Nodia regex DSL is recommended for new code because it stays readable as
patterns grow.
