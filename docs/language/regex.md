# Regex DSL

Nodia v0.6 introduces a native, readable regex DSL. A `regex { ... }` block
evaluates to a first-class **regex value**. When emitted, interpolated, or
converted with `string(...)`, it renders to classic regex text. When used
with the regex builtins (`test`, `find`, `find_all`, `full_match`, `replace`,
`split`), it executes against text.

## A First Pattern

```bash
./target/release/nodia eval '
val date = regex(case_insensitive) {
  start
  named year {
    exactly 4 digit
  }
  "-"
  exactly 2 digit
  "-"
  exactly 2 digit
  end
}
emit date
'
```

```text
(?i)^(?<year>\d{4})-\d{2}-\d{2}$
```

The block is a sequence of regex nodes. Adjacent nodes concatenate. Strings
inside the block are literal matches (escaped as needed).

## Top-Level Flags

```nodia
regex(case_insensitive, multiline) {
  ...
}
```

Available flags:

| Flag                 | Classic equivalent | Meaning                                                      |
| -------------------- | ------------------ | ------------------------------------------------------------ |
| `case_insensitive`   | `(?i)`             | case-insensitive matching                                    |
| `multiline`          | `(?m)`             | `start` / `end` match line boundaries, not just text boundaries |
| `dot_all`            | `(?s)`             | `any_codepoint` matches newline                              |
| `unicode`            | `(?u)`             | unicode-aware character classes                              |
| `ignore_whitespace`  | `(?x)`             | ignore whitespace in the rendered pattern                    |
| `ungreedy`           | `(?U)`             | invert greediness of quantifiers                             |

## Anchors

| Token              | Renders as | Meaning                                  |
| ------------------ | ---------- | ---------------------------------------- |
| `start`            | `^`        | start of input (or line with multiline)  |
| `end`              | `$`        | end of input (or line with multiline)    |
| `word_boundary`    | `\b`       | word boundary                            |
| `not_word_boundary`| `\B`       | non-word boundary                        |

```bash
./target/release/nodia eval '
val w = regex {
  word_boundary
  one_or_more letter
  word_boundary
}
emit w
'
```

```text
\b[A-Za-z]+\b
```

## Character Classes

Built-in classes (use them as bare tokens):

| Token            | Classic           | Meaning                          |
| ---------------- | ----------------- | -------------------------------- |
| `digit`          | `\d`              | digit                            |
| `not_digit`      | `\D`              | non-digit                        |
| `whitespace`     | `\s`              | whitespace                       |
| `not_whitespace` | `\S`              | non-whitespace                   |
| `word_char`      | `\w`              | word character                   |
| `not_word_char`  | `\W`              | non-word character               |
| `letter`         | `[A-Za-z]`        | ASCII letter                     |
| `lowercase`      | `[a-z]`           | ASCII lowercase letter           |
| `uppercase`      | `[A-Z]`           | ASCII uppercase letter           |
| `hex_digit`      | `[0-9A-Fa-f]`     | hex digit                        |
| `alnum`          | `[A-Za-z0-9]`     | ASCII alphanumeric               |
| `space`          | ` ` (literal space) | a literal space character      |
| `tab`            | `\t`              | tab                              |
| `newline`        | `\n`              | newline                          |
| `any_char`       | `.`               | any character except newline     |
| `any_codepoint`  | `[\s\S]`          | any character including newline  |

```bash
./target/release/nodia eval '
val d = regex { one_or_more digit }
emit d
'
```

```text
\d+
```

## Literal Helpers

| Form               | Meaning                                              |
| ------------------ | ---------------------------------------------------- |
| `"text"` (bare)    | literal string (escaped as needed)                   |
| `literal("text")`  | explicit literal helper                              |
| `char("x")`        | explicit single character                            |
| `raw_regex "..."`  | raw escape hatch — embedded directly into output     |

```bash
./target/release/nodia eval '
val p = regex {
  literal("a.b")
  raw_regex "\\d+"
}
emit p
'
```

