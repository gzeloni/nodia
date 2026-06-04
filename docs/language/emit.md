# Emit & Output

Nodia has a dedicated output channel for the **program output** that scripts
produce. The keyword `emit` writes to that channel.

```nodia
emit "one"
emit "two"
```

```bash
./target/release/nodia eval '
emit "one"
emit "two"
'
```

```text
one
two
```

`emit` appends a newline after each emitted value, like `println`.

## Why A Separate Channel?

Most Nodia scripts produce generated artifacts: reports, config files, snippets,
templated text. The output channel is meant to be the **clean payload** the
script generates, independent from incidental writes to `stderr` for logs or
warnings.

In CLI terms:

* `emit` → program output channel
* `write(stderr, ...)` → standard error
* `write(stdout, ...)` → process stdout (raw, no extra newline)

By default, the program output channel **is** the process stdout, so
`./target/release/nodia run report.nod` prints `emit` output directly.

## Redirecting To A File

The CLI provides `--out` / `-o` to write the output channel to a file
**without** giving the script file-write permission:

```bash
./target/release/nodia run report.nod --out report.txt
```

The script does not need `--allow-write` for this — `--out` is a CLI
redirection, not a language-level write. See [`nodia run`](../cli/run.md).

When `--out` is given without a path, Nodia writes beside the source path
with `.out` appended:

```bash
./target/release/nodia run report.nod --out
# writes report.nod.out
```

## Emitting Non-String Values

`emit` accepts any value. Non-string values are converted to text using their
display form:

```bash
./target/release/nodia eval '
emit 42
emit true
emit [1, 2, 3]
emit {name: "Ana"}
emit null
'
```

```text
42
true
[1, 2, 3]
{name: "Ana"}
null
```

For maps and lists the display form is intentionally readable but not a
formal serialization — see [Conversion](../stdlib/conversion.md) for explicit
string/bool/int/float conversion.

## When To Use `write(stdout, ...)`

`emit` is the right choice for the generated payload. For stream-style
output without an added newline — for example, building a single line with
multiple writes — use `write(stdout, text)`:

```bash
./target/release/nodia eval '
write(stdout, "hello")
write(stdout, " world\n")
'
```

```text
hello world
```

`writeln(stdout, text)` is the stream-style equivalent of `emit text`.
