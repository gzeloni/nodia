# Command Line

Nodia exposes a single binary, `nodia`, with subcommands. The general shape is:

```bash
nodia [global-flags] <command> [command-args]
nodia <command> [command-args] [global-flags]
```

Global flags are accepted on either side of the command for nearly every
workflow.

## Commands

| Command                                        | Purpose                                                |
| ---------------------------------------------- | ------------------------------------------------------ |
| [`nodia run`](run.md)                          | Execute a `.nod` file or stdin source.                 |
| [`nodia check`](check.md)                      | Lex, parse, resolve uses, run semantic checks.         |
| [`nodia eval`](eval.md)                        | Execute source passed as CLI text.                     |
| [`nodia fmt`](fmt.md)                          | Format `.nod` files using the canonical style.         |
| [`nodia tokens`](tokens-ast.md#nodia-tokens)   | Print the lexer token stream.                          |
| [`nodia ast`](tokens-ast.md#nodia-ast)         | Print the parsed AST.                                  |
| [`nodia init`](init-version.md#nodia-init)     | Scaffold a new project.                                |
| [`nodia version`](init-version.md#nodia-version) | Print version metadata.                              |

## Global Flags

| Flag                | Meaning                                                                 |
| ------------------- | ----------------------------------------------------------------------- |
| `--json`            | Emit JSON diagnostics and JSON output for commands that support it.     |
| `--quiet`           | Suppress non-error output for commands that support it.                 |
| `--verbose`         | Reserved; accepted but intentionally minimal in 0.7.                    |
| `--color auto`      | Color mode. Output is currently plain text.                             |
| `--color always`    | Color mode. Output is currently plain text.                             |
| `--color never`     | Color mode. Output is currently plain text.                             |
| `--allow-write`     | Allow Nodia code to write files through `io.*` builtins (see below).    |
| `--allow-env`       | Allow Nodia code to read process environment variables.                 |
| `--allow-process`   | Allow Nodia code to spawn subprocesses through `system.exec(...)`.      |
| `--help`, `-h`      | Print help.                                                             |
| `--version`, `-V`   | Print version.                                                          |

### `--allow-write`

`--allow-write` controls **language-level** file writes performed by code:

* `io.write(path, text)`
* `io.append(path, text)`
* `io.open(path, "write")` / `io.open(path, "append")`

Without `--allow-write`, those calls fail with `E3001`:

```text
error[E3001]: file write requires --allow-write
```

This flag does **not** affect:

* `emit` (program output channel),
* `io.write(io.stdout, ...)` / `io.writeln(io.stderr, ...)` (standard streams),
* the `--out` / `-o` CLI flag, which is a CLI redirection of the program's
  emitted output and is always allowed.

### `--allow-env`

`--allow-env` gates `system.env(...)` access from Nodia code.

Without it, environment reads fail with `E3002`.

### `--allow-process`

`--allow-process` gates `system.exec(...)`.

Without it, subprocess execution fails with `E3003`.

## Exit Codes

| Code | Meaning                          |
| ---: | -------------------------------- |
| `0`  | Success.                         |
| `1`  | Language or runtime error.       |
| `2`  | Invalid CLI usage.               |
| `3`  | CLI IO error.                    |
| `4`  | Internal error (reserved).       |

## Help

```bash
nodia help
nodia --help
nodia -h
```

```text
Nodia 0.7.4

Usage:
  nodia run [file.nod] [--var key=value] [--vars key=value ...] [--out output.txt] [--allow-write] [--allow-env] [--allow-process] [-- script-args...]
  nodia check [file.nod] [--json]
  nodia fmt [file.nod|dir] [--check] [--stdout]
  nodia eval 'emit "hello"' [-- script-args...]
  nodia -e 'emit "hello"' [-- script-args...]
  nodia tokens file.nod [--json]
  nodia ast file.nod [--json]
  nodia init [dir]
  nodia version [--json]
```
