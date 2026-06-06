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
  stdout: [111, 117, 116],
  stderr: [101, 114, 114],
  status: 0,
}
```

This requires `--allow-process`.

```bash
./target/release/nodia eval '
use system
use text
val result = system.exec("/bin/sh", [
  "-c",
  "printf out; printf err 1>&2; exit 7",
])
emit text.decode_utf8(result.stdout)
emit text.decode_utf8(result.stderr)
emit result.status
' --allow-process
```

```text
out
err
7
```

`stdout` and `stderr` are raw byte sequences represented as `list<int>`.
This keeps subprocess decoding explicit and avoids hidden lossy conversion.
Use `text.decode_utf8(...)` for strict decoding or `text.decode_utf8_lossy(...)`
when replacement semantics are actually desired.

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
