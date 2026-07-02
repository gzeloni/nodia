# Regex DSL

Nodia has a native, readable regex DSL. A `regex { ... }` block
evaluates to a first-class **regex value**. When emitted, interpolated, or
converted with `conversion.string(...)`, it renders to classic regex text. When used
with the regex builtins (`test`, `find`, `replace`,
`split`), it executes against text.
The `regex` keyword also owns the runtime surface directly, so
`regex.find(...)` and friends need no `use`.

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
| `crlf`               | `(?R)`             | treat CRLF as a newline pair for line anchors and dot rules  |
| `dot_all`            | `(?s)`             | `any_codepoint` matches newline                              |
| `unicode`            | `(?u)`             | unicode-aware character classes                              |
| `ignore_whitespace`  | `(?x)`             | ignore whitespace in the rendered pattern                    |
| `ungreedy`           | `(?U)`             | invert greediness of quantifiers                             |

## Anchors

| Token              | Renders as | Meaning                                  |
| ------------------ | ---------- | ---------------------------------------- |
| `start`            | `^`        | start of input (or line with multiline)  |
| `end`              | `$`        | end of input (or line with multiline)    |
| `start_text`       | `\A`       | hard start of input                      |
| `end_text`         | `\z`       | hard end of input                        |
| `end_text_before_newlines` | `\Z` | end of input, ignoring trailing newlines |
| `left_word_boundary` | `\b{start}` | start-side word boundary               |
| `left_word_half_boundary` | `\b{start-half}` | start-side half boundary     |
| `right_word_boundary` | `\b{end}` | end-side word boundary                  |
| `right_word_half_boundary` | `\b{end-half}` | end-side half boundary       |
| `word_boundary`    | `\b`       | word boundary                            |
| `not_word_boundary`| `\B`       | non-word boundary                        |
| `previous_match_end` | `\G`     | resume from the previous match end       |
| `keep_out`         | `\K`       | drop everything matched so far from group 0 |

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
| `not_hex_digit`  | `\H`              | non-hex digit                    |
| `not_newline`    | `\N`              | any character except newline     |
| `general_newline`| `\R`              | newline sequence (`\r\n`, `\n`, `\r`, ...) |
| `letter`         | `[A-Za-z]`        | ASCII letter                     |
| `lowercase`      | `[a-z]`           | ASCII lowercase letter           |
| `uppercase`      | `[A-Z]`           | ASCII uppercase letter           |
| `hex_digit`      | `[0-9A-Fa-f]`     | hex digit                        |
| `alnum`          | `[A-Za-z0-9]`     | ASCII alphanumeric               |
| `bell`           | `\a`              | bell control character           |
| `escape`         | `\e`              | escape control character         |
| `form_feed`      | `\f`              | form feed                        |
| `space`          | ` ` (literal space) | a literal space character      |
| `tab`            | `\t`              | tab                              |
| `newline`        | `\n`              | newline                          |
| `carriage_return`| `\r`              | carriage return                  |
| `vertical_tab`   | `\v`              | vertical tab                     |
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
| `r"..."` (bare)    | parse classic regex text back into native DSL nodes  |
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

`literal("a.b")` escapes the `.`. A bare raw string inside `regex { ... }`
parses classic regex text back into the native AST, so this:

```nodia
val pat = regex {
  r"(?i)^\d{2}$"
}
```

formats canonically as:

```nodia
val pat = regex(case_insensitive) {
  start
  exactly 2 digit
  end
}
```

`raw_regex` stays as the opaque escape hatch when you really need a snippet to
pass through unchanged instead of normalizing it into the DSL.