```text
a\.b\d+
```

`literal("a.b")` escapes the `.`; `raw_regex` is inserted verbatim and is the
right tool when you really need a snippet of upstream regex.

## Quantifiers

Quantifiers can wrap either a single node or a `{ ... }` block.

| Form                 | Classic    | Meaning                          |
| -------------------- | ---------- | -------------------------------- |
| `optional X`         | `X?`       | zero or one                      |
| `zero_or_more X`     | `X*`       | zero or more                     |
| `one_or_more X`      | `X+`       | one or more                      |
| `exactly N X`        | `X{N}`     | exactly N                        |
| `at_least N X`       | `X{N,}`    | at least N                       |
| `between N and M X`  | `X{N,M}`   | between N and M (inclusive)      |

`X` can be a single token or a block:

```bash
./target/release/nodia eval '
val phone = regex {
  start
  "("
  exactly 3 digit
  ") "
  exactly 3 digit
  "-"
  exactly 4 digit
  end
}
emit phone
emit test("(415) 555-1234", phone)
'
```

```text
^\(\d{3}\) \d{3}-\d{4}$
true
```

`between` requires the keyword `and` between the bounds:

```bash
./target/release/nodia eval '
val hex = regex {
  "#"
  between 3 and 8 hex_digit
}
emit hex
'
```

```text
#[0-9A-Fa-f]{3,8}
```

## Groups

| Form                      | Classic                | Meaning                              |
| ------------------------- | ---------------------- | ------------------------------------ |
| `group { ... }`           | `( ... )`              | capture group                        |
| `capture { ... }`         | `( ... )`              | alias of `group`                     |
| `non_capture { ... }`     | `(?: ... )`            | non-capturing group                  |
| `named NAME { ... }`      | `(?<NAME> ... )`       | named capture                        |
| `atomic { ... }`          | `(?> ... )`            | atomic group (no backtracking)       |

`NAME` is parsed contextually here, so reserved words such as `val` or `from`
are allowed as group names.

```bash
./target/release/nodia eval '
val p = regex {
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
}
emit p
'
```

```text
(?<scheme>(?:http|https))://(?<host>(?:[A-Za-z0-9.\-])+)
```

## Alternation: `either` / `branch`

`either` introduces an alternation; each `branch` is one option. `branch` is
only valid inside `either`:

```bash
./target/release/nodia eval '
val p = regex {
  either {
    branch { "yes" }
    branch { "no" }
    branch { "maybe" }
  }
}
emit p
emit find_all("yes/maybe/no", p)
'
```

```text
(?:yes|no|maybe)
[{end: 3, groups: [], named: {}, start: 0, text: yes}, {end: 9, groups: [], named: {}, start: 4, text: maybe}, {end: 12, groups: [], named: {}, start: 10, text: no}]
```

## Character Sets

`char_set` and `not_char_set` accept entries that can be:

* bare class tokens (`letter`, `digit`, `whitespace`, ...)
* one-character string literals (e.g. `"."`, `"-"`)
* `char("x")` — explicit single character
* `range "a" to "z"` — character range
* `raw_regex "..."` — raw insert into the character class

```bash
./target/release/nodia eval '
val ident = regex {
  char_set { letter "_" }
  zero_or_more {
    char_set { letter digit "_" }
  }
}
emit ident
'
```

```text
[A-Za-z_](?:[A-Za-z0-9_])*
```

Negation:

```bash
./target/release/nodia eval '
val p = regex {
  one_or_more {
    not_char_set { whitespace "," }
  }
}
emit p
emit find_all("a, b , c", p)
'
```

```text
(?:[^\s,])+
[{end: 1, groups: [], named: {}, start: 0, text: a}, {end: 4, groups: [], named: {}, start: 3, text: b}, {end: 8, groups: [], named: {}, start: 7, text: c}]
```

Character ranges use `range "a" to "z"` (not `range "a" "z"`):

```bash
./target/release/nodia eval '
val p = regex {
  one_or_more {
    char_set { range "0" to "9" }
  }
}
emit p
emit find_all("ab12cd34", p)
'
```

