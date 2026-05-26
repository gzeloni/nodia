# Nodia Reference v0.6

This is the complete user-facing reference for Nodia v0.6. It documents the command line,
project layout, language syntax, uses, IO, streams, standard library, and common workflows.
The v0.6 implementation extends the v0.5 baseline with a native regex DSL.

Nodia source files use the `.nod` extension.

## Table Of Contents

- [Install And Build](#install-and-build)
- [Command Line](#command-line)
- [Projects](#projects)
- [Source Files](#source-files)
- [Language Basics](#language-basics)
- [Regex DSL](#regex-dsl)
- [Uses](#uses)
- [IO And Streams](#io-and-streams)
- [Standard Library](#standard-library)
- [Diagnostics](#diagnostics)
- [Formatting Contract](#formatting-contract)
- [VSCode Support](#vscode-support)
- [Complete Examples](#complete-examples)

## Install And Build

Nodia is implemented in Rust and currently uses only the Rust standard library.

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
target/release/nodia version
```

Expected output:

```text
nodia 0.6.0
```

## Command Line

General shape:

```bash
nodia [global-flags] <command> [command-args]
nodia <command> [command-args] [global-flags]
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
| `--allow-write` | Allows Nodia code to write files through IO builtins. |
| `--help`, `-h` | Prints help. |
| `--version`, `-V` | Prints version. |

`--allow-write` only controls writes performed by Nodia code, such as `write(path, text)`,
`append(path, text)`, or `open(path, "write")`. CLI output redirection with `--out` is a CLI
feature and does not require `--allow-write`.

### `nodia run`

Executes an Nodia file.

```bash
nodia run file.nod
```

Example file:

```nodia
val name = input.name
emit "Hello, {name}"
```

Run with one variable:

```bash
nodia run hello.nod --var name=Ana
```

Output:

```text
Hello, Ana
```

Run with multiple variables:

```bash
nodia run hello.nod --vars name=Ana env=prod owner=gzeloni
```

Variables are exposed through the readonly `input` map:

```nodia
emit input.name
emit input.env
emit input.owner
```

Repeated `--var` is also valid:

```bash
nodia run hello.nod --var name=Ana --var env=prod
```

Run source from stdin with `-`:

```bash
printf 'emit "hello"\n' | nodia run -
```

Write the rendered program output to a file:

```bash
nodia run report.nod --out report.txt
nodia run report.nod --output report.txt
nodia run report.nod -o report.txt
```

If `--out` has no explicit path, Nodia writes beside the source path using `.out`:

```bash
nodia run report.nod --out
```

This writes to:

```text
report.nod.out
```

Run a script that writes files through the language:

```bash
nodia run transform.nod --allow-write
```

Without `--allow-write`, file-writing builtins fail with `E3001`.

`--stdout` is accepted by `run` as an explicit stdout target. It is equivalent to the default
behavior when `--out` is not used.

```bash
nodia run report.nod --stdout
```

### `nodia check`

Checks lexing, parsing, uses, regex DSL structure, and the v0.5 semantic baseline without executing the program.

```bash
nodia check file.nod
```

Output:

```text
ok file.nod
```

JSON success output:

```bash
nodia check file.nod --json
```

Output:

```json
{"ok":true,"errors":[]}
```

JSON failure output:

```json
{"ok":false,"errors":[{"code":"E4101","message":"cannot assign to val 'n'","file":"file.nod","line":2,"column":1}]}
```

`check` validates syntax, regex DSL structure, and the v0.5 semantic baseline. It resolves uses for file-backed
programs, validates selected use names, catches undefined variables, rejects assignment to
`val`, validates basic arity, checks control-flow placement, and validates known map/namespace
fields. It does not execute program IO or prove static types/effects.

Example:

```nodia
use "./missing"

emit "syntax is valid"
```

`nodia check` reports missing uses, missing selected exports, and semantic errors before execution.

### `nodia fmt`

Formats `.nod` files using the canonical style.

Format one file:

```bash
nodia fmt file.nod
```

Format a directory recursively:

```bash
nodia fmt .
```

Check without writing changes:

```bash
nodia fmt --check .
```

Print formatted output to stdout:

```bash
nodia fmt --stdout file.nod
```

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

When formatting a directory, Nodia recursively formats `.nod` files and skips `target/`.

### `nodia eval`

Executes source passed on the command line.

```bash
nodia eval 'emit "hello"'
```

Output:

```text
hello
```

Use it for quick expressions or small scripts:

```bash
nodia eval 'emit upper("nodia")'
```

Output:

```text
DOBRA
```

`eval` can also write files if `--allow-write` is passed:

```bash
nodia eval 'write("out.txt", "ok")' --allow-write
```

### `nodia tokens`

Prints lexer tokens for a file. This is useful for editor tooling and parser debugging.

```bash
nodia tokens file.nod
```

Example output shape:

```text
1:1 Val
1:5 Identifier("name")
1:10 Equal
1:12 String("Ana")
```

JSON output:

```bash
nodia tokens file.nod --json
```

Output shape:

```json
{"ok":true,"tokens":[{"kind":"Val","literal":null,"line":1,"column":1}]}
```

### `nodia ast`

Prints the parsed AST for a file.

```bash
nodia ast file.nod
```

The default output is Rust debug text. JSON output wraps that debug representation:

```bash
nodia ast file.nod --json
```

Output shape:

```json
{"ok":true,"ast":"Program { ... }"}
```

The AST command is primarily a tooling/debug command.

### `nodia init`

Creates a minimal Nodia project.

```bash
nodia init
```

Generated layout:

```text
nodia.toml
src/
  main.nod
```

Generated `nodia.toml`:

```toml
name = "nodia-project"
entry = "src/main.nod"
```

Generated `src/main.nod`:

```nodia
val name = input.name

emit "Hello, {name}"
```

Create a project in another directory:

```bash
nodia init demo
```

`init` creates missing files but does not overwrite an existing `nodia.toml` or `src/main.nod`.

JSON output:

```bash
nodia init demo --json
```

Output shape:

```json
{"ok":true,"path":"demo"}
```

### `nodia version`

Prints the current version.

```bash
nodia version
```

Output:

```text
nodia 0.6.0
```

JSON output:

```bash
nodia version --json
```

Output:

```json
{"name":"nodia","version":"0.6.0","rust_std_only":true}
```

### `nodia help`

Prints command usage.

```bash
nodia help
nodia --help
nodia -h
```

## Projects

A project is discovered through `nodia.toml`.

```toml
name = "my-project"
entry = "src/main.nod"
```

If a command needs a file and no file is passed, Nodia searches from the current directory upward
for `nodia.toml` and uses its `entry` path.

Example:

```bash
mkdir demo
cd demo
nodia init
nodia run --var name=Project
```

Because no file path is passed, `nodia run` reads `entry = "src/main.nod"` from `nodia.toml`.

`nodia.toml` currently supports:

| Key | Meaning |
|---|---|
| `name` | Project name. |
| `entry` | Entry `.nod` file used when a command omits a file path. |

## Source Files

Source files use `.nod`.

```text
main.nod
lib/text.nod
showcase/index.nod
```

Statements do not require semicolons. Semicolons are accepted as statement separators, but the
formatter removes stylistic drift and writes canonical layout.

### Comments

Line comments can use `#` or `//`.

```nodia
# preferred for docs-like comments
// also accepted
emit "ok"
```

Block comments are not part of v0.5.

### Reserved Words

Current reserved words:

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

## Language Basics

### Values

Nodia has these runtime value categories:

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
| function | `func greet(name) { ... }` |

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

```nodia
if input.name {
  emit "name exists"
} else {
  emit "missing name"
}
```

### Variables

`val` declares a read-only binding.

```nodia
val app = "nodia"
emit app
```

`var` declares a mutable binding.

```nodia
var count = 0
count = count + 1
emit count
```

Assigning to a `val` is a runtime error:

```nodia
val count = 0
count = 1
```

Error:

```text
error[E2000]: cannot assign to val 'count'
```

### CLI Input

CLI variables are available through `input`.

Command:

```bash
nodia run app.nod --vars app=nodia env=prod
```

File:

```nodia
emit input.app
emit input.env
```

Output:

```text
nodia
prod
```

Variables passed with `--var` or inline `--vars` are strings. Flat JSON variable files can produce
strings, integers, floats, booleans, and `null`.

JSON variables file:

```json
{"app":"nodia","limit":3,"enabled":true}
```

Run:

```bash
nodia run app.nod --vars vars.json
```

YAML variables file support is intentionally flat and simple:

```yaml
app: nodia
env: prod
```

### Strings

Double-quoted strings:

```nodia
emit "hello"
```

Single-quoted strings:

```nodia
emit 'hello'
```

Escapes:

```nodia
emit "line 1\nline 2"
emit "tab\tvalue"
emit "quote: \""
emit "slash: \\"
```

Triple-quoted strings:

```nodia
val config = """
APP_NAME={input.app}
APP_ENV={input.env}
"""

emit config
```

### Interpolation

Strings support `{expr}` interpolation.

```nodia
val name = "Ana"
emit "Hello, {capitalize(name)}"
```

Interpolation can contain expressions:

```nodia
val a = 2
val b = 3
emit "sum={a + b}"
```

Output:

```text
sum=5
```

Escape literal braces with doubled braces:

```nodia
emit "{{value}}"
```

Output:

```text
{value}
```

### Output With `emit`

`emit` appends the value plus a newline to the program output.

```nodia
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

```nodia
emit 1 + 2
emit 5 - 3
emit 4 * 2
emit 8 / 2
emit 7 % 3
```

Comparison:

```nodia
emit 1 < 2
emit 1 <= 1
emit 2 > 1
emit 2 >= 2
```

Equality:

```nodia
emit "a" == "a"
emit "a" != "b"
```

Logical operators use words:

```nodia
emit true and not false
emit false or true
```

Use `not`, not `!`.

```nodia
if not input.disabled {
  emit "enabled"
}
```

### Conditionals

```nodia
if input.env == "prod" {
  emit "Production"
} else {
  emit "Development"
}
```

`else if` is supported by nesting an `if` after `else`:

```nodia
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

```nodia
for name in ["ana", "bruno"] {
  emit capitalize(name)
}
```

For loop over a string iterates characters:

```nodia
for ch in "abc" {
  emit ch
}
```

For loop over a map iterates keys:

```nodia
val user = {name: "Ana", role: "dev"}

for key in user {
  emit "{key}={user[key]}"
}
```

While loop:

```nodia
var n = 0

while n < 3 {
  emit n
  n = n + 1
}
```

`break` exits a loop:

```nodia
for n in range(10) {
  if n == 3 {
    break
  }

  emit n
}
```

`continue` skips to the next iteration:

```nodia
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

```nodia
func greet(name) {
  return "Hello, {capitalize(name)}"
}

emit greet("ana")
```

Functions return `null` when they finish without `return`.

```nodia
func noop() {}

emit noop()
```

Output:

```text
null
```

Return without a value returns `null`:

```nodia
func stop() {
  return
}
```

### Lists

Inline list:

```nodia
val tags = ["compiler", "formatter", "streams"]
emit tags[0]
```

Multiline list:

```nodia
val tags = [
  "compiler",
  "formatter",
  "streams",
]
```

List indexing is zero-based. Negative list indexes count from the end:

```nodia
val tags = ["a", "b", "c"]
emit tags[-1]
```

Output:

```text
c
```

Lists are values. List helper functions return new lists instead of mutating in place:

```nodia
var values = []
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

```nodia
val user = {name: "Ana", role: "dev"}
```

Canonical formatted map:

```nodia
val user = {
  name: "Ana",
  role: "dev",
}
```

Field access:

```nodia
emit user.name
```

Index access:

```nodia
emit user["role"]
```

Map keys can be identifiers or strings:

```nodia
val data = {
  name: "Ana",
  "full name": "Ana Maria",
}

emit data["full name"]
```

### Function Calls

Short calls stay inline:

```nodia
emit join(["a", "b"], ":")
```

Long calls are formatted across lines:

```nodia
emit replace(
  "cobalt/mythril/adamantite",
  "/",
  " -> ",
)
```

Nodia does not use method calls for standard library functions. Prefer function style:

```nodia
val values = push([], "item")
```

## Regex DSL

Nodia v0.6 adds `regex { ... }` as a native expression. It evaluates to a regex value in the runtime. When emitted, interpolated, or converted with `string(...)`, it renders to classic regex text.

Example:

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

The first v0.6 regex surface supports:

- readable tokens such as `digit`, `whitespace`, `word_boundary`, and `any_char`
- `any_codepoint` for an explicit "match including newlines" node
- quantifiers such as `optional`, `one_or_more`, `exactly`, `at_least`, and `between`
- groups such as `group`, `non_capture`, `named`, and `atomic`
- alternation with `either { branch { ... } }`
- character sets with `char_set`, `not_char_set`, and `range`
- lookarounds with `followed_by`, `not_followed_by`, `preceded_by`, and `not_preceded_by`
- backreferences with `same_as` and `same_as_group`
- scoped flag blocks with `with_flags(...) { ... }` and `without_flags(...) { ... }`
- explicit literal helpers such as `literal("...")` and `char("x")`
- `raw_regex "..."` as an escape hatch

Regex execution uses function style, not methods:

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
emit len(find_all("http://a https://b", url))
```

`find(text, pattern)` returns `null` when there is no match. When a match exists, it returns a map with this shape:

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

`start` and `end` use character offsets, so they align with Nodia string indexing and `slice(...)`.

## Uses

Uses are relative to the source file containing the use.

```nodia
use "./lib/constants"
```

The `.nod` extension is optional:

```nodia
use "./lib/constants"
use "./lib/constants.nod"
```

Directories resolve through `index.nod`:

```text
lib/
  index.nod
```

```nodia
use "./lib" as lib
```

### Namespace Uses

```nodia
use "./lib/meta" as meta

emit meta.title
emit meta.version
```

### Direct Uses

```nodia
use "./lib/meta" pick title, version

emit title
emit version
```

### Hide Clause

```nodia
use "./lib/meta" hide internal_token
```

### Use Mutability

Used `val` and `func` bindings are read-only. Used `var` bindings remain mutable.

`counter.nod`:

```nodia
var n = 0
```

`main.nod`:

```nodia
use "./counter" pick n

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

### Circular Uses

Circular uses are allowed. Modules are cached by resolved path and bindings are linked lazily.
A cycle fails only if code reads a binding before it has been initialized.

`a.nod`:

```nodia
use "./b" as b

val name = "A"

func pair() {
  return "{name}/{b.name}"
}
```

`b.nod`:

```nodia
use "./a" as a

val name = "B"

func pair() {
  return "{name}/{a.name}"
}
```

`main.nod`:

```nodia
use "./a" as a
use "./b" as b

emit a.pair()
emit b.pair()
```

Output:

```text
A/B
B/A
```

## IO And Streams

Nodia v0.5 has real file IO and stream values.

### Standard Streams

| Binding | Meaning |
|---|---|
| `stdin` | standard input stream |
| `stdout` | program output stream |
| `stderr` | process standard error stream |

Example:

```nodia
writeln(stdout, "What is your name?")
val name = readln(stdin)
writeln(stdout, "Hello, {name}")
```

### File Paths

Use paths are relative to the source file containing the use. File IO paths are resolved by the current
working directory of the `nodia` process.

Example:

```bash
cd demo
nodia run scripts/build.nod --allow-write
```

Inside `build.nod`, this writes `demo/out.txt`:

```nodia
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

```nodia
val file = open("input.txt", "read")
val text = read(file)
close(file)
emit text
```

Write mode:

```nodia
val file = open("output.txt", "write")
writeln(file, "first")
writeln(file, "second")
close(file)
```

Run with permission:

```bash
nodia run write.nod --allow-write
```

Append mode:

```nodia
val log = open("app.log", "append")
writeln(log, "started")
close(log)
```

### `close(stream)`

Closes a stream. Closing a file writer also flushes pending writes.

```nodia
val out = open("out.txt", "write")
write(out, "ok")
close(out)
```

Closing `stdin`, `stdout`, or `stderr` is accepted as a no-op or flush-equivalent operation.

### `flush(stream)`

Flushes pending writes.

```nodia
val out = open("out.txt", "write")
write(out, "partial")
flush(out)
close(out)
```

`flush` expects a writable stream.

### `read(path)`

Reads a whole file into a string.

```nodia
val text = read("input.txt")
emit upper(text)
```

This does not require `--allow-write`.

### `read(stream)`

Reads the rest of a readable stream.

```nodia
val src = open("input.txt", "read")
val text = read(src)
close(src)
emit text
```

### `read(stream, size)`

Reads a chunk from a readable stream. `size` is a non-negative integer byte count.

```nodia
val src = open("input.txt", "read")
emit read(src, 8)
emit read(src, 8)
close(src)
```

### `readln(stream)`

Reads one line and strips the line ending. Returns `null` at EOF.

```nodia
val src = open("input.txt", "read")

var line = readln(src)
while line != null {
  emit line
  line = readln(src)
}

close(src)
```

### `write(path, text)`

Writes a whole file, replacing any previous content.

```nodia
write("out.txt", "hello\n")
```

Requires:

```bash
nodia run script.nod --allow-write
```

### `write(stream, text)`

Writes text to a stream without adding a newline.

```nodia
val out = open("out.txt", "write")
write(out, "hello")
write(out, " world")
close(out)
```

`write(stdout, text)` writes to the program output:

```nodia
write(stdout, "hello")
write(stdout, " world")
```

### `writeln(stream, text)`

Writes text and a newline to a stream.

```nodia
val out = open("out.txt", "write")
writeln(out, "hello")
writeln(out, "world")
close(out)
```

### `append(path, text)`

Appends text to a file.

```nodia
append("app.log", "started\n")
```

Requires `--allow-write`.

### `eof(stream)`

Returns whether a readable file stream has reached EOF. EOF becomes true after a read operation
reaches the end.

```nodia
val src = open("input.txt", "read")

while not eof(src) {
  val chunk = read(src, 16)
  if chunk != "" {
    emit chunk
  }
}

close(src)
```

For line-oriented code, prefer the simpler `readln(stream) != null` style:

```nodia
var line = readln(src)
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

```nodia
emit upper("nodia")
```

Output:

```text
DOBRA
```

#### `lower(text)`

```nodia
emit lower("DOBRA")
```

Output:

```text
nodia
```

#### `capitalize(text)`

```nodia
emit capitalize("gZELONI")
```

Output:

```text
Gzeloni
```

#### `trim(text)`

```nodia
emit "'{trim('  value  ')}'"
```

Output:

```text
'value'
```

#### `replace(text, from, to)`

```nodia
emit replace("a/b/c", "/", " -> ")
```

Output:

```text
a -> b -> c
```

#### `split(text, sep)`

```nodia
emit split("a,b,c", ",")
```

Output:

```text
[a, b, c]
```

#### `join(list, sep)`

```nodia
emit join(["a", "b", "c"], "|")
```

Output:

```text
a|b|c
```

#### `lines(text)`

```nodia
emit lines("a\nb\nc")
```

Output:

```text
[a, b, c]
```

#### `unlines(list)`

```nodia
emit unlines(["a", "b", "c"])
```

Output:

```text
a
b
c
```

#### `words(text)`

```nodia
emit words("terra blade true night edge")
```

Output:

```text
[terra, blade, true, night, edge]
```

#### `contains(value, needle)`

Strings:

```nodia
emit contains("adamantite", "mant")
```

Lists:

```nodia
emit contains(["compiler", "streams"], "streams")
```

Maps check keys:

```nodia
emit contains({name: "Ana"}, "name")
```

#### `starts(text, prefix)`

```nodia
emit starts("adamantite", "ada")
```

Output:

```text
true
```

#### `ends(text, suffix)`

```nodia
emit ends("adamantite", "ite")
```

Output:

```text
true
```

### Regex Builtins

#### `test(text, pattern)`

Returns `true` when the pattern matches anywhere inside `text`.

```nodia
emit test("go to https://example.com now", regex {
  "https://"
  one_or_more letter
})
```

#### `full_match(text, pattern)`

Returns `true` only when the entire text matches the pattern.

```nodia
emit full_match("abc-42", "^[a-z]+-\\d+$")
```

#### `find(text, pattern)`

Returns the first match as a map, or `null` when nothing matches.

```nodia
val hit = find("go to https://example.com now", regex {
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
})

emit hit.named.host
emit hit.start
emit hit.end
```

#### `find_all(text, pattern)`

Returns a list of all non-overlapping matches.

```nodia
emit len(find_all("http://a https://b", regex {
  either {
    branch {
      "http"
    }
    branch {
      "https"
    }
  }
  "://"
  one_or_more letter
}))
```

#### `indent(text, spaces_or_prefix)`

Indent with spaces:

```nodia
emit indent("a\nb", 2)
```

Output:

```text
  a
  b
```

Indent with a prefix:

```nodia
emit indent("a\nb", "> ")
```

Output:

```text
> a
> b
```

#### `dedent(text)`

```nodia
val text = """
    a
    b
"""

emit dedent(text)
```

### Number Builtins

#### `int(value)`

```nodia
emit int("42")
emit int(3.9)
```

Output:

```text
42
3
```

#### `float(value)`

```nodia
emit float("42")
```

Output:

```text
42.0
```

#### `abs(n)`

```nodia
emit abs(-10)
```

Output:

```text
10
```

#### `floor(n)`

```nodia
emit floor(3.9)
```

Output:

```text
3
```

#### `ceil(n)`

```nodia
emit ceil(3.1)
```

Output:

```text
4
```

#### `round(n)`

```nodia
emit round(3.5)
```

Output:

```text
4
```

#### `sqrt(n)`

```nodia
emit sqrt(9)
```

Output:

```text
3.0
```

#### `pow(a, b)`

```nodia
emit pow(2, 8)
```

Output:

```text
256
```

#### `min(a, b)`

```nodia
emit min(10, 3)
```

Output:

```text
3
```

#### `max(a, b)`

```nodia
emit max(10, 3)
```

Output:

```text
10
```

#### `clamp(n, min, max)`

```nodia
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

```nodia
emit sum([1, 2, 3])
```

Output:

```text
6
```

#### `avg(list)`

```nodia
emit avg([1, 2, 3])
emit avg([])
```

Output:

```text
2.0
null
```

#### `range(end)` and `range(start, end)`

```nodia
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

```nodia
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

```nodia
emit string(42)
emit string(true)
```

Output:

```text
42
true
```

#### `bool(value)`

```nodia
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

```nodia
emit keys({name: "Ana", role: "dev"})
```

Output:

```text
[name, role]
```

Map keys are stored in deterministic sorted order.

#### `values(map)`

```nodia
emit values({name: "Ana", role: "dev"})
```

Output:

```text
[Ana, dev]
```

#### `push(list, value)`

```nodia
emit push([1, 2], 3)
```

Output:

```text
[1, 2, 3]
```

#### `pop(list)`

```nodia
emit pop([1, 2, 3])
emit pop([])
```

Output:

```text
[1, 2]
[]
```

#### `first(list)`

```nodia
emit first(["a", "b"])
emit first([])
```

Output:

```text
a
null
```

#### `last(list)`

```nodia
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

```nodia
emit slice(["a", "b", "c", "d"], 1, 3)
```

Output:

```text
[b, c]
```

Text:

```nodia
emit slice("nodia", 1, 4)
```

Output:

```text
ric
```

Negative indexes count from the end:

```nodia
emit slice(["a", "b", "c", "d"], -3, -1)
```

Output:

```text
[b, c]
```

#### `reverse(list_or_text)`

```nodia
emit reverse([1, 2, 3])
emit reverse("abc")
```

Output:

```text
[3, 2, 1]
cba
```

#### `sort(list)`

```nodia
emit sort([3, 1, 2])
emit sort(["c", "a", "b"])
```

Output:

```text
[1, 2, 3]
[a, b, c]
```

#### `unique(list)`

```nodia
emit unique(["a", "b", "a", "c", "b"])
```

Output:

```text
[a, b, c]
```

## Diagnostics

Language/runtime errors use exit code `1`.

Example:

```nodia
val n = 1
n = 2
```

Output:

```text
error[E4101]: cannot assign to val 'n'
  at file.nod:2:1
```

Parse errors use `E1000`, runtime errors use `E2000`, IO errors use `E3000`, and semantic checker errors use `E41xx`. Write permission errors use `E3001`.

Write permission error:

```nodia
write("out.txt", "blocked")
```

Command:

```bash
nodia run file.nod
```

Output:

```text
error[E3001]: file write requires --allow-write
  at file.nod
```

JSON error output:

```bash
nodia run file.nod --json
```

Shape:

```json
{"ok":false,"error":{"message":"error[E3001]: file write requires --allow-write\n  at file.nod","exit_code":1}}
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

The formatter is part of the language contract. Prefer writing clear code and letting `nodia fmt`
settle layout.

## VSCode Support

Local syntax highlighting is available in:

```text
vscode/nodia-language
```

Install from VSCode:

```text
Developer: Install Extension from Location...
```

Select the `vscode/nodia-language` folder.

The extension supports `.nod` file association, keywords, strings, interpolation, comments,
numbers, operators, builtins, and language globals.

## Complete Examples

### Fibonacci With Lists

```nodia
func fibs(count) {
  var result = []
  var a = 0
  var b = 1

  for i in range(count) {
    result = push(result, a)

    var next = a + b
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

`upper_file.nod`:

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

Run:

```bash
nodia run upper_file.nod --allow-write
```

### Generated Report With Uses

`report/meta.nod`:

```nodia
val title = "Build Report"
val sections = ["summary", "artifacts", "status"]
```

`report/format.nod`:

```nodia
func heading(text) {
  return "== {upper(text)} =="
}

func bullet(value) {
  return "- {value}"
}
```

`main.nod`:

```nodia
use "./report/meta" as meta
use "./report/format" as fmt

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
