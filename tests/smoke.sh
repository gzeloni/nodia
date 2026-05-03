#!/usr/bin/env sh
set -eu

BIN="${ORICH_BIN:-target/debug/orich}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

assert_eq() {
    name="$1"
    expected="$2"
    actual="$3"
    if [ "$actual" != "$expected" ]; then
        printf 'FAIL %s\nExpected:\n%s\nActual:\n%s\n' "$name" "$expected" "$actual" >&2
        exit 1
    fi
    printf 'ok %s\n' "$name"
}

cargo build

cat > "$TMP_DIR/input.och" <<'ORICH'
let name = input.name
emit "Hello, {name}"
ORICH
actual="$($BIN run "$TMP_DIR/input.och" --var name=World)"
assert_eq "input interpolation" "Hello, World" "$actual"

cat > "$TMP_DIR/core.och" <<'ORICH'
let age = int(input.age)
const items = [
  "a",
  "b",
  "c",
]
emit "Age next year: {age + 1}"
emit "Second: {items[1]}"
ORICH
actual="$($BIN run "$TMP_DIR/core.och" --var age=30)"
assert_eq "multiline variables arithmetic index" "Age next year: 31
Second: b" "$actual"

cat > "$TMP_DIR/lib.och" <<'ORICH'
const title = "Imported"

fn shout(value) {
  return uppercase(value)
}
ORICH
cat > "$TMP_DIR/imports.och" <<'ORICH'
import './lib' as lib

emit lib.title
emit lib.shout("ok")
ORICH
actual="$($BIN run "$TMP_DIR/imports.och")"
assert_eq "relative import namespace" "Imported
OK" "$actual"

cat > "$TMP_DIR/filters.och" <<'ORICH'
import './lib' show title

emit title
ORICH
actual="$($BIN run "$TMP_DIR/filters.och")"
assert_eq "import show filter" "Imported" "$actual"

cat > "$TMP_DIR/counter.och" <<'ORICH'
let n = 0
ORICH
cat > "$TMP_DIR/counter_main.och" <<'ORICH'
import './counter' show n

while n < 3 {
  emit n
  n = n + 1
}
ORICH
actual="$($BIN run "$TMP_DIR/counter_main.och")"
assert_eq "mutable imported let" "0
1
2" "$actual"

cat > "$TMP_DIR/a.och" <<'ORICH'
import './b' as b
const name = "A"
fn pair() {
  return "{name}/{b.name}"
}
ORICH
cat > "$TMP_DIR/b.och" <<'ORICH'
import './a' as a
const name = "B"
fn pair() {
  return "{name}/{a.name}"
}
ORICH
cat > "$TMP_DIR/circular.och" <<'ORICH'
import './a' as a
import './b' as b

emit a.pair()
emit b.pair()
ORICH
actual="$($BIN run "$TMP_DIR/circular.och")"
assert_eq "circular imports" "A/B
B/A" "$actual"

cat > "$TMP_DIR/stdlib.och" <<'ORICH'
const tags = split("dev,ops,docs", ",")
emit join(tags, "|")
emit contains(tags, "ops")
emit starts_with("orich", "ori")
emit ends_with("orich", "ich")
ORICH
actual="$($BIN run "$TMP_DIR/stdlib.och")"
assert_eq "stdlib text helpers" "dev|ops|docs
true
true
true" "$actual"

cat > "$TMP_DIR/raw.och" <<'ORICH'
const user={name:"Ana",role:"dev"}
if user.name!=""{emit "hello {user.name}"}
ORICH
actual="$($BIN fmt --stdout "$TMP_DIR/raw.och")"
expected='const user = {
  name: "Ana",
  role: "dev",
}

if user.name != "" {
  emit "hello {user.name}"
}'
assert_eq "formatter stdout" "$expected" "$actual"

$BIN fmt "$TMP_DIR/raw.och" >/dev/null
$BIN fmt --check "$TMP_DIR/raw.och" >/dev/null
$BIN check "$TMP_DIR/raw.och" --json | grep '"ok":true' >/dev/null
$BIN tokens "$TMP_DIR/raw.och" --json | grep '"tokens"' >/dev/null
$BIN ast "$TMP_DIR/raw.och" --json | grep '"ast"' >/dev/null

actual="$($BIN eval 'emit join(["a", "b"], ":")')"
assert_eq "eval command" "a:b" "$actual"

mkdir "$TMP_DIR/project"
$BIN init "$TMP_DIR/project" >/dev/null
actual="$(cd "$TMP_DIR/project" && "$OLDPWD/$BIN" run --var name=Project)"
assert_eq "project entry discovery" "Hello, Project" "$actual"
