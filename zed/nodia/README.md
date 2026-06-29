# Nodia for Zed

Local Zed support for `.nod` files.

## What It Includes

- `.nod` language association
- Tree-sitter grammar for the current Nodia surface
- Syntax highlighting for the language and the regex DSL
- Bracket matching
- Block indentation rules
- Outline entries for functions and top-level bindings

## Install As A Dev Extension

Before installing the dev extension, bootstrap the local Tree-sitter grammar
repository once:

```bash
./zed/bootstrap-dev-grammar.sh
```

1. Open Zed.
2. Run `zed: install dev extension`.
3. Select this folder:

```text
zed/nodia
```

The grammar is loaded from the local repository at:

```text
zed/tree-sitter-nodia
```

## Publishing

The bootstrap script rewrites `extension.toml` to point at the local grammar
directory with an absolute path and keeps the grammar available at `HEAD`.

To publish it for broader use, replace that grammar entry with a public Git
repository that contains the Tree-sitter grammar and pin it by commit SHA.

## Current Scope

This extension provides language metadata, Tree-sitter parsing, highlighting,
brackets, indentation, and outline support.

It does not yet provide checker diagnostics or `nodia fmt` integration inside
Zed. Those need either:

- a Nodia language server, or
- a dedicated Rust/WASM Zed extension layer that shells out to the Nodia CLI.
