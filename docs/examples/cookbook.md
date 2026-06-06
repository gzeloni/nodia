# Cookbook

End-to-end examples that you can paste into `eval` or save into a `.nod` file
and run. Each example has been verified with the `0.7.4` release binary.

## 1. Hello, World

```bash
./target/release/nodia eval 'emit "hello, world"'
```

```text
hello, world
```

## 2. Greet From A CLI Variable

`hello.nod`:

```nodia
use text

val name = input.name

emit "Hello, {text.capitalize(name)}"
```

```bash
./target/release/nodia run hello.nod --var name=ana
```

```text
Hello, Ana
```

## 3. Fibonacci

```bash
./target/release/nodia eval '
use numbers
use collections

func fibs(count) {
  var result = []
  var a = 0
  var b = 1

  for i in numbers.range(count) {
    result = collections.push(result, a)
    var next = a + b
    a = b
    b = next
  }

  return result
}

emit fibs(10)
emit numbers.sum(fibs(10))
emit numbers.avg(fibs(10))
'
```

```text
[0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
88
8.8
```

## 4. Markdown Bullet List From A List

```bash
./target/release/nodia eval '
use text

val tags = ["compiler", "formatter", "streams"]

for tag in tags {
  emit "- {text.capitalize(tag)}"
}
'
```

```text
- Compiler
- Formatter
- Streams
```

## 5. Word Frequency Histogram

```bash
./target/release/nodia eval '
use text as txt
use collections

val content = "ana bruno ana carla bruno ana"

var counts = {}
for tok in txt.words(content) {
  counts[tok] = collections.get(counts, tok, 0) + 1
}

for (key, count) in counts {
  emit "{key}={count}"
}
'
```

```text
ana=3
bruno=2
carla=1
```

This uses `collections.get(..., default)` to remove the manual missing-key
branch, while still relying on mutable `var` map bindings, index assignment,
and pair iteration.

## 6. Extract URLs With Regex

```bash
./target/release/nodia eval '
use re

val pat = regex(case_insensitive) {
  named scheme {
    either {
      branch { "http" }
      branch { "https" }
    }
  }
  "://"
  named host {
    one_or_more {
      char_set { letter digit "." "-" }
    }
  }
}

val text = "see http://a.example or https://b.example for details"

for hit in re.find(text, pat, re.all) {
  emit "{hit.named.scheme} -> {hit.named.host}"
}
'
```

```text
http -> a.example
https -> b.example
```

## 7. Sanitize Numbers In A String

```bash
./target/release/nodia eval '
use text

val pat = regex { one_or_more digit }
emit text.replace("ana 42 bruno 77 carla 5", pat, "#")
'
```

```text
ana # bruno # carla #
```

## 8. Split A Path

```bash
./target/release/nodia eval 'use text
emit text.split("/usr/local/bin", "/")'
```

```text
["", "usr", "local", "bin"]
```

## 9. Parse JSON And Emit Structured Fields

```bash
./target/release/nodia eval '
use json
use text

val doc = json.read(text.encode("""
{"name":"Ana","meta":{"count":2},"flags":[true,false]}
""", text.utf8))
emit doc.name
emit doc.meta.count
emit doc.flags
'
```

```text
Ana
2
[true, false]
```

## 10. Read CSV With Headers

```bash
./target/release/nodia eval '
use csv
use text

val rows = csv.read(text.encode("name,role\nAna,dev\n\"Bia, Jr\",ops", text.utf8), true)
emit rows[0].name
emit rows[1]
'
```

```text
Ana
{name: "Bia, Jr", role: "ops"}
```

## 11. Read A File, Uppercase, Write Out

`upper_file.nod`:

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

## 12. Generated Report With Modules

`report/meta.nod`:

```nodia
val title = "Build Report"
val sections = ["summary", "artifacts", "status"]
```

`report/format.nod`:

```nodia
use text as txt

func heading(value) {
  return "== {txt.upper(value)} =="
}

func bullet(value) {
  return "- {value}"
}
```

`main.nod`:

```nodia
use "./report/meta" as meta
use "./report/format" as fmt

emit fmt.heading(meta.title)

for section in meta.sections {
  emit fmt.bullet(section)
}
```

```text
== BUILD REPORT ==
- summary
- artifacts
- status
```

## 13. Date Arithmetic And ISO Formatting

```bash
./target/release/nodia eval '
use datetime

val base = datetime.parse("2024-01-31T23:00:00Z", datetime.as_datetime)
val next = datetime.add(base, datetime.duration({hours: 2, minutes: 30}))

emit datetime.isoformat(datetime.add(datetime.date(2024, 1, 31), 1, datetime.months))
emit datetime.isoformat(next)
emit datetime.strftime(next, "%F %T %:z")
'
```

```text
2024-02-29
2024-02-01T01:30:00Z
2024-02-01 01:30:00 Z
```

## 14. Validate A Date Format

```bash
./target/release/nodia eval '
use re

val date = regex {
  start
  exactly 4 digit
  "-"
  exactly 2 digit
  "-"
  exactly 2 digit
  end
}

emit re.test("2026-05-26", date, re.full)
emit re.test("2026/05/26", date, re.full)
'
```

```text
true
false
```

## 15. Stats Summary

```bash
./target/release/nodia eval '
use numbers
use collections

val values = [3, 1, 4, 1, 5, 9, 2, 6]
val sorted = collections.sort(values)

emit "count={collections.len(values)}"
emit "sum={numbers.sum(values)}"
emit "avg={numbers.avg(values)}"
emit "min={collections.first(sorted)}"
emit "max={collections.last(sorted)}"
'
```

```text
count=8
sum=31
avg=3.875
min=1
max=9
```

## 16. Stream-Style Stdout And Stderr

