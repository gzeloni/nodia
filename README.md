# Nodia

Nodia is a small programming language for text automation and structured output generation.

It is built for scripts that assemble files, prompts, configuration snippets, reports, changelogs,
payloads, and other text artifacts without reaching for a full general-purpose language.

Source files use the `.nod` extension.

Complete documentation is available in [docs/index.md](docs/index.md).

The archived v0.5 baseline is documented in
[docs/_archive/specification.md](docs/_archive/specification.md). The current
implementation keeps the regex DSL introduced in v0.6 and now defines
explicit text semantics on top of that baseline.

## Status

Nodia is experimental. The current release is `v0.7.3`.

The v0.7 focus is explicit text semantics: Nodia text is UTF-8, string
positions stay scalar-based, byte boundaries are part of the public model, and
normalization/case-folding, UTF-8 encode/decode, newline cleanup, and
unit-aware access stay explicit rather than implicit magic.

## Install From Source

Nodia is implemented in Rust and uses the standard library plus
`fancy-regex`, `unicode-normalization`, `unicode-segmentation`, and `caseless`
for regex execution and explicit Unicode text semantics.

```bash
cargo build --release
```

The release binary is generated at:

```bash
target/release/nodia
```

## Quick Start

Create `hello.nod`:

```nodia
val name = input.name
emit "Hello, {name}"
```

Run it:

```bash
nodia run hello.nod --var name=Gustavo
```

Output:

```text
Hello, Gustavo
```

## CLI

```bash
nodia run file.nod
nodia check file.nod
nodia fmt file.nod
nodia fmt .
nodia fmt --check .
nodia fmt --stdout file.nod
nodia eval 'emit "hello"'
nodia tokens file.nod --json
nodia ast file.nod --json
nodia init
nodia version
```

Global flags:

```bash
--json
--quiet
--verbose
--color auto|always|never
--allow-write
--allow-env
--allow-process
--help
--version
```

### Run

```bash
nodia run file.nod
nodia run file.nod --var name=Ana
nodia run file.nod --vars name=Ana env=prod
nodia run file.nod --vars config.json
nodia run file.nod --out output.txt
nodia run file.nod --allow-write
```

CLI variables are exposed through the readonly `input` object.

### Check

```bash
nodia check file.nod
nodia check file.nod --json
```

`check` validates lexing, parsing, module uses, regex DSL structure, and the
v0.7 semantic checks without executing the program.

## Regex DSL

Nodia keeps a native `regex { ... }` expression. It evaluates to a regex value
in the runtime. When emitted, interpolated, or converted with `conversion.string(...)`,
it renders to classic regex text.

```nodia
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
```

Output:

```text
(?i)^(?<year>\d{4})-\d{2}-\d{2}$
```

The regex AST now separates:
- literals
- anchors
- character classes
- `any_char` and `any_codepoint`
- groups and lookarounds
- references
- quantifier kind and quantifier mode
- global and scoped flags

The syntax accepts both compact sugar and explicit forms such as `literal("abc")`, `char(".")`, `with_flags(...) { ... }`, and `without_flags(...) { ... }`.

Regex execution uses function style:

```nodia
use re

val url = regex(case_insensitive) {
  named scheme {
    either {
      branch {
        "http"
      }
      branch {
        "https"
      }
    }
  }
  "://"
  named host {
    one_or_more {
      char_set {
        letter
        digit
        "."
        "-"
      }
    }
  }
}

val hit = re.find("go to https://example.com now", url)
emit hit.named.host
emit re.test("http://a", url)
emit re.full_match("https://example.com", url)
emit re.replace("go to https://example.com now", url, "<$(host)>")
emit re.split("ana   bruno\tcarla", regex {
  one_or_more whitespace
})
```

Regex replacements use Nodia placeholders:
- `$(0)` for the whole match
- `$(1)`, `$(2)`, ... for indexed captures
- `$(name)` for named captures
- `$$` for a literal dollar sign

### Format

Nodia has one canonical style. The formatter decides layout.

```bash
nodia fmt file.nod
nodia fmt .
nodia fmt --check .
nodia fmt --stdout file.nod
```

Formatter rules:

| Rule | Style |
|---|---|
| Indentation | 2 spaces |
| Braces | opening brace on the same line |
| Operators | spaced on both sides |
| Blocks | always use `{}` |
| File ending | exactly one trailing newline |
| Maps | multi-line when non-empty |
| Short lists/calls | inline when they fit |
| Comments | preserved as statement comments |
| Line width | formatter-controlled lines target 60 characters |

