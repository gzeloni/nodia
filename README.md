# Nodia

Nodia is a small programming language for text automation and structured output generation.

It is built for scripts that assemble files, prompts, configuration snippets, reports, changelogs,
payloads, and other text artifacts without reaching for a full general-purpose language.

Source files use the `.nod` extension.

Complete documentation is available in [docs/reference.md](docs/reference.md).

The formal v0.5 baseline is documented in [docs/specification.md](docs/specification.md).
The current implementation adds the v0.6 regex DSL on top of that baseline.

## Status

Nodia is experimental. The current implementation is `v0.6`.

The v0.6 focus is text work: the v0.5 identity surface remains intact, and the first new native syntax layer is `regex { ... }`, a readable DSL that evaluates to regex values and can render to classic regex text.

## Install From Source

Nodia is implemented in Rust and currently uses only the Rust standard library.

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

`check` validates lexing, parsing, module uses, regex DSL structure, and the v0.5 semantic baseline without executing the program.

## Regex DSL

Nodia v0.6 adds a native `regex { ... }` expression. It evaluates to a regex value in the runtime. When emitted, interpolated, or converted with `string(...)`, it renders to classic regex text.

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

The v0.6 regex AST now separates:
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

val hit = find("go to https://example.com now", url)
emit hit.named.host
emit test("http://a", url)
emit full_match("https://example.com", url)
emit replace("go to https://example.com now", url, "<$(host)>")
emit split("ana   bruno\tcarla", regex {
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
val user = "john"
emit "Hello, {capitalize(user)}"
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
for user in ["ana", "john", "maria"] {
  emit capitalize(user)
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
func greet(name) {
  return "Hello, {capitalize(name)}"
}

emit greet("ana")
emit map(lambda(x) { x * 2 }, [1, 2, 3])
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

Nodia v0.5 supports real file IO through streams.

```nodia
val src = open("input.txt", "read")
val out = open("output.txt", "write")

var line = readln(src)
while line != null {
  writeln(out, upper(line))
  line = readln(src)
}

close(src)
close(out)
```

Short file helpers are built on the same IO model:

```nodia
val text = read("input.txt")
write("output.txt", upper(text))
append("output.txt", "\n")
```

File writes require explicit permission:

```bash
nodia run script.nod --allow-write
```

Without it, Nodia returns `error[E3001]: file write requires --allow-write`.

Standard streams are available as values:

```nodia
writeln(stdout, "ok")
writeln(stderr, "error")
val line = readln(stdin)
```

## Standard Library

Text:

| Function | Description |
|---|---|
| `upper(value)` | Converts text to uppercase |
| `lower(value)` | Converts text to lowercase |
| `capitalize(value)` | Capitalizes text |
| `trim(value)` | Trims surrounding whitespace |
| `replace(value, from, to)` | Replaces text with literal or regex patterns |
| `replace_all(value, from, to)` | Explicit alias of `replace(...)` |
| `split(value, delimiter)` | Splits text with a literal or regex delimiter |
| `split_regex(value, pattern)` | Explicit alias of `split(...)` |
| `join(list, delimiter)` | Joins a list into text |
| `lines(value)` | Splits text into lines |
| `unlines(list)` | Joins values with newlines |
| `words(value)` | Splits text by whitespace |
| `contains(value, needle)` | Checks strings, lists, or map keys |
| `starts(value, prefix)` | Checks a text prefix |
| `ends(value, suffix)` | Checks a text suffix |
| `indent(text, spaces_or_prefix)` | Prefixes each line |
| `dedent(text)` | Removes common indentation |

Math:

| Function | Description |
|---|---|
| `abs(value)` | Absolute value |
| `floor(value)` | Rounds down |
| `ceil(value)` | Rounds up |
| `round(value)` | Rounds to nearest integer |
| `sqrt(value)` | Square root |
| `pow(base, exponent)` | Power |
| `min(a, b)` | Minimum |
| `max(a, b)` | Maximum |
| `clamp(value, min, max)` | Clamps into a range |
| `sum(list)` | Sums numeric values |
| `avg(list)` | Averages numeric values |

Data and conversion:

| Function | Description |
|---|---|
| `keys(map)` | Returns map keys |
| `values(map)` | Returns map values |
| `entries(map)` | Returns `{key, value}` entries |
| `len(value)` | Returns length of a string, list, or map |
| `int(value)` | Converts to integer |
| `float(value)` | Converts to float |
| `string(value)` | Converts to string |
| `bool(value)` | Converts to boolean |
| `range(end)` | Produces integers from `0` to `end - 1` |
| `range(start, end)` | Produces integers from `start` to `end - 1` |
| `push(list, value)` | Returns a list with value appended |
| `pop(list)` | Returns a list without the last value |
| `first(list)` | Returns first value or `null` |
| `last(list)` | Returns last value or `null` |
| `slice(list_or_text, start, end)` | Returns a slice |
| `reverse(list_or_text)` | Reverses value |
| `sort(list)` | Sorts values deterministically |
| `unique(list)` | Removes duplicate values |

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
