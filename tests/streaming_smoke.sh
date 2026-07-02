#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
TMPDIR_ROOT="${TMPDIR:-/tmp}"
TMPDIR_PATH="$(mktemp -d "$TMPDIR_ROOT/nodia-streaming.XXXXXX")"
trap 'rm -rf "$TMPDIR_PATH"' EXIT HUP INT TERM

cd "$ROOT"

cargo build --release >/dev/null

printf 'alpha\nbeta\ngamma\n' > "$TMPDIR_PATH/lines.txt"
printf 'abcdefghi' > "$TMPDIR_PATH/chunks.txt"
printf '\000\001\002\003\004' > "$TMPDIR_PATH/bytes.bin"
awk 'BEGIN { for (i = 0; i < 50000; i++) print "row-" i }' \
  > "$TMPDIR_PATH/big.txt"

run_case() {
  source="$1"
  expected="$2"
  actual="$("./target/release/nodia" -e "$source")"
  if [ "$actual" != "$expected" ]; then
    echo "streaming smoke test failed" >&2
    echo "expected:" >&2
    printf '%s\n' "$expected" >&2
    echo "actual:" >&2
    printf '%s\n' "$actual" >&2
    exit 1
  fi
}

run_case "$(cat <<EOF
use io
val src = io.open("$TMPDIR_PATH/lines.txt", "read")
for line in io.lines(src) {
  emit line
}
io.close(src)
EOF
)" "alpha
beta
gamma"

run_case "$(cat <<EOF
use io
use collections
use text
func loud(chunk) {
  return text.upper(chunk)
}
val src = io.open("$TMPDIR_PATH/chunks.txt", "read")
emit collections.collect(
  collections.map(loud, io.chunks(src, 3)),
)
io.close(src)
EOF
)" "[\"ABC\", \"DEF\", \"GHI\"]"

run_case "$(cat <<EOF
use io
use collections
use text
func keep(line) {
  return text.contains(line, "a")
}
val src = io.open("$TMPDIR_PATH/lines.txt", "read")
for line in collections.filter(keep, io.lines(src)) {
  emit line
}
io.close(src)
EOF
)" "alpha
beta
gamma"

run_case "$(cat <<EOF
use io
use collections
func pair(line) {
  return [line, line + "!"]
}
val src = io.open("$TMPDIR_PATH/lines.txt", "read")
for (left, right) in collections.map(pair, io.lines(src)) {
  emit "{left}|{right}"
}
io.close(src)
EOF
)" "alpha|alpha!
beta|beta!
gamma|gamma!"

run_case "$(cat <<EOF
use io
use collections
val src = io.open("$TMPDIR_PATH/bytes.bin", "read")
emit collections.collect(io.chunks(src, io.bytes, 2))
io.close(src)
EOF
)" "[b\"\0\x01\", b\"\x02\x03\", b\"\x04\"]"

run_case "$(cat <<EOF
use io
use collections
func add(acc, _) {
  return acc + 1
}
val src = io.open("$TMPDIR_PATH/big.txt", "read")
emit collections.reduce(add, 0, io.lines(src))
io.close(src)
EOF
)" "50000"

set +e
error_output="$("./target/release/nodia" -e "$(cat <<EOF
use io
val src = io.open("$TMPDIR_PATH/chunks.txt", "read")
val chunks = io.chunks(src, 0)
emit chunks
EOF
)" 2>&1)"
status=$?
set -e

if [ "$status" -eq 0 ] || [ "$error_output" != "error[E2000]: io.chunks() expects positive size as second argument" ]; then
  echo "streaming smoke test failed" >&2
  echo "expected error: error[E2000]: io.chunks() expects positive size as second argument" >&2
  echo "actual status: $status" >&2
  printf '%s\n' "$error_output" >&2
  exit 1
fi