```text
(?:[0-9])+
[{end: 4, groups: [], named: {}, start: 2, text: 12}, {end: 8, groups: [], named: {}, start: 6, text: 34}]
```

## Lookarounds

| Form                            | Classic           |
| ------------------------------- | ----------------- |
| `followed_by { ... }`           | `(?= ... )`       |
| `not_followed_by { ... }`       | `(?! ... )`       |
| `preceded_by { ... }`           | `(?<= ... )`      |
| `not_preceded_by { ... }`       | `(?<! ... )`      |

Lookarounds always take a block:

```bash
./target/release/nodia eval '
val p = regex {
  one_or_more digit
  followed_by { "px" }
}
emit p
emit find_all("12px 7em 3px 99", p)
'
```

```text
\d+(?=px)
[{end: 2, groups: [], named: {}, start: 0, text: 12}, {end: 10, groups: [], named: {}, start: 9, text: 3}]
```

## Backreferences

| Form                 | Classic     | Meaning                                  |
| -------------------- | ----------- | ---------------------------------------- |
| `same_as NAME`       | `\k<NAME>`  | refer to a named capture                 |
| `same_as_group N`    | `\N`        | refer to an indexed capture (1-based)    |

```bash
./target/release/nodia eval '
val dup = regex {
  named word { one_or_more letter }
  whitespace
  same_as word
}
emit dup
emit test("the the cat", dup)
emit test("the cat", dup)
'
```

```text
(?<word>[A-Za-z]+)\s\k<word>
true
false
```

## Scoped Flags

Toggle flags inside a region only:

| Form                              | Classic        |
| --------------------------------- | -------------- |
| `with_flags(flag1, flag2) { ... }` | `(?flags: ... )`|
| `without_flags(flag1, flag2) { ... }` | `(?-flags: ... )` |

```bash
./target/release/nodia eval '
val p = regex {
  with_flags(case_insensitive) {
    "abc"
  }
  "def"
}
emit p
emit test("ABCdef", p)
emit test("ABCDEF", p)
'
```

```text
(?i:abc)def
true
false
```

## Executing A Regex

The full set of regex builtins is documented in
[Standard Library / Regex](../stdlib/regex.md). The most common ones are:

* `test(text, pattern)` — does the pattern match anywhere?
* `full_match(text, pattern)` — does the whole text match?
* `find(text, pattern)` — first match as a structured map, or `null`.
* `find_all(text, pattern)` — list of all non-overlapping matches.
* `replace(text, pattern, replacement)` — replace matches; supports
  `$(0)`, `$(1)`, `$(name)`, `$$` placeholders.
* `split(text, pattern)` — split on matches.

Patterns may be regex values **or** plain strings. When a builtin gets a
string, it treats it as raw regex text.

## Match Shape

A `find(...)` hit is a map:

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

`start` and `end` are **character offsets**, so they line up with Nodia
string indexing and `slice(...)`.

## Replacement Placeholders

When `replace(...)` receives a regex pattern, the replacement string supports
Nodia-style placeholders:

| Placeholder   | Meaning                                   |
| ------------- | ----------------------------------------- |
| `$(0)`        | whole match                               |
| `$(1)`, `$(2)` | indexed captures                         |
| `$(name)`     | named capture                             |
| `$$`          | literal `$`                               |

If a declared capture does not participate in the branch that matched, its
placeholder expands to an empty string. Referring to a capture name that does
not exist in the pattern is an error.

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

A placeholder that refers to a missing capture is a runtime error.

## Why A DSL?

* **Readable**: regex blocks look like outlines, not punctuation soup.
* **Composable**: pieces are nameable identifiers, not magic glyphs.
* **Statically inspectable**: `nodia check` validates regex DSL structure
  before execution.
* **Same engine, same semantics**: `regex { ... }` compiles to a normal
  regex string that the runtime executes via `fancy-regex` — the same engine
  that handles raw-string regex patterns.
