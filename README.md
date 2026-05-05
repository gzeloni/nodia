# Dobra

Dobra is a small programming language for text automation and structured output generation.

It is built for scripts that assemble files, prompts, configuration snippets, reports, changelogs,
payloads, and other text artifacts without reaching for a full general-purpose language.

Source files use the `.dob` extension.

Complete documentation is available in [docs/reference.md](docs/reference.md).

## Status

Dobra is experimental. The current implementation is `v0.3`.

The v0.3 focus is real IO, streams, stronger text helpers, mathematical helpers, list/data helpers,
and the existing v0.2 tooling foundation: formatting, imports, diagnostics, and project support.

## Install From Source

Dobra is implemented in Rust and currently uses only the Rust standard library.

```bash
cargo build --release
```

The release binary is generated at:

```bash
target/release/dobra
```

## Quick Start

Create `hello.dob`:

```dobra
const name = input.name
emit "Hello, {name}"
```

Run it:

```bash
dobra run hello.dob --var name=Gustavo
```

Output:

```text
Hello, Gustavo
```

## CLI

```bash
dobra run file.dob
dobra check file.dob
dobra fmt file.dob
dobra fmt .
dobra fmt --check .
dobra fmt --stdout file.dob
dobra eval 'emit "hello"'
dobra tokens file.dob --json
dobra ast file.dob --json
dobra init
dobra version
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
dobra run file.dob
dobra run file.dob --var name=Ana
dobra run file.dob --vars name=Ana env=prod
dobra run file.dob --vars config.json
dobra run file.dob --out output.txt
dobra run file.dob --allow-write
```

CLI variables are exposed through the readonly `input` object.

### Check

```bash
dobra check file.dob
dobra check file.dob --json
```

`check` validates lexing and parsing without executing the program.

### Format

Dobra has one canonical style. The formatter decides layout.

```bash
dobra fmt file.dob
dobra fmt .
dobra fmt --check .
dobra fmt --stdout file.dob
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

Example input:

```dobra
const user={name:"Ana",role:"dev"}
if user.name!=""{emit "hello {user.name}"}
```

Formatted output:

```dobra
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
dobra init
```

Generated layout:

```text
dobra.toml
src/
  main.dob
```

`dobra.toml`:

```toml
name = "dobra-project"
entry = "src/main.dob"
```

When a command accepts a file path, omitting the path makes Dobra look for `dobra.toml`
and use its `entry` file.

## Language Basics

### Variables

```dobra
let name = "Ana"
const env = "prod"

emit "{name} / {env}"
```

`let` declares a mutable variable. `const` declares an immutable variable.

### Strings and Interpolation

```dobra
const user = "john"
emit "Hello, {capitalize(user)}"
```

Triple-quoted strings are supported:

```dobra
emit """
APP_NAME={input.app}
APP_ENV=prod
"""
```

### Conditionals

```dobra
if input.env == "prod" {
  emit "Production"
} else {
  emit "Development"
}
```

### Loops

```dobra
for user in ["ana", "john", "maria"] {
  emit capitalize(user)
}
```

```dobra
let i = 0

while i < 3 {
  emit "i={i}"
  i = i + 1
}
```

### Functions

```dobra
fn greet(name) {
  return "Hello, {capitalize(name)}"
}

emit greet("ana")
```

### Lists and Maps

```dobra
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
The `.dob` extension is optional.

```dobra
import './lib/format' as fmt

emit fmt.title
```

Without `as`, selected top-level bindings are inserted into the current scope:

```dobra
import './lib/constants' show title, version

emit title
emit version
```

You can also hide names:

```dobra
import './lib/constants' hide internal_token
```

Circular imports are allowed. Dobra caches modules by resolved path and links imported
bindings lazily. A cycle only fails if code tries to read a binding before that module has
initialized it. Imported `let` bindings remain mutable; imported `const` and `fn` bindings are read-only.

## IO

Dobra v0.3 supports real file IO through streams.

```dobra
const src = open("input.txt", "read")
const out = open("output.txt", "write")

let line = readln(src)
while line != null {
  writeln(out, upper(line))
  line = readln(src)
}

close(src)
close(out)
```

Short file helpers are built on the same IO model:

```dobra
const text = read("input.txt")
write("output.txt", upper(text))
append("output.txt", "\n")
```

File writes require explicit permission:

```bash
dobra run script.dob --allow-write
```

Without it, Dobra returns `error[E3001]: file write requires --allow-write`.

Standard streams are available as values:

```dobra
writeln(stdout, "ok")
writeln(stderr, "error")
const line = readln(stdin)
```

## Standard Library

Text:

| Function | Description |
|---|---|
| `upper(value)` | Converts text to uppercase |
| `lower(value)` | Converts text to lowercase |
| `capitalize(value)` | Capitalizes text |
| `trim(value)` | Trims surrounding whitespace |
| `replace(value, from, to)` | Replaces text |
| `split(value, delimiter)` | Splits text into a list |
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
vscode/dobra-language
```

Install it from VSCode with:

```text
Developer: Install Extension from Location...
```

Then select the `vscode/dobra-language` folder.

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
  project.rs    dobra.toml helpers
  runtime.rs    evaluator/runtime
  stdlib.rs     standard library
  token.rs      token definitions
  value.rs      runtime values
```
