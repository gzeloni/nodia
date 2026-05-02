# Orich

Orich is a small scripting language for text automation and structured output generation.

It is designed for cases where using a general-purpose language just to assemble text,
configuration files, payloads, prompts, or reports feels heavier than necessary.

Source files use the `.och` extension.

## Status

Orich is experimental. The current implementation is `v0.1` and focuses on a small,
clear core language:

- explicit output with `emit`
- variables and constants with `let` and `const`
- string interpolation with `{expr}`
- lists, maps, booleans, numbers, strings, and `null`
- `if`, `else`, `for`, `while`, `break`, and `continue`
- user-defined functions with `fn` and `return`
- CLI input through `input`
- a small standard library

## Build

Orich is implemented in Rust and currently uses only the Rust standard library.

```bash
cargo build --release
```

The release binary is generated at:

```bash
target/release/orich
```

## Quick Start

Create a file named `hello.och`:

```orich
let name = input.name
emit "Hello, {name}"
```

Run it:

```bash
target/release/orich run hello.och --vars name=Gustavo
```

Output:

```text
Hello, Gustavo
```

## CLI

Run a file:

```bash
orich run file.och
```

Pass variables:

```bash
orich run file.och --vars name=Ana env=prod
```

Load variables from a flat JSON or YAML file:

```bash
orich run file.och --vars config.json
orich run file.och --vars config.yaml
```

Write output to `file.och.out`:

```bash
orich run file.och --output
```

Show version:

```bash
orich --version
```

## Language Basics

### Variables

```orich
let name = "Ana"
const env = "prod"

emit "{name} / {env}"
```

`let` declares a mutable variable. `const` declares an immutable variable.

### Input

CLI variables are exposed through the readonly `input` object:

```orich
let app = input.app
let port = int(input.port)

emit "APP={app}"
emit "PORT={port}"
```

Run:

```bash
orich run app.och --vars app=api port=8080
```

### Strings and Interpolation

```orich
let user = "john"
emit "Hello, {capitalize(user)}"
```

Triple-quoted strings are supported:

```orich
let app = input.app

emit """
APP_NAME={app}
APP_ENV=prod
"""
```

### Conditionals

```orich
let env = input.env

if env == "prod" {
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
let users = ["ana", "john"]
emit users[0]
```

```orich
let user = {
  name: "Ana",
  role: "admin"
}

emit "{user.name}: {user.role}"
```

## Standard Library

The current standard library includes:

| Function | Description |
|---|---|
| `uppercase(value)` | Converts text to uppercase |
| `lowercase(value)` | Converts text to lowercase |
| `capitalize(value)` | Capitalizes text |
| `trim(value)` | Trims surrounding whitespace |
| `replace(value, from, to)` | Replaces text |
| `split(value, delimiter)` | Splits text into a list |
| `len(value)` | Returns length of a string, list, or map |
| `int(value)` | Converts to integer |
| `float(value)` | Converts to float |
| `string(value)` | Converts to string |
| `bool(value)` | Converts to boolean |
| `range(end)` | Produces integers from `0` to `end - 1` |
| `range(start, end)` | Produces integers from `start` to `end - 1` |

## Reserved Words

Core `v0.1` reserved words:

```text
let const fn return emit
if else for in while break continue
true false null
and or not
import from as
```

Reserved for future versions:

```text
match case default
try catch throw defer
type enum struct namespace use
```

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
  ast.rs       AST definitions
  error.rs     error types
  lexer.rs     lexer/tokenizer
  parser.rs    parser
  runtime.rs   evaluator/runtime
  stdlib.rs    standard library
  token.rs     token definitions
  value.rs     runtime values
  main.rs      CLI
  lib.rs       public Rust API

tests/
  smoke.sh     CLI smoke tests

vscode/
  orich-language/
```

The old Python implementation remains in `illex/` as a historical reference only.

## Development

Run tests:

```bash
make test
```

Run Rust checks directly:

```bash
cargo check
cargo test
```

Format code:

```bash
cargo fmt
```

## License

Orich is licensed under GNU General Public License v3.0 or later.

See [LICENSE](LICENSE).
