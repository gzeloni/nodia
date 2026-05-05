# Dobra AST Schema v0.4

This document defines the public AST schema for the Dobra v0.4 baseline. It is a
human-readable schema derived from the implementation. The AST is not yet a
stable serialized format, but its shape is part of the v0.4 language baseline.

## Program

```text
Program {
  statements: Vec<Stmt>
}
```

A program is an ordered list of statements.

## Statements

### Comment

```text
Comment(text: String)
```

Represents a source line comment without the comment prefix.

### Import

```text
Import {
  path: String,
  alias: Option<String>,
  show: Vec<String>,
  hide: Vec<String>
}
```

Represents one import declaration.

```dob
import "./lib" as lib show title hide internal
```

### Let

```text
Let {
  name: String,
  value: Expr,
  mutable: Bool
}
```

Represents both `let` and `const`.

| Source | AST field |
|---|---|
| `let` | `mutable: true` |
| `const` | `mutable: false` |

### Assign

```text
Assign {
  name: String,
  value: Expr
}
```

Represents assignment to an existing binding.

### Fn

```text
Fn {
  name: String,
  params: Vec<String>,
  body: Vec<Stmt>
}
```

Represents a function declaration.

### Return

```text
Return(Option<Expr>)
```

`return` without an expression stores `None`.

### Emit

```text
Emit(Expr)
```

Represents output through the Dobra output channel.

### If

```text
If {
  condition: Expr,
  then_branch: Vec<Stmt>,
  else_branch: Vec<Stmt>
}
```

`else if` is represented as an `else_branch` containing one nested `If`.

### For

```text
For {
  name: String,
  iterable: Expr,
  body: Vec<Stmt>
}
```

Represents iteration over a runtime iterable.

### While

```text
While {
  condition: Expr,
  body: Vec<Stmt>
}
```

Represents a condition-controlled loop.

### Break

```text
Break
```

Represents loop exit.

### Continue

```text
Continue
```

Represents loop continuation.

### Expr

```text
Expr(Expr)
```

Represents an expression statement whose value is ignored.

## Expressions

### Literal

```text
Literal(Value)
```

Represents literal runtime values.

Supported literal values in source:

- `null`;
- `true` / `false`;
- integer;
- float;
- string.

List and map source forms use dedicated expression nodes.

### Identifier

```text
Identifier(String)
```

Represents a binding lookup.

### Unary

```text
Unary {
  op: UnaryOp,
  expr: Box<Expr>
}
```

Unary operators:

| Source | AST |
|---|---|
| `-` | `Negate` |
| `not` | `Not` |

### Binary

```text
Binary {
  left: Box<Expr>,
  op: BinaryOp,
  right: Box<Expr>
}
```

Binary operators:

| Source | AST |
|---|---|
| `+` | `Add` |
| `-` | `Subtract` |
| `*` | `Multiply` |
| `/` | `Divide` |
| `%` | `Modulo` |
| `==` | `Equal` |
| `!=` | `NotEqual` |
| `<` | `Less` |
| `<=` | `LessEqual` |
| `>` | `Greater` |
| `>=` | `GreaterEqual` |
| `and` | `And` |
| `or` | `Or` |

### Call

```text
Call {
  callee: Box<Expr>,
  args: Vec<Expr>
}
```

Represents a function or callable value invocation.

### Get

```text
Get {
  object: Box<Expr>,
  field: String
}
```

Represents dotted field access.

### Index

```text
Index {
  object: Box<Expr>,
  index: Box<Expr>
}
```

Represents bracket indexing.

### List

```text
List(Vec<Expr>)
```

Represents a list literal.

### Map

```text
Map(Vec<(String, Expr)>)
```

Represents a string-keyed map literal. Source keys may be identifiers or strings;
the AST stores both as strings.

## Schema Evolution Rules

Future versions may replace this AST with a typed AST or CST plus lowering
pipeline. Until then, changes to this schema must be treated as language changes
and reflected in:

- [specification.md](specification.md);
- formatter behavior;
- parser tests;
- corpus examples;
- CLI `ast` output expectations.
