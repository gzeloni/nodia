# Variables

Nodia has two binding forms: immutable `val` and mutable `var`.

## `val` (Immutable)

```nodia
val app = "nodia"
emit app
```

```bash
./target/release/nodia eval 'val app = "nodia"
emit app'
```

```text
nodia
```

Reassigning a `val` is a runtime/semantic error:

```nodia
val count = 0
count = 1
```

```text
error[E4101]: cannot assign to val 'count'
```

## `var` (Mutable)

```nodia
var count = 0
count = count + 1
emit count
```

```bash
./target/release/nodia eval 'var count = 0
count = count + 1
emit count'
```

```text
1
```

Assignment searches enclosing scopes and updates the first matching mutable
binding. Assigning to a name that has never been declared is a runtime error.

## Scopes

Bindings live in lexical block scopes:

* The root scope contains top-level bindings and built-in runtime bindings.
* Blocks create nested scopes (`if`, `else`, `for`, `while`, function bodies).
* Function parameters and `for` loop variables are mutable in v0.6.

```bash
./target/release/nodia eval '
val outer = 1
if true {
  val outer = 2     # shadows the outer binding inside this block
  emit outer
}
emit outer
'
```

```text
2
1
```

## Duplicate Bindings

Declaring two bindings with the same name in the same scope is rejected by
the checker:

```nodia
val name = "a"
val name = "b"
```

```text
error[E4102]: duplicate binding 'name'
```

## CLI Input

CLI variables passed via `--var` and `--vars` are exposed through the read-only
`input` map.

`hello.nod`:

```nodia
emit input.app
emit input.env
```

```bash
./target/release/nodia run hello.nod --vars app=nodia env=prod
```

```text
nodia
prod
```

JSON variables files preserve typed scalars:

```json
{ "app": "nodia", "limit": 3, "enabled": true }
```

YAML variables files are intentionally flat:

```yaml
app: nodia
env: prod
```

`input` is not a special syntactic form — it is a regular map that the runtime
seeds before the program runs. Field access (`input.app`) and index access
(`input["app"]`) both work.

If your CLI does not pass any variables, `input` is an empty map.
