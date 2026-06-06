# Nodia

**Nodia** is a small, focused language for **textual automation**, **structured
output**, and **data/math workflows**. It is intentionally not a systems
language, an application platform, or a general-purpose scripting language.

The current release is **Nodia 0.7.0**, implemented in Rust on top of the
standard library plus
[`fancy-regex`](https://crates.io/crates/fancy-regex) for the regex engine.

```nodia
use text

val tags = ["compiler", "formatter", "streams"]

for tag in tags {
  emit "- {text.capitalize(tag)}"
}
```

```text
- Compiler
- Formatter
- Streams
```

## Why Nodia?

* **Readable**: keywords are words (`val`, `var`, `func`, `and`, `or`, `not`),
  not punctuation.
* **Canonical**: a single non-configurable formatter owns layout
  ([`nodia fmt`](cli/fmt.md)).
* **Predictable**: short, technical stdlib namespaces and call shapes
  (`text.upper`, `text.trim`, `text.split`, `text.lines`,
  `collections.len`, `numbers.range`...) and a small runtime.
* **Native regex DSL**: regexes are first-class values written as readable
  blocks, not opaque strings ([Regex DSL](language/regex.md)).
* **Safe IO by default**: file writes require explicit `--allow-write`.

## At a Glance

| Topic           | Where                                                  |
| --------------- | ------------------------------------------------------ |
| Install & build | [Getting Started](getting-started/install.md)          |
| CLI commands    | [Command Line](cli/index.md)                           |
| Language tour   | [Language](language/source.md)                         |
| Regex blocks    | [Regex DSL](language/regex.md)                         |
| Builtins        | [Standard Library](stdlib/index.md)                    |
| Grammar         | [Grammar](reference/grammar.md)                        |
| Worked examples | [Cookbook](examples/cookbook.md)                       |

## Hello, Nodia

```bash
./target/release/nodia eval 'emit "hello, nodia"'
```

```text
hello, nodia
```

With a CLI variable:

```bash
./target/release/nodia eval 'emit "hello, {input.name}"' # via run --var
```

```bash
./target/release/nodia run hello.nod --var name=Ana
```

```nodia
# hello.nod
emit "hello, {input.name}"
```

```text
hello, Ana
```

## File Extension

All Nodia source files use the `.nod` extension. There is no other accepted
extension and no fallback resolution into non-`.nod` paths.

## Version

```bash
./target/release/nodia version
```

```text
nodia 0.7.0
```

## Status

Nodia 0.7.0 is the current text-semantics baseline: text values are UTF-8,
`len(string)`, `slice(string, ...)`, indexing, and regex offsets are
scalar-based, and `byte_len`, `byte_offset`, and `scalar_offset` expose
explicit byte boundaries. Future versions will keep tightening the type and
effect model around this foundation.
