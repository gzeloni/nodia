# Errors

Recoverable failures in Nodia are handled with `try`, `catch`, and `throw`.
There is no `result` value surface anymore.

## `throw`

`throw` aborts the current control flow with a runtime error:

```nodia
throw {code: "E8000", message: "missing row"}
```

You can also throw a plain value:

```nodia
throw "boom"
```

Plain values are normalized into a runtime error. Maps with canonical error
fields preserve their structure.

## `try` / `catch`

`try` executes a block. If that block raises a runtime error, control jumps to
`catch` and binds the error payload to the chosen name:

```nodia
try {
  emit text.decode(b"\xff", text.utf8)
} catch err {
  emit err.code
  emit err.context[0]
}
```

The catch binding only exists inside the `catch` block.

## Error Shape

The canonical caught error is a map with these fields:

| Field | Meaning |
| --- | --- |
| `code` | stable error code such as `E3000` |
| `message` | human-readable message |
| `file` | source or file path when available |
| `line` | one-based line when available |
| `column` | one-based column when available |
| `context` | call stack-like list of surface names |
| `span` | nested structured span when the operation exposes one |

Example:

```nodia
try {
  emit datetime.parse("2024-99-99", datetime.as_date)
} catch err {
  emit err.code
  emit err.message
  emit err.context[0]
}
```

## What Can Be Caught

Operational failures from IO, decoding, regex compilation/matching, JSON, CSV,
datetime parsing, and explicit `throw` can be caught.

Contract misuse still stays fatal in the sense that it raises an error
immediately:

* wrong arity
* wrong argument kind
* invalid field access
* invalid indexing
* impossible byte/scalar/grapheme boundaries

Those failures are still catchable through `try` / `catch`, but they are not a
separate value layer.
