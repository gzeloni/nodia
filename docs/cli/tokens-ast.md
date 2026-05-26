# Tokens & AST

Both commands are tooling/debug commands intended for editor integrations,
parser debugging, and language tests.

## `nodia tokens`

Print the lexer token stream for a file.

```bash
nodia tokens file.nod
nodia tokens file.nod --json
```

Example file:

```nodia
val name = "Ana"
```

```bash
./target/release/nodia tokens file.nod
```

Output shape:

```text
1:1 Val
1:5 Identifier("name")
1:10 Equal
1:12 String("Ana")
```

JSON form:

```bash
./target/release/nodia tokens file.nod --json
```

```json
{"ok":true,"tokens":[{"kind":"Val","literal":null,"line":1,"column":1}]}
```

## `nodia ast`

Print the parsed AST for a file.

```bash
nodia ast file.nod
nodia ast file.nod --json
```

The default output is the Rust `Debug` representation of the AST. The JSON
form wraps that debug string:

```json
{"ok":true,"ast":"Program { ... }"}
```

For the schema of each AST node, see [AST Schema](../reference/ast.md).

## When To Use

| Command       | Use case                                              |
| ------------- | ----------------------------------------------------- |
| `nodia tokens` | Lexer-level inspection, editor tooling, lexer tests.  |
| `nodia ast`    | Parser-level inspection, formatter and IR work.       |

Neither command executes the program. Both run only the lexer (and parser for
`ast`); no semantic check, no use resolution, no IO.
