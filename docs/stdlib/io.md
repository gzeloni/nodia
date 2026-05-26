# IO Builtins

This page is the quick reference for IO builtins. For the narrative of how
file IO and streams work in Nodia (including `--allow-write`), see
[IO & Streams](../language/io.md).

## Standard Streams

| Binding   | Meaning                          |
| --------- | -------------------------------- |
| `stdin`   | process standard input           |
| `stdout`  | program output (same as `emit`)  |
| `stderr`  | process standard error           |

## File Streams

| Builtin                | Behavior                                                  |
| ---------------------- | --------------------------------------------------------- |
| `open(path, mode)`     | open a stream; modes `"read"`, `"write"`, `"append"`      |
| `close(stream)`        | close a stream; flushes writes                            |
| `flush(stream)`        | flush a writable stream                                   |
| `eof(stream)`          | returns whether a readable file stream is at EOF          |

## Reading

| Builtin                | Behavior                                                |
| ---------------------- | ------------------------------------------------------- |
| `read(path)`           | read a whole file into a string                         |
| `read(stream)`         | read the rest of a readable stream                      |
| `read(stream, size)`   | read a chunk of `size` bytes from a readable stream     |
| `readln(stream)`       | read one line; `null` at EOF                            |

## Writing

All file writes (where `path` or `mode = "write"/"append"` is involved) require
`--allow-write`.

| Builtin                      | Behavior                                            |
| ---------------------------- | --------------------------------------------------- |
| `write(path, text)`          | write a whole file, replacing it                    |
| `write(stream, text)`        | write text to a stream (no newline added)           |
| `writeln(stream, text)`      | write text and a newline to a stream                |
| `append(path, text)`         | append text to a file                               |

## Examples

### Read A File Whole

```nodia
val text = read("input.txt")
emit upper(text)
```

### Line-By-Line Transform

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

### Chunked Read

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

### Stream-Style Stdout

```bash
./target/release/nodia eval '
write(stdout, "hello")
write(stdout, " world\n")
'
```

```text
hello world
```

### Stderr Logging

```bash
./target/release/nodia eval 'writeln(stderr, "warning")'
```

Writes `warning\n` to stderr, leaving the program output channel untouched.

## Errors

| Code     | When                                                       |
| -------- | ---------------------------------------------------------- |
| `E3000`  | underlying IO failure (file not found, permission, etc.)   |
| `E3001`  | file write attempted without `--allow-write`               |
| `E2000`  | misuse (e.g. `read` on a writable stream)                  |
