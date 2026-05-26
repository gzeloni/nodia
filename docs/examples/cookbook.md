# Cookbook

End-to-end examples that you can paste into `eval` or save into a `.nod` file
and run. Each example has been verified with the `0.6.0` release binary.

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
  if contains(counts, tok) {
    counts[tok] = counts[tok] + 1
  } else {
    counts[tok] = 1
  }
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

This relies on three features that landed together: mutable `var` map
bindings, index assignment (`counts[tok] = ...`), and pair iteration
(`for (key, count) in counts`).

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
[, usr, local, bin]
```

## 9. Read A File, Uppercase, Write Out

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

## 10. Generated Report With Modules

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

## 11. Validate A Date Format

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

## 12. Stats Summary

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

## 13. Stream-Style Stdout And Stderr

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

## 14. Detect Duplicate Adjacent Words

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

## 15. Template Replacement

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
