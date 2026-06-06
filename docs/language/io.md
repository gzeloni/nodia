# IO & Streams

Nodia v0.7 has real file IO and stream values. Import `use io` to access
standard streams and file operations.

## Standard Streams

| Binding      | Meaning                          |
| ------------ | -------------------------------- |
| `io.stdin`   | process standard input           |
| `io.stdout`  | program output (same as `emit`)  |
| `io.stderr`  | process standard error           |

```nodia
use io

io.writeln(io.stdout, "What is your name?")
val name = io.readln(io.stdin)
io.writeln(io.stdout, "Hello, {name}")
```

## File Streams

### `open(path, mode)`

Opens a file stream. Modes:

| Mode      | Meaning                                       |
| --------- | --------------------------------------------- |
| `"read"`  | open existing file for reading                |
| `"write"` | create/truncate file for writing              |
| `"append"`| create/open file and append writes            |

Reading:

```nodia
use io

val src = io.open("input.txt", "read")
val text = io.read(src)
io.close(src)
emit text
```

Writing (requires `--allow-write`):

```nodia
use io

val out = io.open("output.txt", "write")
io.writeln(out, "first")
io.writeln(out, "second")
io.close(out)
```

```bash
./target/release/nodia run write.nod --allow-write
```

Appending:

```nodia
use io

val log = io.open("app.log", "append")
io.writeln(log, "started")
io.close(log)
```

### `close(stream)`

Closes a stream. Closing a writable stream also flushes pending writes.

Closing one of `io.stdin` / `io.stdout` / `io.stderr` is accepted as a no-op or
flush-equivalent operation; you do not need to close standard streams in
normal scripts.

### `flush(stream)`

Flushes a writable stream without closing it:

```nodia
use io

val out = io.open("out.txt", "write")
io.write(out, "partial")
io.flush(out)
io.close(out)
```

### `eof(stream)`

Returns whether a readable file stream has reached EOF. EOF becomes true
after a read operation reaches the end:

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

For line-oriented reads, prefer the simpler `readln(stream) != null` style.

## Reading

### `read(path)`

Read an entire file into a string. Does **not** require `--allow-write`:

```nodia
use io
use text

val content = io.read("input.txt")
emit text.upper(content)
```

All `read(...)` forms in Nodia are **UTF-8 strict**. Invalid UTF-8 is an
`E3000` IO error; it is never silently replaced with lossy text.

### `read(stream)`

Read the rest of a readable stream:

```nodia
use io

val src = io.open("input.txt", "read")
val text = io.read(src)
io.close(src)
```

### `read(stream, size)`

Read a chunk from a readable stream. `size` is a non-negative integer byte
budget:

```nodia
use io

val src = io.open("input.txt", "read")
emit io.read(src, 8)
emit io.read(src, 8)
io.close(src)
```

The returned text is always valid UTF-8. If the requested byte budget lands in
the middle of a multi-byte scalar value, the runtime reads a little further to
finish that scalar instead of splitting it. Use `byte_len(text)` when you need
to compare chunk sizes with UTF-8 storage length. `read(stream, 0)` returns
`""` without advancing the stream or forcing EOF.

### `readln(stream)`

Read one line and strip the line ending. Returns `null` at EOF:

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

Like the other text-reading forms, `readln(stream)` rejects invalid UTF-8 with
`E3000` instead of decoding lossily.

## Writing

All file writes require `--allow-write`. Writes to `io.stdout` / `io.stderr`
do not.

### `write(path, text)`

Write a whole file, replacing any previous content:

```nodia
use io

io.write("out.txt", "hello\n")
```

### `write(stream, text)`

Write to a stream without adding a newline:

```nodia
use io

val out = io.open("out.txt", "write")
io.write(out, "hello")
io.write(out, " world")
io.close(out)
```

Stream-style stdout:

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

### `writeln(stream, text)`

Write text and a newline:

```nodia
use io

val out = io.open("out.txt", "write")
io.writeln(out, "hello")
io.writeln(out, "world")
io.close(out)
```

### `append(path, text)`

Append text to a file:

```nodia
use io

io.append("app.log", "started\n")
```

Requires `--allow-write`.

## Write Permission

File-writing builtins (`write(path, ...)`, `append(path, ...)`,
`open(path, "write" | "append")`) require the CLI flag:

```bash
nodia run script.nod --allow-write
```

Without permission, the runtime fails with:

```text
error[E3001]: file write requires --allow-write
```

This is by design — Nodia treats file writes as a privileged effect.

## Paths

CLI commands resolve file IO paths from the current working directory of the
`nodia` process. This is the same convention as shell tools. Module `use`
paths are different — they resolve from the file that contains the `use`
(see [Modules](modules.md)).

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

```bash
./target/release/nodia run upper_file.nod --allow-write
```

The script reads `input.txt` line by line and writes an uppercase copy to
`output.txt`.
