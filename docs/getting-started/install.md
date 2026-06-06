# Install & Build

Nodia is implemented in Rust. The crate name is `nodia` and the binary it
produces is `nodia`.

## Requirements

* Rust toolchain (1.75+ recommended).
* No external runtime dependencies.

The crate depends on the Rust standard library plus
[`fancy-regex`](https://crates.io/crates/fancy-regex) for the regex backend.

## Clone

```bash
git clone https://github.com/gzeloni/orich
cd orich
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
nodia 0.7.0
```

JSON metadata:

```bash
./target/release/nodia version --json
```

```json
{"name":"nodia","version":"0.7.0","rust_std_only":true}
```

`rust_std_only: true` reflects the runtime's design constraint: Nodia's runtime
does not depend on third-party crates beyond `fancy-regex` (which is feature-
gated to regex compilation and execution only).

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

A VSCode extension is included in the repo under:

```text
vscode/nodia-language
```

Install it with **Developer: Install Extension from Location...** and pick the
`vscode/nodia-language` folder. It registers the `.nod` association, highlights
the language surface, and adds completions for stdlib namespaces, `use`
declarations, and the regex DSL.