```bash
./target/release/nodia eval '
use io

io.write(io.stdout, "ready")
io.write(io.stdout, "\n")
io.writeln(io.stderr, "info: started")
'
```

stdout:

```text
ready
```

stderr:

```text
info: started
```

## 17. Detect Duplicate Adjacent Words

```bash
./target/release/nodia eval '
use re

val dup = regex {
  word_boundary
  named word { one_or_more letter }
  one_or_more whitespace
  same_as word
  word_boundary
}

emit re.test("the the cat sat", dup)
emit re.test("the cat sat", dup)
'
```

```text
true
false
```

## 18. Template Replacement

```bash
./target/release/nodia eval '
use text

val tpl = "user=<user> env=<env>"
emit text.replace(text.replace(tpl, "<user>", "ana"), "<env>", "prod")
'
```

```text
user=ana env=prod
```

For literals like `<user>` Nodia interpolation is inert. If you must use
`{name}` style placeholders, escape the braces in the source template with
`{{name}}` — but `replace` is usually cleaner for external templates.

## 19. Format Numeric Columns

```bash
./target/release/nodia eval '
use format

emit format.format("%05d %.2f %-6s", [7, 3.5, "ok"])
emit format.fixed(3.14159, 3)
'
```

```text
00007 3.50 ok    
3.142
```

## 20. Read Script Args And Env

```bash
HOME=/tmp ./target/release/nodia eval '
use system

emit system.args
emit system.env("HOME")
' --allow-env -- one two
```

```text
["one", "two"]
/tmp
```

## 21. Execute A Subprocess

```bash
./target/release/nodia eval '
use system
use text

val result = system.exec("/bin/sh", [
  "-c",
  "printf out; printf err 1>&2; exit 7",
])
emit text.decode(result.stdout, text.utf8)
emit text.decode(result.stderr, text.utf8)
emit result.status
' --allow-process
```

```text
out
err
7
```

## 22. Transform A List With Higher-Order Helpers

```bash
./target/release/nodia eval '
use collections

func double(x) {
  return x * 2
}

func odd(x) {
  return x % 2 != 0
}

func add(acc, x) {
  return acc + x
}

emit collections.map(double, [1, 2, 3])
emit collections.filter(odd, [1, 2, 3, 4])
emit collections.reduce(add, 0, [1, 2, 3, 4])
'
```

```text
[2, 4, 6]
[1, 3]
10
```

## 23. Normalize Messy Line-Oriented Input

```bash
./target/release/nodia eval '
use text
use collections

val collapse = regex { one_or_more whitespace }
val raw = """
  ana    open
# keep for audit
  bruno      closed

// generated note
  carla    pending
"""

var cleaned = []
for line in text.lines(raw) {
  val compact = text.replace(text.trim(line), collapse, " ")
  if compact != "" and not text.starts(compact, "#") and not text.starts(compact, "//") {
    cleaned = collections.push(cleaned, compact)
  }
}

emit cleaned
emit text.join(cleaned, "\n")
'
```

```text
["ana open", "bruno closed", "carla pending"]
ana open
bruno closed
carla pending
```

## 24. Summarize A Noisy Audit Log

```bash
./target/release/nodia eval '
use text
use re
use collections

val entry = regex {
  named level {
    either {
      branch { "INFO" }
      branch { "WARN" }
      branch { "ERROR" }
    }
  }
  one_or_more whitespace
  named user { one_or_more letter }
  one_or_more whitespace
  named action { one_or_more letter }
}

val raw = """
INFO ana deploy
noise
WARN bia retry
ERROR ana deploy
INFO carla sync
"""

var counts = {}
for line in text.lines(raw) {
  val hit = re.find(text.trim(line), entry)
  if hit != null {
    val key = "{hit.named.user}:{hit.named.action}"
    counts[key] = collections.get(counts, key, 0) + 1
  }
}

for key in collections.keys(counts) {
  emit "{key}={counts[key]}"
}
'
```

```text
ana:deploy=2
bia:retry=1
carla:sync=1
```

## 25. Normalize And Compare Unicode Text Explicitly

```bash
./target/release/nodia eval '
use text
use collections

val composed = "é"
val decomposed = "é"

func key(value) {
  return text.casefold(text.normalize(value, text.nfc))
}

emit composed == decomposed
emit text.normalize(composed, text.nfc) == text.normalize(decomposed, text.nfc)
emit text.casefold("Straße") == text.casefold("STRASSE")
emit collections.sort(["Z", "é", "é"])
emit collections.sort_by(key, ["Z", "é", "é"])
'
```

```text
false
true
true
["Z", "é", "é"]
["Z", "é", "é"]
```

## 26. Slice Text By Byte, Scalar, And Grapheme Units

```bash
./target/release/nodia eval '
use text
use collections

val sample = "éx"
emit collections.len(sample)
emit text.len(sample, text.grapheme)
emit text.slice(sample, text.scalar, 0, 2)
emit text.slice(sample, text.grapheme, 0, 1)
emit text.slice("aéb", text.byte, 1, 3)
'
```

```text
3
2
é
é
é
```

## 27. Clean A Messy Byte Buffer Explicitly

```bash
./target/release/nodia eval '
use text

val raw = b"\xef\xbb\xbfa\r\nb\0\xff"
val decoded = text.decode(raw, text.utf8, text.lossy)
emit text.normalize(text.drop_nul(text.strip_bom(decoded)), text.lf)
'
```

```text
a
b�
```

## 28. Format Visible Characters Without Splitting Graphemes

```bash
./target/release/nodia eval '
use format

emit format.format("[%2s][%.1s]", ["é", "éx"])
emit format.pad("é", 2, format.left, ".")
emit format.pad("é", 2, format.right, ".")
'
```

```text
[ é][é]
.é
é.
```
