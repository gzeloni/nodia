# Result Builtins

Recoverable pipeline values live in the `result` namespace.

Import it with `use result`.

In `0.8.3`, this is no longer just a standalone value type. The same model is
used by:

* `text.decode(...)`
* `io.open(...)`, `io.read(...)`, `io.readln(...)`, `io.write(...)`, and the
  rest of `io.*`
* `regex.test(...)` and `regex.find(...)`
* `json.read(...)`
* `csv.read(...)`
* `datetime.parse(...)`

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

Creates a recoverable error payload with these fields:

* `code`
* `message`
* `file`
* `line`
* `column`
* `context`
* `span`

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

Returns the wrapped success value or `null`.

### `value_or(result, fallback)`

Returns the wrapped success value, or the fallback when the result is `err(...)`.

```nodia
use result

emit result.value_or(result.ok("Ana"), "fallback")
emit result.value_or(result.err("E8000", "missing row"), "fallback")
```

```text
Ana
fallback
```

### `error(result)`

Returns the canonical error map or `null`.

When the failure comes from nested text/data work, the map also carries:

* `context`: outer-to-inner pipeline labels such as `["json.read"]` or
  `["text.decode"]`;
* `span`: nested input position such as `{line: 2, column: 1}` when the
  failure points inside JSON, CSV, or regex replacement text.

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

```nodia
use json
use result

val bad = result.error(json.read(r"""{
true}"""))
emit bad.context
emit bad.span.line
emit bad.span.column
```

```text
["json.read"]
2
1
```

## Pipeline Helpers

### `then(result, func)`

Runs `func(value)` only for `ok(...)`.

If `func` returns a plain value, Nodia wraps it back into `ok(...)`. If `func`
already returns a `result`, it is preserved as-is.

```nodia
use result
use text

emit result.then(result.ok("ana"), text.upper)
emit result.then(result.err("E8000", "missing row"), text.upper)
```

```text
ok(ANA)
err({code: "E8000", message: "missing row", file: null, line: null, column: null})
```

### `recover(result, func)`

Runs `func(error)` only for `err(...)`.

```nodia
use result

func fallback(error) {
  return "[{error.code}] {error.message}"
}

emit result.recover(result.err("E8000", "missing row"), fallback)
```

```text
ok([E8000] missing row)
```

### `raise(result)`

Turns a recoverable error back into a fatal runtime failure. This is the
explicit boundary for scripts that want pipeline semantics internally but still
abort on failure at the edge.

Any text already emitted before `raise(...)` stays preserved in the output
channel.

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

## Common Flows

### Fail Fast At The Boundary

```nodia
use json
use result

val doc = result.raise(json.read(r'{"name":"Ana"}'))
emit doc.name
```

### Skip Bad Input

```nodia
use json
use result

for line in [
  r'{"name":"Ana"}',
  r'{"name":',
  r'{"name":"Bia"}',
] {
  val parsed = json.read(line)
  if result.is_err(parsed) {
    continue
  }
  emit result.value(parsed).name
}
```

### Classify And Continue

```nodia
use datetime
use result

func classify(error) {
  return {ok: false, code: error.code, message: error.message}
}

emit result.recover(
  datetime.parse("2024-99-99", datetime.as_date),
  classify,
)
```
