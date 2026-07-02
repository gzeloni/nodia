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
    echo "yaml smoke test failed" >&2
    echo "expected:" >&2
    printf '%s\n' "$expected" >&2
    echo "actual:" >&2
    printf '%s\n' "$actual" >&2
    exit 1
  fi
}

run_case "$(cat <<'EOF'
use yaml

val doc = yaml.read("""
name: Ana
scores:
  - 7
  - 8
meta:
  city: Rio
""")
emit doc.meta.city
emit doc.scores[1]
emit yaml.write(doc)
EOF
)" "Rio
8
meta:
  city: Rio
name: Ana
scores:
  - 7
  - 8"

run_case "$(cat <<'EOF'
use yaml
use text

val doc = yaml.read(text.encode("""
name: Ana
age: 30
""", text.utf8))
emit doc.name
emit doc.age + 5
EOF
)" "Ana
35"

run_case "$(cat <<'EOF'
use yaml

val rows = yaml.read("""
-
  name: Ana
  age: 30
-
  name: Bia
  age: 25
""")
emit rows[1].name
emit yaml.write(rows)
EOF
)" "Bia
-
  age: 30
  name: Ana
-
  age: 25
  name: Bia"

run_case "$(cat <<'EOF'
use yaml

try {
  yaml.read("- name: Ana")
} catch err {
  emit err.code
  emit err.context
  emit "{err.span.line}:{err.span.column}"
}
EOF
)" "E2000
[\"yaml.read\"]
1:3"
