# IO & Streams

Nodia v0.6 has real file IO and stream values. Standard streams are bound
as identifiers; file streams are opened with `open(path, mode)`.

## Standard Streams

| Binding   | Meaning                       |
| --------- | ----------------------------- |
| `stdin`   | process standard input        |
| `stdout`  | program output (same as `emit`) |
| `stderr`  | process standard error        |

```nodia
writeln(stdout, "What is your name?")
val name = readln(stdin)
writeln(stdout, "Hello, {name}")
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
val src = open("input.txt", "read")
val text = read(src)
close(src)
emit text
```

Writing (requires `--allow-write`):

```nodia
val out = open("output.txt", "write")
writeln(out, "first")
writeln(out, "second")
close(out)
```

```bash
./target/release/nodia run write.nod --allow-write
```

Appending:

```nodia
val log = open("app.log", "append")
writeln(log, "started")
close(log)
```

### `close(stream)`

Closes a stream. Closing a writable stream also flushes pending writes.

Closing one of `stdin` / `stdout` / `stderr` is accepted as a no-op or
flush-equivalent operation; you do not need to close standard streams in
normal scripts.

### `flush(stream)`

Flushes a writable stream without closing it:

```nodia
val out = open("out.txt", "write")
write(out, "partial")
flush(out)
close(out)
```

### `eof(stream)`

Returns whether a readable file stream has reached EOF. EOF becomes true
after a read operation reaches the end:

```nodia
val src = open("input.txt", "read")
while not eof(src) {
  val chunk = read(src, 16)
  if chunk != "" {
    emit chunk
  }
}
close(src)
```

For line-oriented reads, prefer the simpler `readln(stream) != null` style.

## Reading

### `read(path)`

Read an entire file into a string. Does **not** require `--allow-write`:

```nodia
val text = read("input.txt")
emit upper(text)
```

### `read(stream)`

Read the rest of a readable stream:

```nodia
val src = open("input.txt", "read")
val text = read(src)
close(src)
```

### `read(stream, size)`

Read a chunk from a readable stream. `size` is a non-negative integer byte
count:

```nodia
val src = open("input.txt", "read")
emit read(src, 8)
emit read(src, 8)
close(src)
```

### `readln(stream)`

Read one line and strip the line ending. Returns `null` at EOF:

```nodia
val src = open("input.txt", "read")

var line = readln(src)
while line != null {
  emit line
  line = readln(src)
}

close(src)
```

## Writing

All file writes require `--allow-write`. Writes to `stdout` / `stderr` do not.

### `write(path, text)`

Write a whole file, replacing any previous content:

```nodia
write("out.txt", "hello\n")
```

### `write(stream, text)`

Write to a stream without adding a newline:

```nodia
val out = open("out.txt", "write")
write(out, "hello")
write(out, " world")
close(out)
```

Stream-style stdout:

```bash
./target/release/nodia eval '
write(stdout, "hello")
write(stdout, " world\n")
'
```

```text
hello world
```

### `writeln(stream, text)`

Write text and a newline:

```nodia
val out = open("out.txt", "write")
writeln(out, "hello")
writeln(out, "world")
close(out)
```

### `append(path, text)`

Append text to a file:

```nodia
append("app.log", "started\n")
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
val src = open("input.txt", "read")
val out = open("output.txt", "write")

var line = readln(src)
while line != null {
  writeln(out, upper(line))
  line = readln(src)
}

close(src)
close(out)
```

```bash
./target/release/nodia run upper_file.nod --allow-write
```

The script reads `input.txt` line by line and writes an uppercase copy to
`output.txt`.
