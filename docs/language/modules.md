# Modules (`use`)

Nodia composes programs from `.nod` files via the `use` declaration. Modules
are lazily linked, cached by canonical path, and support cycles.

`use` also loads selected stdlib namespaces without string paths:

```nodia
use json
use csv as table
use result

emit result.raise(json.read(r'{"name":"Ana"}')).name
emit table.write([{name: "Ana"}])
```

`regex` is built into the language and does not use `use`:

```nodia
use result
emit result.raise(regex.find("ana 42", regex { one_or_more digit })).text
```

Direct stdlib picks work too:

```nodia
use numbers pick range
use conversion pick string

for i in range(3) {
  emit string(i)
}
```

## Basic Form

```nodia
use "./lib/constants"
```

Rules:

* Paths are relative to the source file containing the `use`.
* Absolute paths are accepted.
* `.nod` extension is optional.
* Directories resolve through `index.nod`.

For stdlib modules:

* use a bare identifier such as `json` or `csv`;
* bare `use text` / `use json` binds the whole stdlib module as a namespace;
* `pick` without `as` imports only the selected stdlib names directly;
* `pick` with `as` filters the namespace bound under that alias;
* `hide` excludes names from either form.

```text
lib/
  index.nod
```

```nodia
use "./lib" as lib
```

## Namespace Uses (`as`)

`as` binds the selected exports as a namespace map:

```nodia
use "./lib/meta" as meta

emit meta.title
emit meta.version
```

## Direct Uses (`pick`)

`pick` brings selected names directly into the current scope:

```nodia
use "./lib/meta" pick title, version

emit title
emit version
```

A `pick` name that the module does not export is a semantic error (`E4104`).

## Hide Clause

`hide` lists names to **exclude** from the imported set:

```nodia
use "./lib/meta" hide internal_token
```

When combined with `as`, the hidden names disappear from the namespace map.
When combined with bare `use`, they are not brought into the current scope.

## Combined Form

A `use` declaration can combine clauses:

```nodia
use "./lib" as lib pick title, version hide secret
```

The combination is evaluated as: take everything the module exports, drop
`hide` names, restrict to `pick` names, expose under the `as` name.

## What Is Exported

Top-level declarations in a module are exported:

* `val`
* `var`
* `func`

Local bindings declared inside blocks or function bodies are not exported.

## Mutability

* Used `val` and `func` bindings are **read-only** in the importer.
* Used `var` bindings remain mutable. The mutation is visible across all
  importers of that module instance.

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

```text
0
1
2
```

## Caching

Modules are resolved to a canonical path and cached. Two `use`s of the same
file resolve to the same module instance, so they share state.

## Circular Uses

Cycles are allowed structurally. Bindings are linked lazily — a cycle only
fails if code reads a binding before that binding has been initialized.

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

```text
A/B
B/A
```

## Diagnostics

| Code     | When                                                |
| -------- | --------------------------------------------------- |
| `E1000`  | parse failure inside the module                     |
| `E3000`  | module path could not be resolved on disk           |
| `E4104`  | invalid use selection (`pick` of missing name, etc.) |
| `E4100`  | reading a `pick`-ed name that does not exist        |
