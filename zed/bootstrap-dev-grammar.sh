#!/bin/sh
set -eu

ROOT_DIR=$(
  CDPATH= cd -- "$(dirname "$0")/.." && pwd
)
GRAMMAR_DIR="$ROOT_DIR/zed/tree-sitter-nodia"
EXTENSION_MANIFEST="$ROOT_DIR/zed/nodia/extension.toml"

if [ ! -d "$GRAMMAR_DIR" ]; then
  echo "missing grammar directory: $GRAMMAR_DIR" >&2
  exit 1
fi

awk -v grammar_dir="$GRAMMAR_DIR" '
  /^\[grammars\.nodia\]$/ {
    in_grammar = 1
    print
    next
  }

  in_grammar && /^repository = "/ {
    print "repository = \"" grammar_dir "\""
    in_grammar = 0
    next
  }

  { print }
' "$EXTENSION_MANIFEST" > "$EXTENSION_MANIFEST.tmp"
mv "$EXTENSION_MANIFEST.tmp" "$EXTENSION_MANIFEST"

if [ ! -d "$GRAMMAR_DIR/.git" ]; then
  git -C "$GRAMMAR_DIR" init -b main >/dev/null
fi

git -C "$GRAMMAR_DIR" add grammar.js package.json src

if git -C "$GRAMMAR_DIR" diff --cached --quiet; then
  if git -C "$GRAMMAR_DIR" rev-parse HEAD >/dev/null 2>&1; then
    echo "ok grammar repo already initialized"
    exit 0
  fi
fi

git -C "$GRAMMAR_DIR" \
  -c user.name="Nodia Local Bootstrap" \
  -c user.email="noreply@nodia.local" \
  commit -m "Bootstrap local grammar repo" >/dev/null

if [ -d "$ROOT_DIR/zed/nodia/grammars/nodia/.git" ]; then
  git -C "$ROOT_DIR/zed/nodia/grammars/nodia" remote set-url origin "$GRAMMAR_DIR"
fi

echo "ok grammar repo bootstrapped"
