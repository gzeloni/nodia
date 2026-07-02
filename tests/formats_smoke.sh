#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo build --release >/dev/null

run_case() {
  source="$1"
  expected="$2"
  actual="$("./target/release/nodia" -e "$source")"
  if [ "$actual" != "$expected" ]; then
    echo "formats smoke test failed" >&2
    echo "expected:" >&2
    printf '%s\n' "$expected" >&2
    echo "actual:" >&2
    printf '%s\n' "$actual" >&2
    exit 1
  fi
}

run_case "$(cat <<'EOF'
use toml

val doc = toml.read("""
name = "Ana"
active = true
scores = [7, 8]
[meta]
city = "Rio"
""")
emit doc.meta.city
emit doc.scores[1]
emit toml.write(doc)
EOF
)" "Rio
8
active = true
name = \"Ana\"
scores = [7, 8]

[meta]
city = \"Rio\""

run_case "$(cat <<'EOF'
use markdown

val doc = markdown.read("""
# Hello

- one
- two
""")
emit doc[0].kind
emit doc[1].items[1]
emit markdown.write(doc)
EOF
)" "heading
two
# Hello

- one
- two"

run_case "$(cat <<'EOF'
use html
use xml
use diff

val page = html.read("""<main id="x"><h1>Hello</h1><!--ok--></main>""")
emit page[0].attrs.id
emit page[0].children[0].name
emit html.write(page)

val feed = xml.read("""<note><to>Ana</to></note>""")
emit feed[0].children[0].children[0].text
emit xml.write(feed)

val patch = diff.read("""--- a.txt
+++ b.txt
@@ -1 +1 @@
-old
+new
""")
emit patch.files[0].hunks[0].lines[1].text
emit diff.write(patch)
EOF
)" "x
h1
<main id=\"x\"><h1>Hello</h1><!--ok--></main>
Ana
<note><to>Ana</to></note>
new
--- a.txt
+++ b.txt
@@ -1 +1 @@
-old
+new"

run_case "$(cat <<'EOF'
use toml
use html
use diff

try {
  toml.read("name")
} catch err {
  emit err.context[0]
}

try {
  html.read("<main>")
} catch err {
  emit err.context[0]
}

try {
  diff.read("--- a.txt")
} catch err {
  emit err.context[0]
}
EOF
)" "toml.read
html.read
diff.read"
