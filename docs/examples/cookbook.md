# Cookbook

End-to-end examples that you can paste into `eval` or save into a `.nod` file
and run. Each example has been verified with the `0.6.4` release binary.

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
val name = input.name

emit "Hello, {capitalize(name)}"
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
func fibs(count) {
  var result = []
  var a = 0
  var b = 1

  for i in range(count) {
    result = push(result, a)
    var next = a + b
    a = b
    b = next
  }

  return result
}

emit fibs(10)
emit sum(fibs(10))
emit avg(fibs(10))
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

## 5. Word Frequency Histogram

```bash
./target/release/nodia eval '
val text = "ana bruno ana carla bruno ana"

var counts = {}
for tok in words(text) {
  counts[tok] = get(counts, tok, 0) + 1
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

This uses `get(..., default)` to remove the manual missing-key branch, while
still relying on mutable `var` map bindings, index assignment, and pair
iteration.

## 6. Extract URLs With Regex

```bash
./target/release/nodia eval '
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

for hit in find_all(text, pat) {
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
val pat = regex { one_or_more digit }
emit replace("ana 42 bruno 77 carla 5", pat, "#")
'
```

```text
ana # bruno # carla #
```

## 8. Split A Path

```bash
./target/release/nodia eval 'emit split("/usr/local/bin", "/")'
```

```text
["", "usr", "local", "bin"]
```

## 9. Parse JSON And Emit Structured Fields

```bash
./target/release/nodia eval '
use json

val doc = json.read("""
{"name":"Ana","meta":{"count":2},"flags":[true,false]}
""")
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

val rows = csv.read("name,role\nAna,dev\n\"Bia, Jr\",ops", true)
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

## 12. Generated Report With Modules

`report/meta.nod`:

```nodia
val title = "Build Report"
val sections = ["summary", "artifacts", "status"]
```

`report/format.nod`:

```nodia
func heading(text) {
  return "== {upper(text)} =="
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
val base = parse_datetime("2024-01-31T23:00:00Z")
val next = add_duration(base, duration({hours: 2, minutes: 30}))

emit isoformat(add_months(date(2024, 1, 31), 1))
emit isoformat(next)
emit strftime(next, "%F %T %:z")
'
```

```text
2024-02-29
2024-02-01T01:30:00Z
2024-02-01 01:30:00 Z
```

## 13. Validate A Date Format

```bash
./target/release/nodia eval '
val date = regex {
  start
  exactly 4 digit
  "-"
  exactly 2 digit
  "-"
  exactly 2 digit
  end
}

emit full_match("2026-05-26", date)
emit full_match("2026/05/26", date)
'
```

```text
true
false
```

## 14. Stats Summary

```bash
./target/release/nodia eval '
val numbers = [3, 1, 4, 1, 5, 9, 2, 6]
val sorted = sort(numbers)

emit "count={len(numbers)}"
emit "sum={sum(numbers)}"
emit "avg={avg(numbers)}"
emit "min={first(sorted)}"
emit "max={last(sorted)}"
'
```

```text
count=8
sum=31
avg=3.875
min=1
max=9
```

## 15. Stream-Style Stdout And Stderr

```bash
./target/release/nodia eval '
write(stdout, "ready")
write(stdout, "\n")
writeln(stderr, "info: started")
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

## 16. Detect Duplicate Adjacent Words

```bash
./target/release/nodia eval '
val dup = regex {
  word_boundary
  named word { one_or_more letter }
  one_or_more whitespace
  same_as word
  word_boundary
}

emit test("the the cat sat", dup)
emit test("the cat sat", dup)
'
```

```text
true
false
```

## 17. Template Replacement

```bash
./target/release/nodia eval '
val tpl = "user=<user> env=<env>"
emit replace(replace(tpl, "<user>", "ana"), "<env>", "prod")
'
```

```text
user=ana env=prod
```

For literals like `<user>` Nodia interpolation is inert. If you must use
`{name}` style placeholders, escape the braces in the source template with
`{{name}}` — but `replace` is usually cleaner for external templates.

## 18. Format Numeric Columns

```bash
./target/release/nodia eval '
emit format("%05d %.2f %-6s", [7, 3.5, "ok"])
emit fixed(3.14159, 3)
'
```

```text
00007 3.50 ok    
3.142
```

## 19. Read Script Args And Env

```bash
HOME=/tmp ./target/release/nodia eval '
emit args
emit env("HOME")
' --allow-env -- one two
```

```text
["one", "two"]
/tmp
```

## 20. Execute A Subprocess

```bash
./target/release/nodia eval '
val result = exec("/bin/sh", [
  "-c",
  "printf out; printf err 1>&2; exit 7",
])
emit result.stdout
emit result.stderr
emit result.status
' --allow-process
```

```text
out
err
7
```

## 21. Transform A List With Higher-Order Helpers

```bash
./target/release/nodia eval '
func double(x) {
  return x * 2
}

func odd(x) {
  return x % 2 != 0
}

func add(acc, x) {
  return acc + x
}

emit map(double, [1, 2, 3])
emit filter(odd, [1, 2, 3, 4])
emit reduce(add, 0, [1, 2, 3, 4])
'
```

```text
[2, 4, 6]
[1, 3]
10
```
