# AST Schema

This page documents the public AST shape for Nodia v0.7. The AST is not yet a
stable serialized format, but its shape is part of the language baseline.

`nodia ast file.nod` prints the parsed AST as Rust `Debug` text. `--json`
wraps that text in a JSON envelope.

## Program

```text
Program {
  statements: Vec<Stmt>
}
```

## Statements

### Comment

```text
Comment(text: String)
```

### Use

```text
Use {
  path: String,
  alias: Option<String>,
  pick: Vec<String>,
  hide: Vec<String>,
}
```

### Bind

```text
Bind {
  name: String,
  value: Expr,
  mutable: Bool,
}
```

`var` sets `mutable: true`; `val` sets `mutable: false`.

### Assign

```text
Assign {
  name: String,
  value: Expr,
}
```

### Func

```text
Func {
  name: String,
  params: Vec<String>,
  body: Vec<Stmt>,
}
```

### Return

```text
Return(Option<Expr>)
```

### Emit

```text
Emit(Expr)
```

### If

```text
If {
  condition: Expr,
  then_branch: Vec<Stmt>,
  else_branch: Vec<Stmt>,
}
```

`else if` is represented as an `else_branch` containing one nested `If`.

### For

```text
For {
  name: String,
  iterable: Expr,
  body: Vec<Stmt>,
}
```

### While

```text
While {
  condition: Expr,
  body: Vec<Stmt>,
}
```

### Break / Continue

```text
Break
Continue
```

### Expression Statement

```text
Expr(Expr)
```

## Expressions

### Literal

```text
Literal(Value)
```

For `null`, `true`/`false`, integers, floats, and strings. List and map
literals use their own nodes (below).

### Identifier

```text
Identifier(String)
```

### Unary

```text
Unary {
  op: UnaryOp,
  expr: Box<Expr>,
}
```

### Binary

```text
Binary {
  op: BinaryOp,
  left: Box<Expr>,
  right: Box<Expr>,
}
```

### Call / Get / Index

```text
Call { callee: Box<Expr>, args: Vec<Expr> }
Get  { object: Box<Expr>, field: String }
Index{ collection: Box<Expr>, index: Box<Expr> }
```

### List / Map

```text
List(Vec<Expr>)
Map(Vec<(MapKey, Expr)>)
```

Map keys are either identifiers or strings, captured as `MapKey`.

### Regex

```text
Regex {
  flags: Vec<RegexFlag>,
  nodes: Vec<RegexNode>,
}
```

The `RegexNode` enum mirrors the DSL: anchors, classes, quantifiers, groups,
character sets, lookarounds, alternations (`Either { branches: Vec<Vec<RegexNode>> }`),
backreferences, scoped flag regions, and literal helpers. See
[Regex DSL](../language/regex.md) for the full surface.
