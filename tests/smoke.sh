#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
TMPDIR_ROOT="${TMPDIR:-/tmp}"
TMPDIR_PATH="$(mktemp -d "$TMPDIR_ROOT/nodia-smoke.XXXXXX")"
trap 'rm -rf "$TMPDIR_PATH"' EXIT HUP INT TERM

cd "$ROOT"

cargo build >/dev/null

cat > "$TMPDIR_PATH/main.nod" <<'EOF'
use text
use numbers
use collections as col
use conversion as conv
use format as fmt
use re
use io
use system
use datetime as dt
use json
use csv

val decode = json.read
val encode = json.write
val rows = csv.read("name,age\nAna,30", {
  header: true,
  types: true,
})

emit text.upper("ana")
emit text.nfc("é")
emit text.casefold("Straße")
emit text.grapheme_len("éx")
emit text.grapheme("éx", 0)
emit text.byte_slice("aéb", 1, 3)
emit numbers.abs(-4)
emit conv.string(3)
emit fmt.fixed(3.14, 1)
emit re.find("ana 42", regex { one_or_more digit }).text
emit io.basename("/tmp/report.txt")
emit system.args[1]
emit dt.year(dt.date(2026, 6, 3))
emit col.map(numbers.int, ["1", "2"])
emit decode(r'{"ok":true,"name":"Ana"}').name
emit rows[0].age + 1
emit encode(rows[0], 2)
emit csv.write(rows)
EOF

cat > "$TMPDIR_PATH/expected.txt" <<'EOF'
ANA
é
strasse
2
é
é
4
3
3.1
42
report.txt
one
2026
[1, 2]
Ana
31
{
  "age": 30,
  "name": "Ana"
}
age,name
30,Ana
EOF

./target/debug/nodia run "$TMPDIR_PATH/main.nod" -- zero one > "$TMPDIR_PATH/actual.txt"

if ! cmp -s "$TMPDIR_PATH/expected.txt" "$TMPDIR_PATH/actual.txt"; then
  echo "smoke test failed" >&2
  diff -u "$TMPDIR_PATH/expected.txt" "$TMPDIR_PATH/actual.txt" >&2 || true
  exit 1
fi
