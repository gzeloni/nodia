# Diagnostics

Nodia errors have a stable shape:

```text
error[E1000]: message
  at file.nod:line:column
```

When `--json` is set, the same error becomes structured output:

```json
{
  "ok": false,
  "error": {
    "message": "error[E3001]: file write requires --allow-write\n  at file.nod",
    "exit_code": 1
  }
}
```

`nodia check --json` returns a list under `errors`:

```json
{
  "ok": false,
  "errors": [
    {"code": "E4101", "message": "cannot assign to val 'n'", "file": "file.nod", "line": 2, "column": 1}
  ]
}
```

## Error Codes

| Code     | Class                                              |
| -------- | -------------------------------------------------- |
| `E1000`  | lexical or parse error                             |
| `E2000`  | runtime language error                             |
| `E3000`  | IO error                                           |
| `E3001`  | file write attempted without `--allow-write`       |
| `E4000`  | generic semantic check error                       |
| `E4100`  | undefined variable                                 |
| `E4101`  | assignment to immutable binding                    |
| `E4102`  | duplicate binding or parameter                     |
| `E4103`  | invalid control-flow placement                     |
| `E4104`  | invalid use selection                              |
| `E4105`  | missing known field                                |
| `E4106`  | invalid interpolation                              |
| `E4107`  | invalid arity                                      |

## Exit Codes

| Code | Meaning                          |
| ---: | -------------------------------- |
| `0`  | success                          |
| `1`  | language or runtime error        |
| `2`  | invalid CLI usage                |
| `3`  | CLI IO error                     |
| `4`  | internal error (reserved)        |

## Examples

### Assignment to `val`

```nodia
val n = 1
n = 2
```

```text
error[E4101]: cannot assign to val 'n'
  at file.nod:2:1
```

### Write Without Permission

```nodia
write("out.txt", "blocked")
```

```bash
nodia run file.nod
```

```text
error[E3001]: file write requires --allow-write
  at file.nod
```

Re-run with the flag:

```bash
nodia run file.nod --allow-write
```

### Undefined Variable

```nodia
emit missing
```

```text
error[E4100]: undefined variable 'missing'
  at file.nod:1:6
```

### Invalid Arity

```nodia
emit upper("a", "b")
```

```text
error[E4107]: 'upper' expects 1 argument, got 2
```

### Parse Error

```nodia
val x =
```

```text
error[E1000]: expected expression
```

## Where Diagnostics Are Produced

| Step           | Codes                              |
| -------------- | ---------------------------------- |
| lexer / parser | `E1000`                            |
| use resolution | `E3000`, `E4104`                   |
| semantic check | `E41xx`                            |
| runtime        | `E2000`, `E3000`, `E3001`, `E4xxx` |

`nodia check` runs everything up to and including the semantic check. `nodia
run` / `nodia eval` go all the way through runtime.