The inverse raw-regex path now understands more of the engine surface directly:
properties (`\p{...}` / `\P{...}`), hard anchors (`\A`, `\z`, `\Z`), quoted
literals (`\Q...\E`), mid-pattern flag toggles, subroutine calls, absent
operators, and backtracking verbs.

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
emit regex.test("(415) 555-1234", phone)
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
emit regex.find("yes/maybe/no", p, regex.all)
'
```

```text
(?:yes|no|maybe)
[{end: 3, groups: [], named: {}, start: 0, text: "yes"}, {end: 9, groups: [], named: {}, start: 4, text: "maybe"}, {end: 12, groups: [], named: {}, start: 10, text: "no"}]
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
emit regex.find("a, b , c", p, regex.all)
'
```

```text
(?:[^\s,])+
[{end: 1, groups: [], named: {}, start: 0, text: "a"}, {end: 4, groups: [], named: {}, start: 3, text: "b"}, {end: 8, groups: [], named: {}, start: 7, text: "c"}]
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
emit regex.find("ab12cd34", p, regex.all)
'
```

```text
(?:[0-9])+
[{end: 4, groups: [], named: {}, start: 2, text: "12"}, {end: 8, groups: [], named: {}, start: 6, text: "34"}]
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
emit regex.find("12px 7em 3px 99", p, regex.all)
'
```

```text
\d+(?=px)
[{end: 2, groups: [], named: {}, start: 0, text: "12"}, {end: 10, groups: [], named: {}, start: 9, text: "3"}]
```

## Backreferences

| Form                 | Classic     | Meaning                                  |
| -------------------- | ----------- | ---------------------------------------- |
| `same_as NAME`       | `\k<NAME>`  | refer to a named capture                 |
| `same_as_group N`    | `\1`        | refer to an indexed capture (1-based)    |
| `call NAME`          | `\g<NAME>`  | call a named capture as a subroutine     |
| `call_group N`       | `\g<N>`     | call an indexed capture as a subroutine  |

```bash
./target/release/nodia eval '
val dup = regex {
  named word { one_or_more letter }
  whitespace
  same_as word
}
emit dup
emit regex.test("the the cat", dup)
emit regex.test("the cat", dup)
'
```

```text
(?<word>[A-Za-z]+)\s\k<word>
true
false
```

## Conditionals

Conditional branches can depend on whether a capture participated, or on a
lookaround assertion:

| Form | Meaning |
| --- | --- |
| `if_capture NAME then { ... } else { ... }` | branch on a named capture participating |
| `if_capture N then { ... } else { ... }` | branch on an indexed capture participating |
| `if_followed_by { ... } then { ... } else { ... }` | branch on a lookahead succeeding |
| `if_not_followed_by { ... } then { ... } else { ... }` | branch on a negative lookahead succeeding |
| `if_preceded_by { ... } then { ... } else { ... }` | branch on a lookbehind succeeding |
| `if_not_preceded_by { ... } then { ... } else { ... }` | branch on a negative lookbehind succeeding |

```bash
./target/release/nodia eval '
val p = regex {
  optional group {
    "a"
  }
  "b"
  if_capture 1 then {
    "c"
  } else {
    "d"
  }
}
emit p
emit regex.test("abc", p, regex.full)
emit regex.test("bd", p, regex.full)
emit regex.test("abd", p, regex.full)
'
```

```text
(a)?b(?(1)c|d)
true
true
false
```

Classic regex conditionals also normalize through the inverse raw-regex path:

```nodia
val p = regex {
  r"(a)?b(?(1)c|d)"
}
```

`if_*` also works without `then` / `else` when you want a zero-width condition
only, and `if_matches { ... }` covers general assertion-style conditions that
are not just capture checks or lookarounds.

## Properties, Until, And Control

| Form | Classic | Meaning |
| --- | --- | --- |
| `property "Greek"` | `\p{Greek}` | Unicode property |
| `not_property "Greek"` | `\P{Greek}` | negated Unicode property |
| `until { ... }` | `(?~...)` | match until the inner pattern would match |
| `until { ... } then { ... }` | `(?~|...|...)` | match a body within an until-limited range |
| `until_stop { ... }` | `(?~|...)` | limit the active haystack range |
| `until_clear` | `(?~|)` | clear an active until-stop range |
| `define { ... }` | `(?(DEFINE)...)` | define subroutine groups without matching |
| `fail` / `accept` / `commit` / `skip` / `prune` | `(*FAIL)` etc. | backtracking control verbs |

```bash
./target/release/nodia eval '
val greek = regex {
  start_text
  one_or_more property "Greek"
  "A.+"
  end_text
}
val repeated = regex {
  named num { one_or_more digit }
  " x "
  call num
}
emit greek
emit regex.test("ΩβA.+", greek, regex.full)
emit repeated
emit regex.test("12 x 34", repeated, regex.full)
'
```

```text
\A\p{Greek}+A\.\+\z
true
(?<num>\d+) x \g<num>
true
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
emit regex.test("ABCdef", p)
emit regex.test("ABCDEF", p)
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

* `regex.test(text, pattern)` — returns `bool`; success means the pattern matched somewhere.
* `regex.test(text, pattern, regex.full)` — returns `bool`; success means the whole text matched.
* `regex.find(text, pattern)` — returns the first match map or `null`.
* `regex.find(text, pattern, regex.all)` — returns all non-overlapping matches.
* `regex.replace(text, pattern, replacement)` — replace literal text, or regex
  matches when `pattern` is a regex value; supports `$(0)`, `$(1)`, `$(name)`,
  `$$` placeholders in regex mode.
* `regex.split(text, pattern)` — split on a literal separator, or on regex matches
  when `pattern` is a regex value.

`test` and `find` accept regex values **or** plain strings. A plain string
there is compiled as raw regex text. `replace` and `split` share the
text-builtin behavior: a plain string stays literal text, while a regex value
enables regex mode.

## Match Shape

A `regex.find(...)` hit is a map:

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

`start` and `end` are **Unicode scalar value offsets**, so they line up with
`len(string)` and `slice(...)`. Use `text.slice(..., text.scalar, ...)` when
you want to slice on the same unit, or `text.offset(...)` when you need to
cross the byte/scalar boundary. See
[Text Semantics](../reference/text-semantics.md) for the official `0.7.5`
model.

## Replacement Placeholders

When `regex.replace(...)` receives a regex value as the pattern, the replacement
string supports
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

A placeholder that refers to a missing capture is a runtime error.

When both the regex pattern and the replacement are source literals, `nodia
check` can also reject malformed placeholder syntax and impossible capture
references before runtime.

## Why A DSL?

* **Readable**: regex blocks look like outlines, not punctuation soup.
* **Composable**: pieces are nameable identifiers, not magic glyphs.
* **Statically inspectable**: `nodia check` validates regex DSL structure
  before execution.
* **Same engine, same semantics**: `regex { ... }` compiles to a normal
  regex string that the runtime executes via `fancy-regex` — the same engine
  that handles raw-string regex patterns.
