# `nodia eval`

Execute Nodia source supplied as a CLI string.

```bash
nodia eval 'emit "hello"'
nodia eval '<source>' --allow-write
nodia eval '<source>' --allow-process
nodia -e 'emit "hello"'
```

`eval` is the same pipeline as `run`: lex, parse, check, execute. The only
difference is that the source comes from the command line instead of a file
or stdin.

## Quick Expressions

```bash
./target/release/nodia eval 'emit 1 + 2'
```

```text
3
```

```bash
./target/release/nodia eval 'emit upper("nodia")'
```

```text
NODIA
```

## Multi-Statement Source

The source string can contain any sequence of statements:

```bash
./target/release/nodia eval '
val tags = ["compiler", "formatter", "streams"]
for tag in tags {
  emit "- {capitalize(tag)}"
}
'
```

```text
- Compiler
- Formatter
- Streams
```

## Regex In Eval

The regex DSL works the same in eval:

```bash
./target/release/nodia eval '
val date = regex(case_insensitive) {
  start
  named year { exactly 4 digit }
  "-"
  exactly 2 digit
  "-"
  exactly 2 digit
  end
}
emit date
'
```

```text
(?i)^(?<year>\d{4})-\d{2}-\d{2}$
```

## File Writes In Eval

`eval` honors `--allow-write` just like `run`:

```bash
./target/release/nodia eval 'write("out.txt", "ok")' --allow-write
```

Without the flag the call fails with `E3001`.

## Script Arguments

Like `run`, `eval` accepts trailing script arguments after `--`:

```bash
./target/release/nodia eval 'emit args
emit args[0]' -- one two
```

```text
["one", "two"]
one
```

## Environment Access

`env(...)` is gated separately:

```bash
HOME=/tmp ./target/release/nodia -e 'emit env("HOME")' --allow-env
```

## Process Execution

`exec(...)` is also gated explicitly:

```bash
./target/release/nodia -e 'emit exec("/bin/sh", ["-c", "printf ok"]).stdout' --allow-process
```

## Notes

* `eval` does not have an associated source file; diagnostics report positions
  relative to the eval source (line/column starting at `1:1`).
* Use single quotes around the source string in shells that expand `$`-style
  interpolation, because Nodia uses `{...}` for interpolation, not shell
  variables, but double quotes will be re-interpreted by the shell.
