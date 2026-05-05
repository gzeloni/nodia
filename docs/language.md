# Dobra Language v0.4

This document summarizes the v0.4 language and tooling contract. The normative baseline is [specification.md](specification.md).

For the full user reference with command examples and builtin examples, see [reference.md](reference.md).

## Purpose

Dobra is a focused language for textual automation, structured output, and mathematical/data
workflows. It is not a systems language and it is not trying to become a broad application
platform.

The syntax should stay easy to read and format. Builtins should be short, technical, and
predictable.

## Source Files

Dobra source files use the `.dob` extension.

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

## Formatting

Formatting is canonical and non-configurable.

| Rule | Style |
|---|---|
| Indent | 2 spaces |
| Braces | same line |
| Operators | spaces around binary operators |
| Blocks | always use `{}` |
| Maps | non-empty maps are multi-line |
| Lists/calls | inline when short, multi-line when long |
| Line width | formatter-controlled lines target 60 characters |
| Final newline | required |

The formatter is exposed through:

```bash
dobra fmt file.dob
dobra fmt .
dobra fmt --check .
dobra fmt --stdout file.dob
```

## Imports

```dobra
import "./lib/constants"
import "./lib/format" as fmt
import "./lib/tokens" show title, version
import "./lib/internal" hide secret
```

Rules:

- paths are relative to the importing file;
- `.dob` is optional;
- directories resolve through `index.dob`;
- modules are cached by resolved path;
- circular imports are allowed;
- imported bindings are linked lazily;
- cycles only fail when code reads a binding before initialization;
- `as` imports selected bindings into a namespace map;
- imports without `as` insert selected bindings into the current scope;
- `show` includes only listed names;
- `hide` excludes listed names;
- imported `let` bindings remain mutable;
- imported `const` and `fn` bindings are read-only.

## Semantic Check

`dobra check` is no longer parse-only in v0.4. It validates imports and performs
baseline semantic checks before `run` can execute the program.

It rejects undefined variables, duplicate bindings in the same scope, assignment
to `const`, invalid function/builtin arity, invalid `show` imports, missing fields
on known maps/namespaces, `return` outside functions, and `break`/`continue`
outside loops.

The checker is not yet a static type or effect checker.

## IO

Streams are runtime values.

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

Builtins:

| Builtin | Behavior |
|---|---|
| `open(path, mode)` | opens a stream; modes are `read`, `write`, `append` |
| `close(stream)` | closes a stream |
| `flush(stream)` | flushes pending writes |
| `read(path)` | reads a whole file |
| `read(stream)` | reads the rest of a stream |
| `read(stream, size)` | reads a chunk from a stream |
| `readln(stream)` | reads one line or returns `null` at EOF |
| `write(path, text)` | writes a whole file |
| `write(stream, text)` | writes to a stream |
| `writeln(stream, text)` | writes text plus newline to a stream |
| `append(path, text)` | appends text to a file |
| `eof(stream)` | returns whether a readable file stream reached EOF |

Standard streams:

```dobra
readln(stdin)
write(stdout, "text")
writeln(stderr, "error")
```

File writes require explicit permission:

```bash
dobra run script.dob --allow-write
```

Without permission, file writes fail with `E3001`.

## CLI Contract

```bash
dobra run file.dob
dobra check file.dob
dobra fmt file.dob
dobra eval 'emit "hello"'
dobra tokens file.dob
dobra ast file.dob
dobra init
dobra version
```

Global flags:

```text
--json
--quiet
--verbose
--color auto|always|never
--allow-write
```

Exit codes:

| Code | Meaning |
|---:|---|
| 0 | success |
| 1 | language/runtime error |
| 2 | invalid CLI usage |
| 3 | IO error |
| 4 | internal error |

## Standard Library

Text:

| Builtin | Behavior |
|---|---|
| `upper(text)` | uppercase text |
| `lower(text)` | lowercase text |
| `capitalize(text)` | capitalize text |
| `trim(text)` | trim surrounding whitespace |
| `replace(text, from, to)` | replace text |
| `split(text, sep)` | split text into a list |
| `join(list, sep)` | join list values |
| `lines(text)` | split text into lines |
| `unlines(list)` | join list values with newlines |
| `words(text)` | split text by whitespace |
| `contains(value, needle)` | check string/list/map-key containment |
| `starts(text, prefix)` | check text prefix |
| `ends(text, suffix)` | check text suffix |
| `indent(text, spaces_or_prefix)` | prefix each line |
| `dedent(text)` | remove common indentation |

Numbers:

| Builtin | Behavior |
|---|---|
| `int(value)` | convert to integer |
| `float(value)` | convert to float |
| `abs(n)` | absolute value |
| `floor(n)` | round down |
| `ceil(n)` | round up |
| `round(n)` | round to nearest integer |
| `sqrt(n)` | square root |
| `pow(a, b)` | power |
| `min(a, b)` | minimum |
| `max(a, b)` | maximum |
| `clamp(n, min, max)` | clamp into a range |
| `sum(list)` | sum numeric list |
| `avg(list)` | average numeric list, or `null` for empty list |
| `range(end)` | integers from `0` to `end - 1` |
| `range(start, end)` | integers from `start` to `end - 1` |

Data:

| Builtin | Behavior |
|---|---|
| `len(value)` | length of string/list/map |
| `string(value)` | convert to string |
| `bool(value)` | convert to boolean |
| `keys(map)` | map keys |
| `values(map)` | map values |
| `push(list, value)` | returns list with value appended |
| `pop(list)` | returns list without its last value |
| `first(list)` | first value or `null` |
| `last(list)` | last value or `null` |
| `slice(list_or_text, start, end)` | slice by index |
| `reverse(list_or_text)` | reverse value |
| `sort(list)` | sort values deterministically |
| `unique(list)` | remove duplicate values |
