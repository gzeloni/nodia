#!/bin/sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
TMPDIR_ROOT="${TMPDIR:-/tmp}"
TMPDIR_PATH="$(mktemp -d "$TMPDIR_ROOT/nodia-bench.XXXXXX")"
RUNS="${RUNS:-5}"
BINARY="$ROOT/target/release/nodia"
trap 'rm -rf "$TMPDIR_PATH"' EXIT HUP INT TERM

cargo build --release >/dev/null

generate_messy_fixture() {
  output="$1"
  : > "$output"
  i=0
  while [ "$i" -lt 2000 ]; do
    printf '  team-%s    task-%s   status  \n' "$i" "$((i % 9))" >> "$output"
    if [ $((i % 5)) -eq 0 ]; then
      printf '\n' >> "$output"
    fi
    if [ $((i % 11)) -eq 0 ]; then
      printf '# archived note %s\n' "$i" >> "$output"
    fi
    if [ $((i % 17)) -eq 0 ]; then
      printf '// generated note %s\n' "$i" >> "$output"
    fi
    i=$((i + 1))
  done
}

generate_url_fixture() {
  output="$1"
  : > "$output"
  i=0
  while [ "$i" -lt 3000 ]; do
    printf 'See https://svc-%s.example.dev/docs/%s and http://cdn-%s.example.dev/assets\n' \
      "$((i % 37))" "$i" "$((i % 23))" >> "$output"
    if [ $((i % 7)) -eq 0 ]; then
      printf 'noise line %s without urls\n' "$i" >> "$output"
    fi
    i=$((i + 1))
  done
}

generate_audit_fixture() {
  output="$1"
  : > "$output"
  i=0
  while [ "$i" -lt 3500 ]; do
    case $((i % 3)) in
      0) level="INFO" ;;
      1) level="WARN" ;;
      *) level="ERROR" ;;
    esac
    case $((i % 4)) in
      0) user="ana" ;;
      1) user="bia" ;;
      2) user="carla" ;;
      *) user="davi" ;;
    esac
    case $((i % 5)) in
      0) action="deploy" ;;
      1) action="retry" ;;
      2) action="sync" ;;
      3) action="notify" ;;
      *) action="archive" ;;
    esac
    printf '%s %s %s\n' "$level" "$user" "$action" >> "$output"
    if [ $((i % 9)) -eq 0 ]; then
      printf 'noise %s should be ignored\n' "$i" >> "$output"
    fi
    i=$((i + 1))
  done
}

run_case() {
  name="$1"
  script="$2"
  input="$3"
  times_file="$TMPDIR_PATH/$name.times"
  time_file="$TMPDIR_PATH/$name.time"
  output_file="$TMPDIR_PATH/$name.out"

  : > "$times_file"
  i=1
  while [ "$i" -le "$RUNS" ]; do
    /usr/bin/time -p "$BINARY" run "$ROOT/$script" --var "path=$input" > "$output_file" 2> "$time_file"
    awk '$1 == "real" { print $2 }' "$time_file" >> "$times_file"
    i=$((i + 1))
  done

  input_bytes="$(wc -c < "$input" | tr -d '[:space:]')"
  output_bytes="$(wc -c < "$output_file" | tr -d '[:space:]')"
  best_real="$(awk 'NR == 1 || $1 < min { min = $1 } END { printf "%.3f", min }' "$times_file")"
  avg_real="$(awk '{ sum += $1 } END { printf "%.3f", sum / NR }' "$times_file")"
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$name" "$input_bytes" "$output_bytes" "$RUNS" "$best_real" "$avg_real"
}

MESSY_FIXTURE="$TMPDIR_PATH/messy.txt"
URL_FIXTURE="$TMPDIR_PATH/urls.txt"
AUDIT_FIXTURE="$TMPDIR_PATH/audit.log"

generate_messy_fixture "$MESSY_FIXTURE"
generate_url_fixture "$URL_FIXTURE"
generate_audit_fixture "$AUDIT_FIXTURE"

printf '# %s\n' "$("$BINARY" version)"
printf '# runs=%s\n' "$RUNS"
printf 'workflow\tinput_bytes\toutput_bytes\truns\tbest_real_s\tavg_real_s\n'
run_case "normalize-messy-text" "bench/workflows/normalize_messy_text.nod" "$MESSY_FIXTURE"
run_case "extract-urls" "bench/workflows/extract_urls.nod" "$URL_FIXTURE"
run_case "summarize-audit-log" "bench/workflows/summarize_audit_log.nod" "$AUDIT_FIXTURE"
