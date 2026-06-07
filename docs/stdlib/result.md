# Result Builtins

`0.8.0` introduces a first-class `result` value for recoverable pipeline
failure.

Import this namespace with `use result`.

This release introduces the core model only:

* `result.ok(...)` and `result.err(...)` construct explicit success/failure
  values.
* `result.is_ok(...)`, `result.is_err(...)`, `result.value(...)`, and
  `result.error(...)` inspect them.
* `result.raise(...)` converts a recoverable error back into a fatal runtime
  failure.

Most existing stdlib operations still fail fatally in `0.8.0`. Broader IO /
decode / JSON / CSV / regex adoption belongs to later `0.8.x` releases.

## Constructors

### `ok(value)`

Wraps a successful value:

```nodia
use result

emit result.ok("Ana")
```

```text
ok(Ana)
```

### `err(code, message)`

Creates a recoverable error value with the canonical error shape:

* `code`
* `message`
* `file`
* `line`
* `column`

`file`, `line`, and `column` are `null` when the error has no source location.

```nodia
use result

emit result.err("E8000", "missing row")
```

```text
err({code: "E8000", message: "missing row", file: null, line: null, column: null})
```

## Inspection

### `is_ok(result)` / `is_err(result)`

```nodia
use result

val ok = result.ok("Ana")
val bad = result.err("E8000", "missing row")

emit result.is_ok(ok)
emit result.is_err(bad)
```

```text
true
true
```

### `value(result)`

Returns the wrapped success value or `null` for `err(...)`.

### `error(result)`

Returns the canonical error map or `null` for `ok(...)`.

```nodia
use result

val bad = result.err("E8000", "missing row")
emit result.error(bad).code
emit result.error(bad).message
```

```text
E8000
missing row
```

## `raise(result)`

Turns a recoverable error back into a fatal runtime failure. This is the
explicit boundary between recoverable pipeline state and process-failing
runtime errors.

```nodia
use result

emit result.raise(result.ok("Ana"))
```

```text
Ana
```

```nodia
use result

emit result.raise(result.err("E8000", "missing row"))
```

```text
error[E8000]: missing row
```