Example input:

```nodia
val user={name:"Ana",role:"dev"}
if user.name!=""{emit "hello {user.name}"}
```

Formatted output:

```nodia
val user = {
  name: "Ana",
  role: "dev",
}

if user.name != "" {
  emit "hello {user.name}"
}
```

## Projects

Create a basic project:

```bash
nodia init
```

Generated layout:

```text
nodia.toml
src/
  main.nod
```

`nodia.toml`:

```toml
name = "nodia-project"
entry = "src/main.nod"
```

When a command accepts a file path, omitting the path makes Nodia look for `nodia.toml`
and use its `entry` file.

## Language Basics

### Variables

```nodia
var name = "Ana"
val env = "prod"

emit "{name} / {env}"
```

`var` declares a mutable variable. `val` declares an immutable variable.

### Strings and Interpolation

```nodia
use text

val user = "john"
emit "Hello, {text.capitalize(user)}"
```

Raw and triple-quoted strings are supported:

```nodia
emit r'{"name":"Ana","tpl":"hello {world}"}'

emit """
APP_NAME=nodia
APP_ENV=prod
"""
```

### Conditionals

```nodia
if input.env == "prod" {
  emit "Production"
} else {
  emit "Development"
}
```

### Loops

```nodia
use text

for user in ["ana", "john", "maria"] {
  emit text.capitalize(user)
}
```

```nodia
var i = 0

while i < 3 {
  emit "i={i}"
  i = i + 1
}
```

### Functions

```nodia
use text
use collections

func greet(name) {
  return "Hello, {text.capitalize(name)}"
}

emit greet("ana")
emit collections.map(lambda(x) { x * 2 }, [1, 2, 3])
```

### Lists and Maps

```nodia
val user = {
  name: "Ana",
  roles: ["admin", "dev"],
}

emit user.name
emit user.roles[0]
```

Lists, maps, calls, and function parameters can be written across multiple lines.

### Uses

Uses are relative to the current file and use Dart-style clauses in a smaller form.
The `.nod` extension is optional.

```nodia
use './lib/format' as fmt

emit fmt.title
```

Without `as`, selected top-level bindings are inserted into the current scope:

```nodia
use './lib/constants' pick title, version

emit title
emit version
```

You can also hide names:

```nodia
use './lib/constants' hide internal_token
```

Circular uses are allowed. Nodia caches modules by resolved path and links used
bindings lazily. A cycle only fails if code tries to read a binding before that module has
initialized it. Used `var` bindings remain mutable; used `val` and `func` bindings are read-only.

## IO

Nodia v0.7 supports real file IO through the `io` namespace.

```nodia
use io
use text

val src = io.open("input.txt", "read")
val out = io.open("output.txt", "write")

var line = io.readln(src)
while line != null {
  io.writeln(out, text.upper(line))
  line = io.readln(src)
}

io.close(src)
io.close(out)
```

Short file helpers are built on the same IO model:

```nodia
use io
use text

val content = io.read("input.txt")
io.write("output.txt", text.upper(content))
io.append("output.txt", "\n")
```

File writes require explicit permission:

```bash
nodia run script.nod --allow-write
```

Without it, Nodia returns `error[E3001]: file write requires --allow-write`.

Standard streams are available as values:

```nodia
use io

io.writeln(io.stdout, "ok")
io.writeln(io.stderr, "error")
val line = io.readln(io.stdin)
```

## Standard Library

There is no implicit stdlib prelude. Outside of reserved words and local/module
bindings, stdlib access is always explicit through `use`.

Text (`use text`):

