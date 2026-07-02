# Scanner Builtins

Import this namespace with `use scan`.

`scan` is the low-level parsing toolkit for semi-structured text. It does not
try to be a full PEG or parser framework. It gives you a mutable scanner
cursor, explicit positions/spans, literal or regex prefix matching, and
recoverable parse diagnostics.

Patterns accepted by `scan.match`, `scan.expect`, `scan.take_while`, and
`scan.take_until` can be either:

* a literal string; or
* a compiled `regex { ... }` value.

Regex scanner patterns must consume at least one character when they match.
Empty-match regexes are rejected.

## Cursor

### `cursor(text)`

Creates a scanner over one string:

```bash
./target/release/nodia eval '
use scan

val s = scan.cursor("abc")
emit scan.lookahead(s)
emit scan.advance(s)
emit scan.lookahead(s, 2)
'
```

```text
a
a
bc
```

### `at_end(scanner)`

Reports whether the cursor reached the end of the input:

```bash
./target/release/nodia eval '
use scan

val s = scan.cursor("a")
emit scan.at_end(s)
scan.advance(s)
emit scan.at_end(s)
'
```

```text
false
true
```

## Positions And Spans

### `pos(scanner)`

Returns the current cursor position as:

```nodia
{
  offset: 0,
  line: 1,
  column: 1,
}
```

`offset` is a Unicode scalar offset, not a byte offset.

### `span(scanner, start)`

Builds a span from a previously saved position to the current cursor:

```bash
./target/release/nodia eval '
use scan

val s = scan.cursor("alpha=42")
val start = scan.pos(s)
scan.take_until(s, "=")
emit scan.span(s, start)
'
```

```text
{end: {column: 6, line: 1, offset: 5}, start: {column: 1, line: 1, offset: 0}, text: "alpha"}
```

The returned span map always has:

* `text`
* `start`
* `end`

Where `start` and `end` are position maps with `offset`, `line`, and `column`.

## Primitive Cursor Moves

### `lookahead(scanner)` / `lookahead(scanner, count)`

Peeks without consuming. At EOF it returns `null`.

### `advance(scanner)` / `advance(scanner, count)`

Consumes and returns raw text. If the cursor is already at EOF, it returns `""`.

```bash
./target/release/nodia eval '
use scan

val s = scan.cursor("nodia")
emit scan.lookahead(s, 3)
emit scan.advance(s, 2)
emit scan.lookahead(s, 3)
'
```

```text
nod
no
dia
```

## Basic Combinators

### `match(scanner, pattern)`

Consumes one prefix match and returns its span, or `null` when the prefix does
not match:

```bash
./target/release/nodia eval '
use scan

val s = scan.cursor("abc123")
emit scan.match(s, regex { one_or_more letter }).text
emit scan.match(s, regex { one_or_more digit }).text
'
```

```text
abc
123
```

### `expect(scanner, pattern)` / `expect(scanner, pattern, label)`

Same as `match(...)`, but throws `E4300` when the prefix does not match:

```bash
./target/release/nodia eval '
use scan

try {
  val s = scan.cursor("name 42")
  scan.take_until(s, " ")
  scan.expect(s, "=", "\"=\" after key")
} catch err {
  emit err.code
  emit err.context
  emit "{err.span.line}:{err.span.column}"
}
'
```

```text
E4300
["scan.expect"]
1:5
```

### `take_while(scanner, pattern)`

Consumes repeated prefix matches and returns one combined span. It may return an
empty span when nothing matched:

```bash
./target/release/nodia eval '
use scan

val s = scan.cursor("   abc")
emit scan.take_while(s, " ").text
emit scan.expect(s, regex { one_or_more letter }, "identifier").text
'
```

```text
   
abc
```

### `take_until(scanner, pattern)`

Consumes text until the next prefix match of `pattern`, without consuming the
delimiter itself:

```bash
./target/release/nodia eval '
use scan

val s = scan.cursor("left=right")
emit scan.take_until(s, "=").text
scan.expect(s, "=")
emit scan.take_until(s, "x").text
'
```

```text
left
right
```

## Token Helper

### `token(kind, span)`

Wraps a span as a token map:

```bash
./target/release/nodia eval '
use scan

val s = scan.cursor("key=value")
val key = scan.take_until(s, "=")
emit scan.token("ident", key)
'
```

```text
{kind: "ident", span: {end: {column: 4, line: 1, offset: 3}, start: {column: 1, line: 1, offset: 0}, text: "key"}, text: "key"}
```

## Custom Parse Failure

### `error(scanner, message)`

Throws a recoverable parse error at the current cursor position:

```nodia
use scan

val s = scan.cursor("???")
scan.error(s, "unexpected header")
```

This throws `E4300` with `context = ["scan.error"]`.

## Examples

### Log Line Parser

```nodia
use scan

func parse_log(line) {
  val s = scan.cursor(line)
  val stamp = scan.take_until(s, " ")
  scan.expect(s, " ", "space after timestamp")
  val level = scan.take_until(s, " ")
  scan.expect(s, " ", "space after level")

  val body_start = scan.pos(s)
  while not scan.at_end(s) {
    scan.advance(s)
  }
  val body = scan.span(s, body_start)

  return {
    stamp: stamp.text,
    level: level.text,
    body: body.text,
  }
}

emit parse_log("2026-06-29 INFO boot complete")
```

### Mini-Config Parser

This example intentionally combines stage 3 and 4: `io.lines(...)` streams the
file lazily, and `scan` parses each line.

```nodia
use io
use scan

func parse_config(path) {
  val src = io.open(path, "read")
  var out = {}

  for line in io.lines(src) {
    val s = scan.cursor(line)
    scan.take_while(s, " ")
    if scan.at_end(s) {
      continue
    }
    if scan.lookahead(s) == "#" {
      continue
    }
    val key = scan.take_until(s, "=")
    scan.expect(s, "=", "\"=\" after key")
    val value_start = scan.pos(s)
    while not scan.at_end(s) {
      scan.advance(s)
    }
    val value = scan.span(s, value_start)
    out[key.text] = value.text
  }

  io.close(src)
  return out
}
```

### Delimited Block Parser

```nodia
use scan

func parse_block(text) {
  val s = scan.cursor(text)
  scan.expect(s, "<<<", "opening marker")
  scan.expect(s, "\n", "newline after opening marker")
  val body = scan.take_until(s, ">>>")
  scan.expect(s, ">>>", "closing marker")
  return body.text
}

emit parse_block("<<<\nalpha\nbeta\n>>>tail")
```

## Notes

* Scanner positions and spans are meant to stay inside one scanner instance.
* `scan` is single-pass and mutable by design.
* Prefer regex when one pattern is enough. Prefer `scan` when you need staged,
  readable, span-aware parsing.
