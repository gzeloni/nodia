# `nodia fmt`

Format `.nod` files using the canonical, non-configurable style. The formatter
is part of the language contract — new syntax is considered incomplete until it
has canonical formatting.

```bash
nodia fmt file.nod
nodia fmt .
nodia fmt --check .
nodia fmt --stdout file.nod
```

## Format A File In Place

```bash
./target/release/nodia fmt src/main.nod
```

Rewrites the file with canonical layout.

## Format A Directory

```bash
./target/release/nodia fmt .
```

Recursively visits `.nod` files and rewrites them. The `target/` directory is
always skipped.

## Check Without Writing

`--check` exits non-zero if any file would change:

```bash
./target/release/nodia fmt --check .
```

This is the form to use in CI.

## Print To Stdout

`--stdout` prints the formatted result without touching the file:

```bash
./target/release/nodia fmt --stdout src/main.nod
```

Useful for editor integrations that need the formatted text on stdout.

## Canonical Style

| Rule           | Style                                                |
| -------------- | ---------------------------------------------------- |
| Indent         | 2 spaces                                             |
| Braces         | opening brace on the same line                       |
| Operators      | spaces around binary operators                       |
| Blocks         | always use `{}`                                      |
| Maps           | non-empty maps are multi-line                        |
| Lists / calls  | inline when short, multi-line when long              |
| Line width     | formatter-controlled lines target 60 characters      |
| Final newline  | required                                             |
| Comments       | preserved as comment statements                      |
| Long strings   | may be split with `+`                                |

## Example

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

See [Formatter](../reference/formatter.md) for the full contract.
