# IO & Streams

Import `use io` for streams and file operations.

Operational IO builtins return their direct success value. On failure, they
raise a runtime error that can be caught with `try` / `catch`.

```nodia
use io

val src = io.open("input.txt", "read")
val text = io.read(src)
io.close(src)
emit text
```

Contract misuse still stays fatal: wrong arity, invalid mode names, reading
from a writable stream, writing raw bytes to `io.stdout`, and similar mistakes
remain `E2000`.

## Standard Streams

| Binding | Meaning |
| --- | --- |
| `io.stdin` | process standard input |
| `io.stdout` | program output channel |
| `io.stderr` | process standard error |

```nodia
use io

io.writeln(io.stdout, "What is your name?")
val name = io.readln(io.stdin)
io.writeln(io.stdout, "Hello, {name}")
```

## File Streams

### `open(path, mode)`

Opens a file stream and returns the stream value.

| Mode | Meaning |
| --- | --- |
| `"read"` | open an existing file for reading |
| `"write"` | create or truncate a file for writing |
| `"append"` | create or open a file and append writes |

```nodia
use io

val out = io.open("output.txt", "write")
io.writeln(out, "first")
io.writeln(out, "second")
io.close(out)
```

### `close(stream)` / `flush(stream)` / `eof(stream)`

These builtins also return their direct success value.

* `close(...)` flushes writable streams before closing.
* `flush(...)` forces buffered output without closing.
* `eof(...)` reports whether a readable file stream has reached end-of-file.

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

## Reading

| Builtin | Success value |
| --- | --- |
| `io.read(path)` | full text |
| `io.read(stream)` | remaining text |
| `io.read(stream, size)` | one UTF-8-safe text chunk |
| `io.read(path, io.bytes)` | full `bytes` |
| `io.read(stream, io.bytes)` | remaining `bytes` |
| `io.read(stream, io.bytes, size)` | up to `size` raw bytes |
| `io.readln(stream)` | one line, or `null` at EOF |

Text readers are UTF-8 strict. Invalid bytes raise `E3000`.
When you need undecoded input, use `io.read(..., io.bytes)` and decode
explicitly with `text.decode(...)`.

```nodia
use io
use text

val content = io.read("input.txt")
emit text.upper(content)
```

`io.read(stream, size)` uses `size` as a byte budget, but it never returns half
of one UTF-8 scalar value. The runtime may read slightly past the budget to
finish the current scalar cleanly.

```nodia
use io

val src = io.open("input.txt", "read")
emit io.read(src, 8)
emit io.read(src, 8)
io.close(src)
```

Line reads strip trailing `\n` or `\r\n`:

```nodia
use io

val src = io.open("input.txt", "read")

var line = io.readln(src)
while line != null {
  emit line
  line = io.readln(src)
}

io.close(src)
```

Raw-byte reads keep decode choices explicit:

```nodia
use io
use text

val raw = io.read("input.bin", io.bytes)
emit text.decode(raw, text.utf8, text.lossy)
```

## Writing

All file-writing operations require `--allow-write`.

| Builtin | Behavior |
| --- | --- |
| `io.write(path, value)` | replace a file with text or bytes |
| `io.write(stream, value)` | write text or bytes to a stream |
| `io.writeln(stream, text)` | write text plus newline |
| `io.append(path, value)` | append text or bytes to a file |

Each returns its direct success value.

```nodia
use io

io.write("payload.bin", b"\0\x01\x02\xff")
io.append("payload.bin", b"\n")
```

`io.stdout` remains a text-output channel, so `io.write(io.stdout, b"...")` is
rejected deliberately.

Without `--allow-write`, the call returns:

```text
error[E3001]: file write requires --allow-write
```

## Paths

IO paths resolve from the current working directory of the `nodia` process.
This is different from module `use` paths, which resolve from the file that
declares the `use`.

## End-To-End Example

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
