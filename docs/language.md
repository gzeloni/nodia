# Orich Language v0.2

This document defines the v0.2 language and tooling contract.

## Purpose

Orich is a small language for text automation and structured output generation. It is not trying
to be a general-purpose application language.

## Source Files

Orich source files use the `.och` extension.

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

Formatting is canonical and non-configurable in v0.2.

| Rule | Style |
|---|---|
| Indent | 2 spaces |
| Braces | same line |
| Operators | spaces around binary operators |
| Maps | non-empty maps are multi-line |
| Lists/calls | inline when short, multi-line when long |
| Final newline | required |

The formatter is exposed through:

```bash
orich fmt file.och
orich fmt .
orich fmt --check .
orich fmt --stdout file.och
```

## Imports

```orich
import './lib/constants'
import './lib/format' as fmt
import './lib/tokens' show title, version
import './lib/internal' hide secret
```

Rules:

- paths are relative to the importing file;
- `.och` is optional;
- directories resolve through `index.och`;
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

## CLI Contract

```bash
orich run file.och
orich check file.och
orich fmt file.och
orich eval 'emit "hello"'
orich tokens file.och
orich ast file.och
orich init
orich version
```

Global flags:

```text
--json
--quiet
--verbose
--color auto|always|never
```

Exit codes:

| Code | Meaning |
|---:|---|
| 0 | success |
| 1 | language/runtime error |
| 2 | invalid CLI usage |
| 3 | IO error |
| 4 | internal error |
