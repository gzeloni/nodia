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
| `E3002`  | environment read attempted without `--allow-env`   |
| `E3003`  | process execution attempted without `--allow-process` |
| `E4000`  | generic semantic check error                       |
| `E4100`  | undefined variable                                 |
| `E4101`  | assignment to immutable binding                    |
| `E4102`  | duplicate binding or parameter                     |
| `E4103`  | invalid control-flow placement                     |
| `E4104`  | invalid use selection                              |
| `E4105`  | missing known field or key                         |
| `E4106`  | invalid interpolation                              |
| `E4107`  | invalid arity                                      |
| `E4200`  | regex semantic or replacement-placeholder error    |

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
use io

io.write("out.txt", "blocked")
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
use text

emit text.upper("a", "b")
```

```text
error[E4107]: text.upper() expects 1 argument(s), got 2
```

### Invalid Regex Replacement Placeholder

```nodia
use text

emit text.replace("ana", regex {
  named word {
    one_or_more letter
  }
}, "$(missing)")
```

```text
error[E4200]: regex replacement refers to missing named capture 'missing'
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
| semantic check | `E41xx`, `E4200`                   |
| runtime        | `E2000`, `E3000`, `E3001`, `E3002`, `E3003`, `E4xxx` |

`nodia check` runs everything up to and including the semantic check. `nodia
run` / `nodia eval` go all the way through runtime.
