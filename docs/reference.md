# Dobra Reference v0.4

This is the complete user-facing reference for Dobra v0.4. It documents the command line,
project layout, language syntax, imports, IO, streams, standard library, and common workflows.

Dobra source files use the `.dob` extension.

## Table Of Contents

- [Install And Build](#install-and-build)
- [Command Line](#command-line)
- [Projects](#projects)
- [Source Files](#source-files)
- [Language Basics](#language-basics)
- [Imports](#imports)
- [IO And Streams](#io-and-streams)
- [Standard Library](#standard-library)
- [Diagnostics](#diagnostics)
- [Formatting Contract](#formatting-contract)
- [VSCode Support](#vscode-support)
- [Complete Examples](#complete-examples)

## Install And Build

Dobra is implemented in Rust and currently uses only the Rust standard library.

Build the debug binary:

```bash
cargo build
```

Build the release binary:

```bash
cargo build --release
```

Run the release binary directly:

```bash
target/release/dobra version
```

Expected output:

```text
dobra 0.4.0
```

## Command Line

General shape:

```bash
dobra [global-flags] <command> [command-args]
dobra <command> [command-args] [global-flags]
```

Global flags are accepted before or after the command for most workflows.

### Global Flags

| Flag | Meaning |
|---|---|
| `--json` | Uses JSON diagnostics for errors and JSON output for supported commands. |
| `--quiet` | Suppresses normal success output for commands that support quiet output. |
| `--verbose` | Reserved for richer diagnostics; currently accepted but intentionally minimal. |
| `--color auto` | Accepts color mode. Current output is plain text. |
| `--color always` | Accepts color mode. Current output is plain text. |
| `--color never` | Accepts color mode. Current output is plain text. |
| `--allow-write` | Allows Dobra code to write files through IO builtins. |
| `--help`, `-h` | Prints help. |
| `--version`, `-V` | Prints version. |

`--allow-write` only controls writes performed by Dobra code, such as `write(path, text)`,
`append(path, text)`, or `open(path, "write")`. CLI output redirection with `--out` is a CLI
feature and does not require `--allow-write`.

### `dobra run`

Executes an Dobra file.

```bash
dobra run file.dob
```

Example file:

```dobra
const name = input.name
emit "Hello, {name}"
```

Run with one variable:

```bash
dobra run hello.dob --var name=Ana
```

Output:

```text
Hello, Ana
```

Run with multiple variables:

```bash
dobra run hello.dob --vars name=Ana env=prod owner=gzeloni
```

Variables are exposed through the readonly `input` map:

```dobra
emit input.name
emit input.env
emit input.owner
```

Repeated `--var` is also valid:

```bash
dobra run hello.dob --var name=Ana --var env=prod
```

Run source from stdin with `-`:

```bash
printf 'emit "hello"\n' | dobra run -
```

Write the rendered program output to a file:

```bash
dobra run report.dob --out report.txt
dobra run report.dob --output report.txt
dobra run report.dob -o report.txt
```

If `--out` has no explicit path, Dobra writes beside the source path using `.out`:

```bash
dobra run report.dob --out
```

This writes to:

```text
report.dob.out
```

Run a script that writes files through the language:

```bash
dobra run transform.dob --allow-write
```

Without `--allow-write`, file-writing builtins fail with `E3001`.

`--stdout` is accepted by `run` as an explicit stdout target. It is equivalent to the default
behavior when `--out` is not used.

```bash
dobra run report.dob --stdout
```

### `dobra check`

Checks lexing, parsing, imports, and v0.4 semantic rules without executing the program.

```bash
dobra check file.dob
```

Output:

```text
ok file.dob
```

JSON success output:

```bash
dobra check file.dob --json
```

Output:

```json
{"ok":true,"errors":[]}
```

JSON failure output:

```json
{"ok":false,"errors":[{"code":"E4101","message":"cannot assign to const 'n'","file":"file.dob","line":2,"column":1}]}
```

`check` validates syntax and v0.4 semantic rules. It resolves imports for file-backed
programs, validates selected import names, catches undefined variables, rejects assignment to
`const`, validates basic arity, checks control-flow placement, and validates known map/namespace
fields. It does not execute program IO or prove static types/effects.

Example:

```dobra
import "./missing"

emit "syntax is valid"
```

`dobra check` reports missing imports, missing selected exports, and semantic errors before execution.

### `dobra fmt`

Formats `.dob` files using the canonical style.

Format one file:

```bash
dobra fmt file.dob
```

Format a directory recursively:

```bash
dobra fmt .
```

Check without writing changes:

```bash
dobra fmt --check .
```

Print formatted output to stdout:

```bash
dobra fmt --stdout file.dob
```

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

When formatting a directory, Dobra recursively formats `.dob` files and skips `target/`.

### `dobra eval`

Executes source passed on the command line.

```bash
dobra eval 'emit "hello"'
```

Output:

```text
hello
```

Use it for quick expressions or small scripts:

```bash
dobra eval 'emit upper("dobra")'
```

Output:

```text
DOBRA
```

`eval` can also write files if `--allow-write` is passed:

```bash
dobra eval 'write("out.txt", "ok")' --allow-write
```

### `dobra tokens`

Prints lexer tokens for a file. This is useful for editor tooling and parser debugging.

```bash
dobra tokens file.dob
```

Example output shape:

```text
1:1 Let
1:5 Identifier("name")
1:10 Equal
1:12 String("Ana")
```

JSON output:

```bash
dobra tokens file.dob --json
```

Output shape:

```json
{"ok":true,"tokens":[{"kind":"Let","literal":null,"line":1,"column":1}]}
```

### `dobra ast`

Prints the parsed AST for a file.

```bash
dobra ast file.dob
```

The default output is Rust debug text. JSON output wraps that debug representation:

```bash
dobra ast file.dob --json
```

Output shape:

```json
{"ok":true,"ast":"Program { ... }"}
```

The AST command is primarily a tooling/debug command.

### `dobra init`

Creates a minimal Dobra project.

```bash
dobra init
```

Generated layout:

```text
dobra.toml
src/
  main.dob
```

Generated `dobra.toml`:

```toml
name = "dobra-project"
entry = "src/main.dob"
```

Generated `src/main.dob`:

```dobra
const name = input.name

emit "Hello, {name}"
```

Create a project in another directory:

```bash
dobra init demo
```

`init` creates missing files but does not overwrite an existing `dobra.toml` or `src/main.dob`.

JSON output:

```bash
dobra init demo --json
```

Output shape:

```json
{"ok":true,"path":"demo"}
```

### `dobra version`

Prints the current version.

```bash
dobra version
```

Output:

```text
dobra 0.4.0
```

JSON output:

```bash
dobra version --json
```

Output:

```json
{"name":"dobra","version":"0.3.0","rust_std_only":true}
```

### `dobra help`

Prints command usage.

```bash
dobra help
dobra --help
dobra -h
```

## Projects

A project is discovered through `dobra.toml`.

```toml
name = "my-project"
entry = "src/main.dob"
```

If a command needs a file and no file is passed, Dobra searches from the current directory upward
for `dobra.toml` and uses its `entry` path.

Example:

```bash
mkdir demo
cd demo
dobra init
dobra run --var name=Project
```

Because no file path is passed, `dobra run` reads `entry = "src/main.dob"` from `dobra.toml`.

`dobra.toml` currently supports:

| Key | Meaning |
|---|---|
| `name` | Project name. |
| `entry` | Entry `.dob` file used when a command omits a file path. |

## Source Files

Source files use `.dob`.

```text
main.dob
lib/text.dob
showcase/index.dob
```

Statements do not require semicolons. Semicolons are accepted as statement separators, but the
formatter removes stylistic drift and writes canonical layout.

### Comments

Line comments can use `#` or `//`.

```dobra
# preferred for docs-like comments
// also accepted
emit "ok"
```

Block comments are not part of v0.4.

### Reserved Words

Current reserved words:

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

## Language Basics

### Values

Dobra has these runtime value categories:

| Value | Example |
|---|---|
| `null` | `null` |
| boolean | `true`, `false` |
| integer | `42` |
| float | `3.14` |
| string | `"hello"`, `'hello'` |
| list | `[1, 2, 3]` |
| map | `{name: "Ana", role: "dev"}` |
| stream | `open("file.txt", "read")`, `stdout` |
| function | `fn greet(name) { ... }` |

### Truthiness

Values used in `if`, `while`, `and`, `or`, or `not` follow truthiness rules.

| Value | Truthy? |
|---|---|
| `null` | false |
| `false` | false |
| `0` | false |
| `0.0` | false |
| empty string | false |
| empty list | false |
| empty map | false |
| streams | true |
| functions | true |
| non-empty values | true |

Example:

```dobra
if input.name {
  emit "name exists"
} else {
  emit "missing name"
}
```

### Variables

`const` declares a read-only binding.

```dobra
const app = "dobra"
emit app
```

`let` declares a mutable binding.

```dobra
let count = 0
count = count + 1
emit count
```

Assigning to a `const` is a runtime error:

```dobra
const count = 0
count = 1
```

Error:

```text
error[E2000]: cannot assign to const 'count'
```

### CLI Input

CLI variables are available through `input`.

Command:

```bash
dobra run app.dob --vars app=dobra env=prod
```

File:

```dobra
emit input.app
emit input.env
```

Output:

```text
dobra
prod
```

Variables passed with `--var` or inline `--vars` are strings. Flat JSON variable files can produce
strings, integers, floats, booleans, and `null`.

JSON variables file:

```json
{"app":"dobra","limit":3,"enabled":true}
```

Run:

```bash
dobra run app.dob --vars vars.json
```

YAML variables file support is intentionally flat and simple:

```yaml
app: dobra
env: prod
```

### Strings

Double-quoted strings:

```dobra
emit "hello"
```

Single-quoted strings:

```dobra
emit 'hello'
```

Escapes:

```dobra
emit "line 1\nline 2"
emit "tab\tvalue"
emit "quote: \""
emit "slash: \\"
```

Triple-quoted strings:

```dobra
const config = """
APP_NAME={input.app}
APP_ENV={input.env}
"""

emit config
```

### Interpolation

Strings support `{expr}` interpolation.

```dobra
const name = "Ana"
emit "Hello, {capitalize(name)}"
```

Interpolation can contain expressions:

```dobra
const a = 2
const b = 3
emit "sum={a + b}"
```

Output:

```text
sum=5
```

Escape literal braces with doubled braces:

```dobra
emit "{{value}}"
```

Output:

```text
{value}
```

### Output With `emit`

`emit` appends the value plus a newline to the program output.

```dobra
emit "one"
emit "two"
```

Output:

```text
one
two
```

`emit` is good for generated artifacts. For stream-style output, use `write(stdout, text)` or
`writeln(stdout, text)`.

### Operators

Arithmetic:

```dobra
emit 1 + 2
emit 5 - 3
emit 4 * 2
emit 8 / 2
emit 7 % 3
```

Comparison:

```dobra
emit 1 < 2
emit 1 <= 1
emit 2 > 1
emit 2 >= 2
```

Equality:

```dobra
emit "a" == "a"
emit "a" != "b"
```

Logical operators use words:

```dobra
emit true and not false
emit false or true
```

Use `not`, not `!`.

```dobra
if not input.disabled {
  emit "enabled"
}
```

### Conditionals

```dobra
if input.env == "prod" {
  emit "Production"
} else {
  emit "Development"
}
```

`else if` is supported by nesting an `if` after `else`:

```dobra
if input.env == "prod" {
  emit "prod"
} else if input.env == "stage" {
  emit "stage"
} else {
  emit "dev"
}
```

### Loops

For loop over a list:

```dobra
for name in ["ana", "bruno"] {
  emit capitalize(name)
}
```

For loop over a string iterates characters:

```dobra
for ch in "abc" {
  emit ch
}
```

For loop over a map iterates keys:

```dobra
const user = {name: "Ana", role: "dev"}

for key in user {
  emit "{key}={user[key]}"
}
```

While loop:

```dobra
let n = 0

while n < 3 {
  emit n
  n = n + 1
}
```

`break` exits a loop:

```dobra
for n in range(10) {
  if n == 3 {
    break
  }

  emit n
}
```

`continue` skips to the next iteration:

```dobra
for n in range(5) {
  if n == 2 {
    continue
  }

  emit n
}
```

`while` loops have a safety limit of 100000 iterations.

### Functions

Define a function:

```dobra
fn greet(name) {
  return "Hello, {capitalize(name)}"
}

emit greet("ana")
```

Functions return `null` when they finish without `return`.

```dobra
fn noop() {}

emit noop()
```

Output:

```text
null
```

Return without a value returns `null`:

```dobra
fn stop() {
  return
}
```

### Lists

Inline list:

```dobra
const tags = ["compiler", "formatter", "streams"]
emit tags[0]
```

Multiline list:

```dobra
const tags = [
  "compiler",
  "formatter",
  "streams",
]
```

List indexing is zero-based. Negative list indexes count from the end:

```dobra
const tags = ["a", "b", "c"]
emit tags[-1]
```

Output:

```text
c
```

Lists are values. List helper functions return new lists instead of mutating in place:

```dobra
let values = []
values = push(values, "a")
values = push(values, "b")
emit values
```

Output:

```text
[a, b]
```

### Maps

Inline map:

```dobra
const user = {name: "Ana", role: "dev"}
```

Canonical formatted map:

```dobra
const user = {
  name: "Ana",
  role: "dev",
}
```

Field access:

```dobra
emit user.name
```

Index access:

```dobra
emit user["role"]
```

Map keys can be identifiers or strings:

```dobra
const data = {
  name: "Ana",
  "full name": "Ana Maria",
}

emit data["full name"]
```

### Function Calls

Short calls stay inline:

```dobra
emit join(["a", "b"], ":")
```

Long calls are formatted across lines:

```dobra
emit replace(
  "cobalt/mythril/adamantite",
  "/",
  " -> ",
)
```

Dobra does not use method calls for standard library functions. Prefer function style:

```dobra
const values = push([], "item")
```

## Imports

Imports are relative to the importing file.

```dobra
import "./lib/constants"
```

The `.dob` extension is optional:

```dobra
import "./lib/constants"
import "./lib/constants.dob"
```

Directories resolve through `index.dob`:

```text
lib/
  index.dob
```

```dobra
import "./lib" as lib
```

### Namespace Imports

```dobra
import "./lib/meta" as meta

emit meta.title
emit meta.version
```

### Direct Imports

```dobra
import "./lib/meta" show title, version

emit title
emit version
```

### Hide Clause

```dobra
import "./lib/meta" hide internal_token
```

### Import Mutability

Imported `const` and `fn` bindings are read-only. Imported `let` bindings remain mutable.

`counter.dob`:

```dobra
let n = 0
```

`main.dob`:

```dobra
import "./counter" show n

while n < 3 {
  emit n
  n = n + 1
}
```

Output:

```text
0
1
2
```

### Circular Imports

Circular imports are allowed. Modules are cached by resolved path and bindings are linked lazily.
A cycle fails only if code reads a binding before it has been initialized.

`a.dob`:

```dobra
import "./b" as b

const name = "A"

fn pair() {
  return "{name}/{b.name}"
}
```

`b.dob`:

```dobra
import "./a" as a

const name = "B"

fn pair() {
  return "{name}/{a.name}"
}
```

`main.dob`:

```dobra
import "./a" as a
import "./b" as b

emit a.pair()
emit b.pair()
```

Output:

```text
A/B
B/A
```

## IO And Streams

Dobra v0.4 has real file IO and stream values.

### Standard Streams

| Binding | Meaning |
|---|---|
| `stdin` | standard input stream |
| `stdout` | program output stream |
| `stderr` | process standard error stream |

Example:

```dobra
writeln(stdout, "What is your name?")
const name = readln(stdin)
writeln(stdout, "Hello, {name}")
```

### File Paths

Import paths are relative to the importing source file. File IO paths are resolved by the current
working directory of the `dobra` process.

Example:

```bash
cd demo
dobra run scripts/build.dob --allow-write
```

Inside `build.dob`, this writes `demo/out.txt`:

```dobra
write("out.txt", "ok")
```

### `open(path, mode)`

Opens a file stream.

Modes:

| Mode | Meaning |
|---|---|
| `read` | open existing file for reading |
| `write` | create/truncate file for writing |
| `append` | create/open file and append writes |

Read mode:

```dobra
const file = open("input.txt", "read")
const text = read(file)
close(file)
emit text
```

Write mode:

```dobra
const file = open("output.txt", "write")
writeln(file, "first")
writeln(file, "second")
close(file)
```

Run with permission:

```bash
dobra run write.dob --allow-write
```

Append mode:

```dobra
const log = open("app.log", "append")
writeln(log, "started")
close(log)
```

### `close(stream)`

Closes a stream. Closing a file writer also flushes pending writes.

```dobra
const out = open("out.txt", "write")
write(out, "ok")
close(out)
```

Closing `stdin`, `stdout`, or `stderr` is accepted as a no-op or flush-equivalent operation.

### `flush(stream)`

Flushes pending writes.

```dobra
const out = open("out.txt", "write")
write(out, "partial")
flush(out)
close(out)
```

`flush` expects a writable stream.

### `read(path)`

Reads a whole file into a string.

```dobra
const text = read("input.txt")
emit upper(text)
```

This does not require `--allow-write`.

### `read(stream)`

Reads the rest of a readable stream.

```dobra
const src = open("input.txt", "read")
const text = read(src)
close(src)
emit text
```

### `read(stream, size)`

Reads a chunk from a readable stream. `size` is a non-negative integer byte count.

```dobra
const src = open("input.txt", "read")
emit read(src, 8)
emit read(src, 8)
close(src)
```

### `readln(stream)`

Reads one line and strips the line ending. Returns `null` at EOF.

```dobra
const src = open("input.txt", "read")

let line = readln(src)
while line != null {
  emit line
  line = readln(src)
}

close(src)
```

### `write(path, text)`

Writes a whole file, replacing any previous content.

```dobra
write("out.txt", "hello\n")
```

Requires:

```bash
dobra run script.dob --allow-write
```

### `write(stream, text)`

Writes text to a stream without adding a newline.

```dobra
const out = open("out.txt", "write")
write(out, "hello")
write(out, " world")
close(out)
```

`write(stdout, text)` writes to the program output:

```dobra
write(stdout, "hello")
write(stdout, " world")
```

### `writeln(stream, text)`

Writes text and a newline to a stream.

```dobra
const out = open("out.txt", "write")
writeln(out, "hello")
writeln(out, "world")
close(out)
```

### `append(path, text)`

Appends text to a file.

```dobra
append("app.log", "started\n")
```

Requires `--allow-write`.

### `eof(stream)`

Returns whether a readable file stream has reached EOF. EOF becomes true after a read operation
reaches the end.

```dobra
const src = open("input.txt", "read")

while not eof(src) {
  const chunk = read(src, 16)
  if chunk != "" {
    emit chunk
  }
}

close(src)
```

For line-oriented code, prefer the simpler `readln(stream) != null` style:

```dobra
let line = readln(src)
while line != null {
  emit line
  line = readln(src)
}
```

## Standard Library

Builtin names are short, technical, and predictable. The simplicity is in syntax and canonical
formatting, not in overly humanized names.

Legacy aliases `uppercase`, `lowercase`, `starts_with`, and `ends_with` are accepted for now, but
new code should use `upper`, `lower`, `starts`, and `ends`.

### Text Builtins

#### `upper(text)`

```dobra
emit upper("dobra")
```

Output:

```text
DOBRA
```

#### `lower(text)`

```dobra
emit lower("DOBRA")
```

Output:

```text
dobra
```

#### `capitalize(text)`

```dobra
emit capitalize("gZELONI")
```

Output:

```text
Gzeloni
```

#### `trim(text)`

```dobra
emit "'{trim('  value  ')}'"
```

Output:

```text
'value'
```

#### `replace(text, from, to)`

```dobra
emit replace("a/b/c", "/", " -> ")
```

Output:

```text
a -> b -> c
```

#### `split(text, sep)`

```dobra
emit split("a,b,c", ",")
```

Output:

```text
[a, b, c]
```

#### `join(list, sep)`

```dobra
emit join(["a", "b", "c"], "|")
```

Output:

```text
a|b|c
```

#### `lines(text)`

```dobra
emit lines("a\nb\nc")
```

Output:

```text
[a, b, c]
```

#### `unlines(list)`

```dobra
emit unlines(["a", "b", "c"])
```

Output:

```text
a
b
c
```

#### `words(text)`

```dobra
emit words("terra blade true night edge")
```

Output:

```text
[terra, blade, true, night, edge]
```

#### `contains(value, needle)`

Strings:

```dobra
emit contains("adamantite", "mant")
```

Lists:

```dobra
emit contains(["compiler", "streams"], "streams")
```

Maps check keys:

```dobra
emit contains({name: "Ana"}, "name")
```

#### `starts(text, prefix)`

```dobra
emit starts("adamantite", "ada")
```

Output:

```text
true
```

#### `ends(text, suffix)`

```dobra
emit ends("adamantite", "ite")
```

Output:

```text
true
```

#### `indent(text, spaces_or_prefix)`

Indent with spaces:

```dobra
emit indent("a\nb", 2)
```

Output:

```text
  a
  b
```

Indent with a prefix:

```dobra
emit indent("a\nb", "> ")
```

Output:

```text
> a
> b
```

#### `dedent(text)`

```dobra
const text = """
    a
    b
"""

emit dedent(text)
```

### Number Builtins

#### `int(value)`

```dobra
emit int("42")
emit int(3.9)
```

Output:

```text
42
3
```

#### `float(value)`

```dobra
emit float("42")
```

Output:

```text
42.0
```

#### `abs(n)`

```dobra
emit abs(-10)
```

Output:

```text
10
```

#### `floor(n)`

```dobra
emit floor(3.9)
```

Output:

```text
3
```

#### `ceil(n)`

```dobra
emit ceil(3.1)
```

Output:

```text
4
```

#### `round(n)`

```dobra
emit round(3.5)
```

Output:

```text
4
```

#### `sqrt(n)`

```dobra
emit sqrt(9)
```

Output:

```text
3.0
```

#### `pow(a, b)`

```dobra
emit pow(2, 8)
```

Output:

```text
256
```

#### `min(a, b)`

```dobra
emit min(10, 3)
```

Output:

```text
3
```

#### `max(a, b)`

```dobra
emit max(10, 3)
```

Output:

```text
10
```

#### `clamp(n, min, max)`

```dobra
emit clamp(12, 0, 10)
emit clamp(-1, 0, 10)
emit clamp(5, 0, 10)
```

Output:

```text
10
0
5
```

#### `sum(list)`

```dobra
emit sum([1, 2, 3])
```

Output:

```text
6
```

#### `avg(list)`

```dobra
emit avg([1, 2, 3])
emit avg([])
```

Output:

```text
2.0
null
```

#### `range(end)` and `range(start, end)`

```dobra
emit range(4)
emit range(2, 5)
emit range(5, 2)
```

Output:

```text
[0, 1, 2, 3]
[2, 3, 4]
[5, 4, 3]
```

The end value is excluded.

### Data And Conversion Builtins

#### `len(value)`

```dobra
emit len("abc")
emit len([1, 2, 3])
emit len({name: "Ana"})
```

Output:

```text
3
3
1
```

#### `string(value)`

```dobra
emit string(42)
emit string(true)
```

Output:

```text
42
true
```

#### `bool(value)`

```dobra
emit bool(null)
emit bool(1)
emit bool("text")
```

Output:

```text
false
true
true
```

#### `keys(map)`

```dobra
emit keys({name: "Ana", role: "dev"})
```

Output:

```text
[name, role]
```

Map keys are stored in deterministic sorted order.

#### `values(map)`

```dobra
emit values({name: "Ana", role: "dev"})
```

Output:

```text
[Ana, dev]
```

#### `push(list, value)`

```dobra
emit push([1, 2], 3)
```

Output:

```text
[1, 2, 3]
```

#### `pop(list)`

```dobra
emit pop([1, 2, 3])
emit pop([])
```

Output:

```text
[1, 2]
[]
```

#### `first(list)`

```dobra
emit first(["a", "b"])
emit first([])
```

Output:

```text
a
null
```

#### `last(list)`

```dobra
emit last(["a", "b"])
emit last([])
```

Output:

```text
b
null
```

#### `slice(list_or_text, start, end)`

Lists:

```dobra
emit slice(["a", "b", "c", "d"], 1, 3)
```

Output:

```text
[b, c]
```

Text:

```dobra
emit slice("dobra", 1, 4)
```

Output:

```text
ric
```

Negative indexes count from the end:

```dobra
emit slice(["a", "b", "c", "d"], -3, -1)
```

Output:

```text
[b, c]
```

#### `reverse(list_or_text)`

```dobra
emit reverse([1, 2, 3])
emit reverse("abc")
```

Output:

```text
[3, 2, 1]
cba
```

#### `sort(list)`

```dobra
emit sort([3, 1, 2])
emit sort(["c", "a", "b"])
```

Output:

```text
[1, 2, 3]
[a, b, c]
```

#### `unique(list)`

```dobra
emit unique(["a", "b", "a", "c", "b"])
```

Output:

```text
[a, b, c]
```

## Diagnostics

Language/runtime errors use exit code `1`.

Example:

```dobra
const n = 1
n = 2
```

Output:

```text
error[E4101]: cannot assign to const 'n'
  at file.dob:2:1
```

Parse errors use `E1000`, runtime errors use `E2000`, IO errors use `E3000`, and semantic checker errors use `E41xx`. Write permission errors use `E3001`.

Write permission error:

```dobra
write("out.txt", "blocked")
```

Command:

```bash
dobra run file.dob
```

Output:

```text
error[E3001]: file write requires --allow-write
  at file.dob
```

JSON error output:

```bash
dobra run file.dob --json
```

Shape:

```json
{"ok":false,"error":{"message":"error[E3001]: file write requires --allow-write\n  at file.dob","exit_code":1}}
```

Exit codes:

| Code | Meaning |
|---:|---|
| `0` | Success. |
| `1` | Language/runtime error. |
| `2` | Invalid CLI usage. |
| `3` | CLI IO error. |
| `4` | Internal error, reserved. |

## Formatting Contract

Formatting is canonical and non-configurable.

| Rule | Style |
|---|---|
| Indent | 2 spaces |
| Braces | opening brace on the same line |
| Operators | spaces around binary operators |
| Blocks | always use `{}` |
| Maps | non-empty maps are multi-line |
| Lists/calls | inline when short, multi-line when long |
| Final newline | required |
| Line width | formatter-controlled lines target 60 characters |

The formatter is part of the language contract. Prefer writing clear code and letting `dobra fmt`
settle layout.

## VSCode Support

Local syntax highlighting is available in:

```text
vscode/dobra-language
```

Install from VSCode:

```text
Developer: Install Extension from Location...
```

Select the `vscode/dobra-language` folder.

The extension supports `.dob` file association, keywords, strings, interpolation, comments,
numbers, operators, builtins, and language globals.

## Complete Examples

### Fibonacci With Lists

```dobra
fn fibs(count) {
  let result = []
  let a = 0
  let b = 1

  for i in range(count) {
    result = push(result, a)

    let next = a + b
    a = b
    b = next
  }

  return result
}

emit fibs(10)
emit sum(fibs(10))
emit avg(fibs(10))
```

Output:

```text
[0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
88
8.8
```

### File Transformer

`upper_file.dob`:

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

Run:

```bash
dobra run upper_file.dob --allow-write
```

### Generated Report With Imports

`report/meta.dob`:

```dobra
const title = "Build Report"
const sections = ["summary", "artifacts", "status"]
```

`report/format.dob`:

```dobra
fn heading(text) {
  return "== {upper(text)} =="
}

fn bullet(value) {
  return "- {value}"
}
```

`main.dob`:

```dobra
import "./report/meta" as meta
import "./report/format" as fmt

emit fmt.heading(meta.title)

for section in meta.sections {
  emit fmt.bullet(section)
}
```

Output:

```text
== BUILD REPORT ==
- summary
- artifacts
- status
```
