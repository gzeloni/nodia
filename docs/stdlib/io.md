# IO Builtins

This page is the quick reference for IO builtins. For the narrative of how
file IO and streams work in Nodia (including `--allow-write`), see
[IO & Streams](../language/io.md).

Import this namespace with `use io`.

## Standard Streams

| Binding      | Meaning                          |
| ------------ | -------------------------------- |
| `io.stdin`   | process standard input           |
| `io.stdout`  | program output (same as `emit`)  |
| `io.stderr`  | process standard error           |

## File Streams

| Builtin                | Behavior                                                  |
| ---------------------- | --------------------------------------------------------- |
| `open(path, mode)`     | open a stream; modes `"read"`, `"write"`, `"append"`      |
| `close(stream)`        | close a stream; flushes writes                            |
| `flush(stream)`        | flush a writable stream                                   |
| `eof(stream)`          | returns whether a readable file stream is at EOF          |

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
| `read(path)`           | read a whole file into a string                         |
| `read(stream)`         | read the rest of a readable stream                      |
| `read(stream, size)`   | read a UTF-8-safe chunk using `size` as a byte budget   |
| `read_bytes(path)`     | read a whole file into `list<int>` bytes                |
| `read_bytes(stream)`   | read the rest of a readable stream into bytes           |
| `read_bytes(stream, size)` | read up to `size` raw bytes without UTF-8 decoding |
| `readln(stream)`       | read one line; `null` at EOF                            |

`readln(stream)` strips a trailing `\n` or `\r\n` and still returns the final
line when the file does not end with a newline.
All text readers are UTF-8 strict: invalid bytes fail with `E3000`.
When you need undecoded input, use `read_bytes(...)` and choose
`text.decode_utf8(...)` or `text.decode_utf8_lossy(...)` explicitly.

## Writing

All file writes (where `path` or `mode = "write"/"append"` is involved) require
`--allow-write`.

| Builtin                      | Behavior                                            |
| ---------------------------- | --------------------------------------------------- |
| `write(path, text)`          | write a whole file, replacing it                    |
| `write(stream, text)`        | write text to a stream (no newline added)           |
| `writeln(stream, text)`      | write text and a newline to a stream                |
| `append(path, text)`         | append text to a file                               |
| `write_bytes(path, bytes)`   | write raw bytes, replacing existing content         |
| `write_bytes(stream, bytes)` | write raw bytes to a writable file stream or stderr |
| `append_bytes(path, bytes)`  | append raw bytes to a file                          |

`write_bytes(io.stdout, ...)` is rejected on purpose. `io.stdout` is still the
program text-output channel used by `emit`, so raw bytes must go to files or
`io.stderr`.

## Examples

### Read A File Whole

```nodia
use io
use text

val content = io.read("input.txt")
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

val src = io.open("input.txt", "read")
val out = io.open("output.txt", "write")

var line = io.readln(src)
while line != null {
  io.writeln(out, text.upper(line))
  line = io.readln(src)
}

io.close(src)
io.close(out)
```

```bash
./target/release/nodia run upper_file.nod --allow-write
```

### Chunked Read

```nodia
use io

val src = io.open("input.txt", "read")
while not io.eof(src) {
  val chunk = io.read(src, 16)
  if chunk != "" {
    emit chunk
  }
}
io.close(src)
```

`read(stream, size)` never returns half of a UTF-8 scalar value. If the chunk
boundary would split one, the runtime reads slightly further to return valid
text. Use `byte_len(text)` when you need to compare chunk size with UTF-8
storage length. `read(stream, 0)` is a no-op that returns `""`.

### Raw Bytes With Explicit Decode

```nodia
use io
use text

val raw = io.read_bytes("payload.bin")
val cleaned = text.drop_nul(
  text.strip_bom(
    text.decode_utf8_lossy(raw),
  ),
)
emit text.normalize_lf(cleaned)
```

### Stream-Style Stdout

```bash
./target/release/nodia eval '
use io
io.write(io.stdout, "hello")
io.write(io.stdout, " world\n")
'
```

```text
hello world
```

### Stderr Logging

```bash
./target/release/nodia eval 'use io
io.writeln(io.stderr, "warning")'
```

Writes `warning\n` to stderr, leaving the program output channel untouched.

## Errors

| Code     | When                                                       |
| -------- | ---------------------------------------------------------- |
| `E3000`  | underlying IO failure (file not found, permission, etc.)   |
| `E3001`  | file write attempted without `--allow-write`               |
| `E2000`  | misuse (e.g. `read` on a writable stream)                  |
