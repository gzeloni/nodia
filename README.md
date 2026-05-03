# Orich

Orich is a small programming language for text automation and structured output generation.

It is built for scripts that assemble files, prompts, configuration snippets, reports, changelogs,
payloads, and other text artifacts without reaching for a full general-purpose language.

Source files use the `.och` extension.

## Status

Orich is experimental. The current implementation is `v0.2`.

The v0.2 focus is tooling: a stronger CLI, automatic formatting, parser hardening,
relative imports, JSON-friendly diagnostics, and a small standard library for text/data work.

## Install From Source

Orich is implemented in Rust and currently uses only the Rust standard library.

```bash
cargo build --release
```

The release binary is generated at:

```bash
target/release/orich
```

## Quick Start

Create `hello.och`:

```orich
const name = input.name
emit "Hello, {name}"
```

Run it:

```bash
orich run hello.och --var name=Gustavo
```

Output:

```text
Hello, Gustavo
```

## CLI

```bash
orich run file.och
orich check file.och
orich fmt file.och
orich fmt .
orich fmt --check .
orich fmt --stdout file.och
orich eval 'emit "hello"'
orich tokens file.och --json
orich ast file.och --json
orich init
orich version
```

Global flags:

```bash
--json
--quiet
--verbose
--color auto|always|never
--help
--version
```

### Run

```bash
orich run file.och
orich run file.och --var name=Ana
orich run file.och --vars name=Ana env=prod
orich run file.och --vars config.json
orich run file.och --out output.txt
```

CLI variables are exposed through the readonly `input` object.

### Check

```bash
orich check file.och
orich check file.och --json
```

`check` validates lexing and parsing without executing the program.

### Format

Orich has one canonical style. The formatter decides layout.

```bash
orich fmt file.och
orich fmt .
orich fmt --check .
orich fmt --stdout file.och
```

Formatter rules in v0.2:

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

Example input:

```orich
const user={name:"Ana",role:"dev"}
if user.name!=""{emit "hello {user.name}"}
```

Formatted output:

```orich
const user = {
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
orich init
```

Generated layout:

```text
orich.toml
src/
  main.och
```

`orich.toml`:

```toml
name = "orich-project"
entry = "src/main.och"
```

When a command accepts a file path, omitting the path makes Orich look for `orich.toml`
and use its `entry` file.

## Language Basics

### Variables

```orich
let name = "Ana"
const env = "prod"

emit "{name} / {env}"
```

`let` declares a mutable variable. `const` declares an immutable variable.

### Strings and Interpolation

```orich
const user = "john"
emit "Hello, {capitalize(user)}"
```

Triple-quoted strings are supported:

```orich
emit """
APP_NAME={input.app}
APP_ENV=prod
"""
```

### Conditionals

```orich
if input.env == "prod" {
  emit "Production"
} else {
  emit "Development"
}
```

### Loops

```orich
for user in ["ana", "john", "maria"] {
  emit capitalize(user)
}
```

```orich
let i = 0

while i < 3 {
  emit "i={i}"
  i = i + 1
}
```

### Functions

```orich
fn greet(name) {
  return "Hello, {capitalize(name)}"
}

emit greet("ana")
```

### Lists and Maps

```orich
const user = {
  name: "Ana",
  roles: ["admin", "dev"],
}

emit user.name
emit user.roles[0]
```

Lists, maps, calls, and function parameters can be written across multiple lines.

### Imports

Imports are relative to the current file and use Dart-style clauses in a smaller form.
The `.och` extension is optional.

```orich
import './lib/format' as fmt

emit fmt.title
```

Without `as`, selected top-level bindings are inserted into the current scope:

```orich
import './lib/constants' show title, version

emit title
emit version
```

You can also hide names:

```orich
import './lib/constants' hide internal_token
```

Circular imports are allowed. Orich caches modules by resolved path and links imported
bindings lazily. A cycle only fails if code tries to read a binding before that module has
initialized it. Imported `let` bindings remain mutable; imported `const` and `fn` bindings are read-only.

## Standard Library

| Function | Description |
|---|---|
| `uppercase(value)` | Converts text to uppercase |
| `lowercase(value)` | Converts text to lowercase |
| `capitalize(value)` | Capitalizes text |
| `trim(value)` | Trims surrounding whitespace |
| `replace(value, from, to)` | Replaces text |
| `split(value, delimiter)` | Splits text into a list |
| `join(list, delimiter)` | Joins a list into text |
| `contains(value, needle)` | Checks strings, lists, or map keys |
| `starts_with(value, prefix)` | Checks a text prefix |
| `ends_with(value, suffix)` | Checks a text suffix |
| `indent(text, spaces_or_prefix)` | Prefixes each line |
| `dedent(text)` | Removes common indentation |
| `keys(map)` | Returns map keys |
| `values(map)` | Returns map values |
| `len(value)` | Returns length of a string, list, or map |
| `int(value)` | Converts to integer |
| `float(value)` | Converts to float |
| `string(value)` | Converts to string |
| `bool(value)` | Converts to boolean |
| `range(end)` | Produces integers from `0` to `end - 1` |
| `range(start, end)` | Produces integers from `start` to `end - 1` |

## Reserved Words

```text
const let fn return
if else for in while break continue
emit import as show hide
true false null
and or not
```

Reserved for future versions:

```text
from match case default
try catch throw defer
type enum struct namespace use
```

## Exit Codes

| Code | Meaning |
|---:|---|
| `0` | Success |
| `1` | Language/runtime error |
| `2` | Invalid CLI usage |
| `3` | IO error |
| `4` | Internal error |

## VSCode Syntax Highlighting

A local VSCode extension is included at:

```text
vscode/orich-language
```

Install it from VSCode with:

```text
Developer: Install Extension from Location...
```

Then select the `vscode/orich-language` folder.

## Project Layout

```text
src/
  ast.rs        AST definitions
  cli.rs        command-line interface
  error.rs      diagnostics and error types
  formatter.rs  canonical formatter
  lexer.rs      lexer/tokenizer
  lib.rs        public Rust API
  parser.rs     parser
  project.rs    orich.toml helpers
  runtime.rs    evaluator/runtime
  stdlib.rs     standard library
  token.rs      token definitions
  value.rs      runtime values
```
