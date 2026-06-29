# IO Builtins

This page is the quick reference for IO builtins. For the narrative of how
file IO and streams work in Nodia (including `--allow-write`), see
[IO & Streams](../language/io.md).

Import this namespace with `use io`.

Operational IO builtins return `result`. Contract misuse still stays fatal:
wrong arity, invalid mode names, writing raw bytes to `io.stdout`, reading from
a writable stream, and similar mistakes are still `E2000`.

## Standard Streams

| Binding      | Meaning                          |
| ------------ | -------------------------------- |
| `io.stdin`   | process standard input           |
| `io.stdout`  | program output (same as `emit`)  |
| `io.stderr`  | process standard error           |

## File Streams

| Builtin                | Behavior                                                  |
| ---------------------- | --------------------------------------------------------- |
| `open(path, mode)`     | open a stream and return `result`                         |
| `close(stream)`        | close a stream, flush writes, and return `result`         |
| `flush(stream)`        | flush a writable stream and return `result`               |
| `eof(stream)`          | report EOF state through `result`                         |

## Path Helpers

| Builtin            | Behavior                                                  |
| ------------------ | --------------------------------------------------------- |
| `basename(path)`   | last lexical path component                               |
| `dirname(path)`    | lexical parent path; `"."` when there is no parent       |
| `exists(path)`     | whether a path exists                                     |
| `is_file(path)`    | whether a path exists and is a file                       |
| `is_dir(path)`     | whether a path exists and is a directory                  |

## Directories And Globs

| Builtin            | Behavior                                                  |
| ------------------ | --------------------------------------------------------- |
| `list_dir(path)`   | returns entry names in lexicographic order                |
| `glob(pattern)`    | returns matching paths in lexicographic order             |

`glob(...)` supports:

* `*` for zero or more characters inside one path segment
* `?` for exactly one character inside one path segment
* `**` for zero or more directory segments

## Reading

| Builtin                | Behavior                                                |
| ---------------------- | ------------------------------------------------------- |
| `read(path)`           | read a whole file into a string through `result`        |
| `read(stream)`         | read the rest of a readable stream through `result`     |
| `read(stream, size)`   | read a UTF-8-safe chunk through `result`                |
| `read(path, io.bytes)` | read a whole file into `bytes` through `result`         |
| `read(stream, io.bytes)` | read the rest of a readable stream into bytes through `result` |
| `read(stream, io.bytes, size)` | read up to `size` raw bytes through `result` |
| `readln(stream)`       | read one line through `result`; `ok(null)` at EOF       |

`readln(stream)` strips a trailing `\n` or `\r\n` and still returns the final
line when the file does not end with a newline.
All text readers are UTF-8 strict: invalid bytes produce `err({code: "E3000", ...})`.
When you need undecoded input, use `read(..., io.bytes)` and choose
`text.decode(..., text.utf8)` or `text.decode(..., text.utf8, text.lossy)`
explicitly.
The resulting value is `bytes`, so direct indexing returns `int` bytes and
`collections.slice(...)` returns another `bytes` value.

## Writing

All file writes (where `path` or `mode = "write"/"append"` is involved) require
`--allow-write`.

| Builtin                      | Behavior                                            |
| ---------------------------- | --------------------------------------------------- |
| `write(path, value)`         | write text or raw bytes through `result`            |
| `write(stream, value)`       | write text or raw bytes through `result`            |
| `writeln(stream, text)`      | write text and a newline through `result`           |
| `append(path, value)`        | append text or raw bytes through `result`           |

`write(io.stdout, b"...")` is rejected on purpose. `io.stdout` is still the
program text-output channel used by `emit`, so raw bytes must go to files or
`io.stderr`.

```nodia
use io
use result

result.raise(io.write("payload.bin", b"\0\x01\x02\xff"))
result.raise(io.append("payload.bin", b"\n"))
```

## Examples

### Read A File Whole

```nodia
use io
use text
use result

val content = result.raise(io.read("input.txt"))
emit text.upper(content)
```

### Path And Directory Queries

```nodia
use io

emit io.basename("/tmp/report.txt")
emit io.dirname("/tmp/report.txt")
emit io.list_dir("/tmp")
emit io.glob("/tmp/**/*.txt")
```

`list_dir(...)` returns entry names. `glob(...)` returns path strings that
match the pattern.

### Line-By-Line Transform

```nodia
use io
use text
use result

val src = result.raise(io.open("input.txt", "read"))
val out = result.raise(io.open("output.txt", "write"))

var line = result.raise(io.readln(src))
while line != null {
  result.raise(io.writeln(out, text.upper(line)))
  line = result.raise(io.readln(src))
}

result.raise(io.close(src))
result.raise(io.close(out))
```

```bash
./target/release/nodia run upper_file.nod --allow-write
```

### Chunked Read

```nodia
use io
use result

val src = result.raise(io.open("input.txt", "read"))
while not result.raise(io.eof(src)) {
  val chunk = result.raise(io.read(src, 16))
  if chunk != "" {
    emit chunk
  }
}
result.raise(io.close(src))
```

`read(stream, size)` never returns half of a UTF-8 scalar value. If the chunk
boundary would split one, the runtime reads slightly further to return valid
text. Use `text.len(text, text.byte)` when you need to compare chunk size with UTF-8
storage length. `read(stream, 0)` is a no-op that returns `""`.

### Raw Bytes With Explicit Decode

```nodia
use io
use text
use result

val raw = result.raise(io.read("payload.bin", io.bytes))
val cleaned = text.drop_nul(
  text.strip_bom(
    result.raise(text.decode(raw, text.utf8, text.lossy)),
  ),
)
emit text.normalize(cleaned, text.lf)
```

### Stream-Style Stdout

```bash
./target/release/nodia eval '
use io
use result
result.raise(io.write(io.stdout, "hello"))
result.raise(io.write(io.stdout, " world\n"))
'
```

```text
hello world
```

### Stderr Logging

```bash
./target/release/nodia eval 'use io
use result
result.raise(io.writeln(io.stderr, "warning"))'
```

Writes `warning\n` to stderr, leaving the program output channel untouched.

## Errors

| Code     | When                                                       |
| -------- | ---------------------------------------------------------- |
| `E3000`  | underlying IO failure (file not found, permission, etc.)   |
| `E3001`  | file write attempted without `--allow-write`               |
| `E2000`  | misuse (e.g. `read` on a writable stream)                  |
