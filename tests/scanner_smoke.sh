#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
TMPDIR_ROOT="${TMPDIR:-/tmp}"
TMPDIR_PATH="$(mktemp -d "$TMPDIR_ROOT/nodia-scan.XXXXXX")"
trap 'rm -rf "$TMPDIR_PATH"' EXIT HUP INT TERM

cd "$ROOT"

cargo build --release >/dev/null

cat > "$TMPDIR_PATH/config.txt" <<'EOF'
# app
host=localhost
port=8080

mode=dev
EOF

run_case() {
  source="$1"
  expected="$2"
  actual="$("./target/release/nodia" -e "$source")"
  if [ "$actual" != "$expected" ]; then
    echo "scanner smoke test failed" >&2
    echo "expected:" >&2
    printf '%s\n' "$expected" >&2
    echo "actual:" >&2
    printf '%s\n' "$actual" >&2
    exit 1
  fi
}

run_case "$(cat <<'EOF'
use scan

func parse_log(line) {
  val s = scan.cursor(line)
  val stamp = scan.take_until(s, " ")
  scan.expect(s, " ", "space after timestamp")
  val level = scan.take_until(s, " ")
  scan.expect(s, " ", "space after level")
  val start = scan.pos(s)
  while not scan.at_end(s) {
    scan.advance(s)
  }
  val body = scan.span(s, start)
  return {
    stamp: stamp.text,
    level: level.text,
    body: body.text,
  }
}

val parsed = parse_log("2026-06-29 INFO boot complete")
emit parsed.stamp
emit parsed.level
emit parsed.body
EOF
)" "2026-06-29
INFO
boot complete"

run_case "$(cat <<EOF
use io
use scan

func parse_config(path) {
  val src = io.open("$TMPDIR_PATH/config.txt", "read")
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
    val start = scan.pos(s)
    while not scan.at_end(s) {
      scan.advance(s)
    }
    val value = scan.span(s, start)
    out[key.text] = value.text
  }

  io.close(src)
  return out
}

val parsed = parse_config("$TMPDIR_PATH/config.txt")
emit parsed.host
emit parsed.port
emit parsed.mode
EOF
)" "localhost
8080
dev"

run_case "$(cat <<'EOF'
use scan

func parse_block(text) {
  val s = scan.cursor(text)
  scan.expect(s, "<<<", "opening marker")
  scan.expect(s, "\n", "newline after opening marker")
  val body = scan.take_until(s, ">>>")
  scan.expect(s, ">>>", "closing marker")
  emit body.text
  emit scan.lookahead(s, 4)
}

parse_block("<<<
alpha
beta
>>>tail")
EOF
)" "alpha
beta

tail"

run_case "$(cat <<'EOF'
use scan

val s = scan.cursor("key=value")
val key = scan.take_until(s, "=")
emit scan.token("ident", key).kind
emit scan.token("ident", key).span.start.offset
emit scan.token("ident", key).text
EOF
)" "ident
0
key"

run_case "$(cat <<'EOF'
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
EOF
)" "E4300
[\"scan.expect\"]
1:5"