| Function | Description |
|---|---|
| `text.upper(value)` | Converts text to uppercase |
| `text.lower(value)` | Converts text to lowercase |
| `text.casefold(value)` | Applies Unicode default case folding |
| `text.capitalize(value)` | Capitalizes text |
| `text.trim(value)` | Trims surrounding whitespace |
| `text.nfc(value)` | Canonically normalizes text |
| `text.nfd(value)` | Canonically decomposes text |
| `text.nfkc(value)` | Compatibility-normalizes text |
| `text.nfkd(value)` | Compatibility-decomposes text |
| `text.replace(value, from, to)` | Replaces text with literal or regex patterns |
| `text.replace_all(value, from, to)` | Explicit alias of `text.replace(...)` |
| `text.split(value, delimiter)` | Splits text with a literal or regex delimiter |
| `text.split_regex(value, pattern)` | Explicit alias of `text.split(...)` |
| `text.join(list, delimiter)` | Joins a list into text |
| `text.lines(value)` | Splits text into lines |
| `text.unlines(list)` | Joins values with newlines |
| `text.words(value)` | Splits text by whitespace |
| `text.contains(value, needle)` | Checks strings, lists, or map keys |
| `text.starts(value, prefix)` | Checks a text prefix |
| `text.ends(value, suffix)` | Checks a text suffix |
| `text.indent(text, spaces_or_prefix)` | Prefixes each line |
| `text.dedent(text)` | Removes common indentation |
| `text.byte_len(text)` | Returns the UTF-8 byte length |
| `text.byte_offset(text, scalar_offset)` | Converts scalar offsets to byte offsets |
| `text.scalar_offset(text, byte_offset)` | Converts byte offsets to scalar offsets |
| `text.scalar(text, scalar_index)` | Returns one Unicode scalar value |
| `text.grapheme_len(text)` | Counts grapheme clusters |
| `text.grapheme(text, grapheme_index)` | Returns one grapheme cluster |
| `text.byte_slice(text, start, end)` | Slices with explicit byte offsets |
| `text.scalar_slice(text, start, end)` | Slices with explicit scalar offsets |
| `text.grapheme_slice(text, start, end)` | Slices with explicit grapheme offsets |

Math (`use numbers`):

| Function | Description |
|---|---|
| `numbers.abs(value)` | Absolute value |
| `numbers.floor(value)` | Rounds down |
| `numbers.ceil(value)` | Rounds up |
| `numbers.round(value)` | Rounds to nearest integer |
| `numbers.sqrt(value)` | Square root |
| `numbers.pow(base, exponent)` | Power |
| `numbers.min(a, b)` | Minimum |
| `numbers.max(a, b)` | Maximum |
| `numbers.clamp(value, min, max)` | Clamps into a range |
| `numbers.sum(list)` | Sums numeric values |
| `numbers.avg(list)` | Averages numeric values |

Data, collections, and conversion (`use collections`, `use conversion`):

| Function | Description |
|---|---|
| `collections.keys(map)` | Returns map keys |
| `collections.values(map)` | Returns map values |
| `collections.entries(map)` | Returns `{key, value}` entries |
| `collections.len(value)` | Returns length of a string, list, or map |
| `conversion.int(value)` | Converts to integer |
| `conversion.float(value)` | Converts to float |
| `conversion.string(value)` | Converts to string |
| `conversion.bool(value)` | Converts to boolean |
| `numbers.range(end)` | Produces integers from `0` to `end - 1` |
| `numbers.range(start, end)` | Produces integers from `start` to `end - 1` |
| `collections.push(list, value)` | Returns a list with value appended |
| `collections.pop(list)` | Returns a list without the last value |
| `collections.first(list)` | Returns first value or `null` |
| `collections.last(list)` | Returns last value or `null` |
| `collections.slice(list_or_text, start, end)` | Returns a slice |
| `collections.reverse(list_or_text)` | Reverses value |
| `collections.sort(list)` | Sorts values deterministically |
| `collections.unique(list)` | Removes duplicate values |

## Reserved Words

```text
val var func return
if else for in while break continue
emit use as pick hide
true false null
and or not
```

Reserved for future versions:

```text
from match case default
try catch throw defer
type enum struct namespace
```

## Exit Codes

| Code | Meaning |
|---:|---|
| `0` | Success |
| `1` | Language/runtime error |
| `2` | Invalid CLI usage |
| `3` | IO error |
| `4` | Internal error |

## VSCode Extension

A local VSCode extension is included at:

```text
vscode/nodia-language
```

Install it from VSCode with:

```text
Developer: Install Extension from Location...
```

Then select the `vscode/nodia-language` folder.

It provides syntax highlighting plus completions for stdlib namespaces such as
`json.read()`, `csv.write()`, `text.upper()`, and `use re`.

## Project Layout

```text
src/
  ast.rs        AST definitions
  cli.rs        command-line interface
  error.rs      diagnostics and error types
  formatter.rs  canonical formatter
  io.rs         file and stream runtime support
  lexer.rs      lexer/tokenizer
  lib.rs        public Rust API
  parser.rs     parser
  project.rs    nodia.toml helpers
  runtime.rs    evaluator/runtime
  stdlib.rs     standard library
  token.rs      token definitions
  value.rs      runtime values
```
