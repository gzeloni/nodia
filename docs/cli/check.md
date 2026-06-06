# `nodia check`

`nodia check` lexes, parses, resolves uses, validates regex DSL structure, and
runs the v0.7 semantic checks **without executing the program**. It is the
fastest way to surface syntactic and semantic errors in CI.

```bash
nodia check file.nod
nodia check file.nod --json
nodia check                   # uses nodia.toml entry
```

## Success

```bash
./target/release/nodia check src/main.nod
```

```text
ok src/main.nod
```

JSON form:

```bash
./target/release/nodia check src/main.nod --json
```

```json
{"ok":true,"errors":[]}
```

## Failure

`bad.nod`:

```nodia
val n = 1
n = 2
```

```bash
./target/release/nodia check bad.nod
```

```text
error[E4101]: cannot assign to val 'n'
  at bad.nod:2:1
```

JSON form:

```bash
./target/release/nodia check bad.nod --json
```

```json
{"ok":false,"errors":[{"code":"E4101","message":"cannot assign to val 'n'","file":"bad.nod","line":2,"column":1}]}
```

## What `check` Validates

`check` runs the v0.7 semantic checker over the program AST:

* Lexical and parse correctness (`E1000`).
* `use` resolution and selection (`E4104`).
* Undefined variables (`E4100`).
* Assignment to `val` bindings (`E4101`).
* Duplicate bindings or parameters in the same scope (`E4102`).
* `return` outside a function, `break` / `continue` outside a loop (`E4103`).
* Invalid arity on user functions and known builtins (`E4107`).
* Missing fields or known map keys on used namespaces and literal maps (`E4105`).
* Invalid string interpolation (`E4106`).
* Regex DSL structural validity and statically-checkable regex replacement placeholders (`E4200`).

It is **not yet** a static type, effect, ownership, or allocation checker.

## Use In CI

A typical CI step:

```bash
./target/release/nodia fmt --check .
./target/release/nodia check src/main.nod
```

Either step exits non-zero on failure; combine with your runner's standard
error handling.
