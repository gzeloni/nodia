#!/usr/bin/env sh
set -eu

BIN="${DOBRA_BIN:-target/debug/dobra}"
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

cat > "$TMP_DIR/input.dob" <<'DOBRA'
let name = input.name
emit "Hello, {name}"
DOBRA
actual="$($BIN run "$TMP_DIR/input.dob" --var name=World)"
assert_eq "input interpolation" "Hello, World" "$actual"

cat > "$TMP_DIR/core.dob" <<'DOBRA'
let age = int(input.age)
const items = [
  "a",
  "b",
  "c",
]
emit "Age next year: {age + 1}"
emit "Second: {items[1]}"
DOBRA
actual="$($BIN run "$TMP_DIR/core.dob" --var age=30)"
assert_eq "multiline variables arithmetic index" "Age next year: 31
Second: b" "$actual"

cat > "$TMP_DIR/lib.dob" <<'DOBRA'
const title = "Imported"

fn shout(value) {
  return upper(value)
}
DOBRA
cat > "$TMP_DIR/imports.dob" <<'DOBRA'
import './lib' as lib

emit lib.title
emit lib.shout("ok")
DOBRA
actual="$($BIN run "$TMP_DIR/imports.dob")"
assert_eq "relative import namespace" "Imported
OK" "$actual"

cat > "$TMP_DIR/filters.dob" <<'DOBRA'
import './lib' show title

emit title
DOBRA
actual="$($BIN run "$TMP_DIR/filters.dob")"
assert_eq "import show filter" "Imported" "$actual"

cat > "$TMP_DIR/counter.dob" <<'DOBRA'
let n = 0
DOBRA
cat > "$TMP_DIR/counter_main.dob" <<'DOBRA'
import './counter' show n

while n < 3 {
  emit n
  n = n + 1
}
DOBRA
actual="$($BIN run "$TMP_DIR/counter_main.dob")"
assert_eq "mutable imported let" "0
1
2" "$actual"

cat > "$TMP_DIR/a.dob" <<'DOBRA'
import './b' as b
const name = "A"
fn pair() {
  return "{name}/{b.name}"
}
DOBRA
cat > "$TMP_DIR/b.dob" <<'DOBRA'
import './a' as a
const name = "B"
fn pair() {
  return "{name}/{a.name}"
}
DOBRA
cat > "$TMP_DIR/circular.dob" <<'DOBRA'
import './a' as a
import './b' as b

emit a.pair()
emit b.pair()
DOBRA
actual="$($BIN run "$TMP_DIR/circular.dob")"
assert_eq "circular imports" "A/B
B/A" "$actual"

cat > "$TMP_DIR/stdlib.dob" <<'DOBRA'
const tags = split("dev,ops,docs", ",")
emit join(tags, "|")
emit contains(tags, "ops")
emit starts("dobra", "dob")
emit ends("dobra", "bra")
DOBRA
actual="$($BIN run "$TMP_DIR/stdlib.dob")"
assert_eq "stdlib text helpers" "dev|ops|docs
true
true
true" "$actual"

cat > "$TMP_DIR/math_list.dob" <<'DOBRA'
const nums = push([1, 2], 3)
emit nums
emit sum(nums)
emit avg(nums)
emit sqrt(9)
emit sort([3, 1, 2])
emit unique(["a", "b", "a"])
DOBRA
actual="$($BIN run "$TMP_DIR/math_list.dob")"
assert_eq "math and list helpers" "[1, 2, 3]
6
2.0
3.0
[1, 2, 3]
[a, b]" "$actual"


cat > "$TMP_DIR/io_input.txt" <<'TEXT'
one
two
TEXT
cat > "$TMP_DIR/io.dob" <<'DOBRA'
const src = open("io_input.txt", "read")
const out = open("io_output.txt", "write")

let line = readln(src)
while line != null {
  writeln(out, upper(line))
  line = readln(src)
}

close(src)
close(out)
emit read("io_output.txt")
DOBRA
actual="$(cd "$TMP_DIR" && "$OLDPWD/$BIN" run io.dob --allow-write)"
assert_eq "stream file io" "ONE
TWO" "$actual"
assert_eq "stream file content" "ONE
TWO" "$(sed '$ s/$//' "$TMP_DIR/io_output.txt")"

cat > "$TMP_DIR/io_denied.dob" <<'DOBRA'
write("blocked.txt", "nope")
DOBRA
if (cd "$TMP_DIR" && "$OLDPWD/$BIN" run io_denied.dob > denied.out 2> denied.err); then
    printf 'FAIL io write permission\nExpected command to fail\n' >&2
    exit 1
fi
grep 'error\[E3001\]' "$TMP_DIR/denied.err" >/dev/null
printf 'ok io write permission\n'

cat > "$TMP_DIR/raw.dob" <<'DOBRA'
const user={name:"Ana",role:"dev"}
if user.name!=""{emit "hello {user.name}"}
DOBRA
actual="$($BIN fmt --stdout "$TMP_DIR/raw.dob")"
expected='const user = {
  name: "Ana",
  role: "dev",
}

if user.name != "" {
  emit "hello {user.name}"
}'
assert_eq "formatter stdout" "$expected" "$actual"

$BIN fmt "$TMP_DIR/raw.dob" >/dev/null
$BIN fmt --check "$TMP_DIR/raw.dob" >/dev/null
$BIN check "$TMP_DIR/raw.dob" --json | grep '"ok":true' >/dev/null
$BIN tokens "$TMP_DIR/raw.dob" --json | grep '"tokens"' >/dev/null
$BIN ast "$TMP_DIR/raw.dob" --json | grep '"ast"' >/dev/null

actual="$($BIN eval 'emit join(["a", "b"], ":")')"
assert_eq "eval command" "a:b" "$actual"

$BIN version --json | grep '"name":"dobra"' >/dev/null
printf 'ok version json\n'

mkdir "$TMP_DIR/project"
$BIN init "$TMP_DIR/project" >/dev/null
actual="$(cd "$TMP_DIR/project" && "$OLDPWD/$BIN" run --var name=Project)"
assert_eq "project entry discovery" "Hello, Project" "$actual"
