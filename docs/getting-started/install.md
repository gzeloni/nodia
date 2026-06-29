# Install & Build

Nodia is implemented in Rust. The crate name is `nodia` and the binary it
produces is `nodia`.

## Requirements

* Rust toolchain (1.75+ recommended).
* No external runtime dependencies.

The crate depends on the Rust standard library plus
[`fancy-regex`](https://crates.io/crates/fancy-regex),
`unicode-normalization`, `unicode-segmentation`, and `caseless` for regex
execution and explicit Unicode text normalization/case-folding/segmentation.

## Clone

```bash
git clone https://github.com/gzeloni/nodia
cd nodia
```

## Debug Build

```bash
cargo build
```

The debug binary lives at:

```text
target/debug/nodia
```

## Release Build

```bash
cargo build --release
```

The release binary lives at:

```text
target/release/nodia
```

Throughout this site every example uses the release binary:

```bash
./target/release/nodia <command> [args]
```

## Verify

```bash
./target/release/nodia version
```

```text
nodia 0.8.3
```

JSON metadata:

```bash
./target/release/nodia version --json
```

```json
{"name":"nodia","version":"0.8.3","rust_std_only":false}
```

`rust_std_only: false` reflects the current runtime shape: Nodia now depends on
targeted third-party crates for regex execution and explicit Unicode
normalization/case-folding/segmentation.

## Quick Smoke Test

```bash
./target/release/nodia eval 'use text
emit text.upper("nodia")'
```

```text
NODIA
```

If you see `NODIA`, the build is working end to end: lexer, parser, checker,
runtime, and stdlib `text.upper`.

## Editor Support

Two local editor integrations are included in the repository:

```text
vscode/nodia-language
zed/nodia
```

### VS Code

Install it with **Developer: Install Extension from Location...** and pick the
`vscode/nodia-language` folder. It registers the `.nod` association, highlights
the language surface, adds completions for stdlib namespaces, `use`
declarations, and the regex DSL, and integrates `nodia fmt` plus `nodia check`.

### Zed

Before installing the Zed extension, bootstrap the local grammar repository:

```bash
./zed/bootstrap-dev-grammar.sh
```

Install it with `zed: install dev extension` and pick the `zed/nodia` folder.
It uses the local Tree-sitter grammar in `zed/tree-sitter-nodia` and provides
language association, syntax highlighting, bracket matching, indentation, and
outline support for `.nod` files.
