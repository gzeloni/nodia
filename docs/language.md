# Nodia Language v0.5

This document summarizes the v0.5 language and tooling contract. The normative baseline is [specification.md](specification.md).

For the full user reference with command examples and builtin examples, see [reference.md](reference.md).

## Purpose

Nodia is a focused language for textual automation, structured output, and mathematical/data
workflows. It is not a systems language and it is not trying to become a broad application
platform.

The syntax should stay easy to read and format. Builtins should be short, technical, and
predictable.

## Source Files

Nodia source files use the `.nod` extension.

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
nodia fmt file.nod
nodia fmt .
nodia fmt --check .
nodia fmt --stdout file.nod
```

## Uses

```nodia
use "./lib/constants"
use "./lib/format" as fmt
use "./lib/tokens" pick title, version
use "./lib/internal" hide secret
```

Rules:

- paths are relative to the source file containing the use;
- `.nod` is optional;
- directories resolve through `index.nod`;
- modules are cached by resolved path;
- circular uses are allowed;
- used bindings are linked lazily;
- cycles only fail when code reads a binding before initialization;
- `as` uses selected bindings into a namespace map;
- uses without `as` insert selected bindings into the current scope;
- `pick` includes only listed names;
- `hide` excludes listed names;
- used `var` bindings remain mutable;
- used `val` and `func` bindings are read-only.

## Semantic Check

`nodia check` validates the v0.5 syntax and semantic baseline. It validates uses and performs
baseline semantic checks before `run` can execute the program.

It rejects undefined variables, duplicate bindings in the same scope, assignment
to `val`, invalid function/builtin arity, invalid `pick` uses, missing fields
on known maps/namespaces, `return` outside functions, and `break`/`continue`
outside loops.

The checker is not yet a static type or effect checker.

## IO

Streams are runtime values.

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

```nodia
readln(stdin)
write(stdout, "text")
writeln(stderr, "error")
```

File writes require explicit permission:

```bash
nodia run script.nod --allow-write
```

Without permission, file writes fail with `E3001`.

## CLI Contract

```bash
nodia run file.nod
nodia check file.nod
nodia fmt file.nod
nodia eval 'emit "hello"'
nodia tokens file.nod
nodia ast file.nod
nodia init
nodia version
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
