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

Nodia is experimental. The current release is `v0.8.3`.

The `0.7.x` text-semantics line is now closed: Nodia text is UTF-8, string
positions stay scalar-based, byte boundaries are part of the public model, and
normalization/case-folding, UTF-8 encode/decode, newline cleanup, explicit
bytes-aware JSON/CSV parsing, and grapheme-safe formatting stay explicit
rather than implicit magic.

`0.8.0` opened the recoverable-error line with first-class `result` values.
`0.8.1` adopts that model across IO, decode, regex matching, JSON, CSV, and
datetime parsing. `0.8.2` adds the idiomatic pipeline helpers
`result.value_or(...)`, `result.then(...)`, and `result.recover(...)`.
`0.8.3` adds structured nested error context/span reporting and preserves
partial `emit` output when a later fatal error aborts the run.

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
use result

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

val hit = result.raise(regex.find("go to https://example.com now", url))
emit hit.named.host
emit result.raise(regex.test("http://a", url))
emit result.raise(regex.test("https://example.com", url, regex.full))
emit regex.replace("go to https://example.com now", url, "<$(host)>")
emit regex.split("ana   bruno\tcarla", regex {
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
use result

val src = result.raise(io.open("input.txt", "read"))
val out = result.raise(io.open("output.txt", "write"))

var line = result.raise(io.readln(src))
while line != null {
  result.raise(io.writeln(out, text.upper(line)))
  line = result.raise(io.readln(src))
}

result.raise(io.close(src))
result.raise(io.close(out))
```

Short file helpers are built on the same IO model:

```nodia
use io
use text
use result

val content = result.raise(io.read("input.txt"))
result.raise(io.write("output.txt", text.upper(content)))
result.raise(io.append("output.txt", "\n"))
```

File writes require explicit permission:

```bash
nodia run script.nod --allow-write
```

Without it, IO builtins return `err({code: "E3001", ...})`.

Standard streams are available as values:

```nodia
use io
use result

result.raise(io.writeln(io.stdout, "ok"))
result.raise(io.writeln(io.stderr, "error"))
val line = result.raise(io.readln(io.stdin))
```

## Standard Library

There is no implicit stdlib prelude. Outside of reserved words and local/module
bindings, stdlib access is always explicit through `use`.

The current surface is namespace-first:

- `use text` for case, normalization, codecs, bytes, and unit-aware access
- `use numbers` for math and `numbers.range(...)`
- `use collections` for list/map helpers
- `use conversion` for explicit `string`, `int`, `float`, `bool`
- `use format`, `use io`, `use system`, `use result`, `use datetime`, `use json`, `use csv`
- `regex` is built into the language: `regex { ... }`, `regex.test(...)`, `regex.find(...)`, `regex.replace(...)`, `regex.split(...)`

Direct selected imports are also supported when they improve clarity:

```nodia
use numbers pick range
use conversion pick string

for i in range(3) {
  emit string(i)
}
```

The text-semantics line is stabilized in `0.7.5` around:

- `text.normalize(text, text.nfc | text.nfd | text.nfkc | text.nfkd | text.lf | text.crlf)`
- `text.encode(text, text.utf8)` and `text.decode(bytes, text.utf8[, text.lossy])`
- `text.len`, `text.at`, `text.slice`, and `text.offset` with explicit `text.byte`, `text.scalar`, and `text.grapheme`
- first-class `bytes` values across `io`, `system.exec`, `json.read`, and `csv.read`

For the complete module docs, see [docs/stdlib/index.md](docs/stdlib/index.md).
For upgrade guidance from older `0.7.x` naming, see
[docs/reference/migration-0.7.5.md](docs/reference/migration-0.7.5.md).

The recoverable-error surface now covers:

- `result.ok(value)` and `result.err(code, message)`
- `result.is_ok(...)` / `result.is_err(...)`
- `result.value(...)` / `result.value_or(...)` / `result.error(...)`
- `result.then(...)` / `result.recover(...)`
- `result.raise(...)` to turn a recoverable error back into a fatal runtime failure
- `text.decode(...)`, `io.*`, `regex.test(...)`, `regex.find(...)`, `json.read(...)`,
  `csv.read(...)`, and `datetime.parse(...)` now return `result`

## Reserved Words

```text
val var func return
if else for in while break continue
emit use as pick hide
true false null
and or not
regex
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

## Editor Support

A local VS Code extension is included at:

```text
vscode/nodia-language
```

Install it from VS Code with:

```text
Developer: Install Extension from Location...
```

Then select the `vscode/nodia-language` folder.

It provides syntax highlighting, stdlib-aware completions, format-on-save
through `nodia fmt`, and checker diagnostics through `nodia check`.

A local Zed extension is also included at:

```text
zed/nodia
```

Install it from Zed with:

```text
zed: install dev extension
```

Before installing it, bootstrap the local grammar repository once:

```bash
./zed/bootstrap-dev-grammar.sh
```

Then select the `zed/nodia` folder.

It uses the local Tree-sitter grammar at `zed/tree-sitter-nodia` and currently
provides `.nod` file association, syntax highlighting, bracket matching,
indentation, and outline support.

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
