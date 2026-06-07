# Formatter

Nodia ships with a canonical, **non-configurable** formatter accessible via
[`nodia fmt`](../cli/fmt.md). The formatter is part of the language contract:
new syntax is considered incomplete until it has canonical formatting.

## Rules

| Rule           | Style                                                    |
| -------------- | -------------------------------------------------------- |
| Indent         | 2 spaces                                                 |
| Braces         | opening brace on the same line                           |
| Operators      | spaces around binary operators                           |
| Blocks         | always use `{}`                                          |
| Maps           | non-empty maps are multi-line                            |
| Lists / calls  | inline when they fit; otherwise multi-line with trailing commas |
| Line width     | formatter-controlled lines target 60 characters          |
| Final newline  | always emitted                                           |
| Comments       | preserved as comment statements                          |
| Long strings   | may be split with `+`; interpolated strings split only at literal boundaries |

## Before / After

Input:

```nodia
val user={name:"Ana",role:"dev"}
if user.name!=""{emit "hello {user.name}"}
```

Output:

```nodia
val user = {
  name: "Ana",
  role: "dev",
}

if user.name != "" {
  emit "hello {user.name}"
}
```

## CI

Use `--check` to fail without modifying files:

```bash
./target/release/nodia fmt --check .
```

This exits non-zero when any file would change. Combine with the rest of
your CI step (`nodia check src/main.nod`, tests, etc.).

## Skipped Paths

When given a directory, `nodia fmt` recursively visits `.nod` files and skips
the `target/` directory.
