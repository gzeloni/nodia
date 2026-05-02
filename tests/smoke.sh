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
actual="$("$BIN" run "$TMP_DIR/input.och" --vars name=World)"
assert_eq "input interpolation" "Hello, World" "$actual"

cat > "$TMP_DIR/core.och" <<'ORICH'
let age = int(input.age)
const items = ["a", "b", "c"]
emit "Age next year: {age + 1}"
emit "Second: {items[1]}"
ORICH
actual="$("$BIN" run "$TMP_DIR/core.och" --vars age=30)"
assert_eq "variables arithmetic index" "Age next year: 31
Second: b" "$actual"

cat > "$TMP_DIR/flow.och" <<'ORICH'
for item in ["ana", "john"] {
  if item == "ana" {
    emit uppercase(item)
  } else {
    emit capitalize(item)
  }
}
ORICH
actual="$("$BIN" run "$TMP_DIR/flow.och")"
assert_eq "for if builtins" "ANA
John" "$actual"

cat > "$TMP_DIR/fn.och" <<'ORICH'
fn greet(name) {
  return "Hi, {name}"
}

emit greet("Gustavo")
ORICH
actual="$("$BIN" run "$TMP_DIR/fn.och")"
assert_eq "function return" "Hi, Gustavo" "$actual"
