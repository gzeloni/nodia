# System Builtins

These features connect Nodia code to the process that launched it.

Import this namespace with `use system`.

## `args`

`system.args` is a read-only runtime binding containing trailing script
arguments as a
list of strings.

For `run` and `eval`, pass script arguments after `--`:

```bash
./target/release/nodia eval 'use system
emit system.args
emit system.args[1]' -- one two
```

```text
["one", "two"]
two
```

## `env(name)` / `env(name, default)`

Reads an environment variable. This requires `--allow-env`.

If the variable is missing:

* `system.env(name)` returns `null`
* `system.env(name, default)` returns `default`

```bash
HOME=/tmp ./target/release/nodia eval 'use system
emit system.env("HOME")' --allow-env
```

```text
/tmp
```

Without permission, `system.env(...)` fails with `E3002`.

## `exec(cmd)` / `exec(cmd, args)`

Runs a subprocess and returns:

```nodia
{
  stdout: b"out",
  stderr: b"err",
  status: 0,
}
```

This requires `--allow-process`.

```bash
./target/release/nodia eval '
use system
use text
use result
val exec_result = system.exec("/bin/sh", [
  "-c",
  "printf out; printf err 1>&2; exit 7",
])
emit result.raise(text.decode(exec_result.stdout, text.utf8))
emit result.raise(text.decode(exec_result.stderr, text.utf8))
emit exec_result.status
' --allow-process
```

```text
out
err
7
```

`stdout` and `stderr` are raw `bytes` values.
This keeps subprocess decoding explicit and avoids hidden lossy conversion.
Use `result.raise(text.decode(..., text.utf8))` for strict decoding or
`result.raise(text.decode(..., text.utf8, text.lossy))` when replacement
semantics are actually desired.

## `exit()` / `exit(code)`

Stops execution immediately and returns an exit status to the shell.

```bash
./target/release/nodia eval '
use system
emit "before"
system.exit(7)
emit "after"
'
echo $?
```

```text
before
7
```

`system.exit()` without an argument uses status `0`.
