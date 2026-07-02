# Conditionals & Match

Nodia has two branching tools:

* `if` / `else` for ordinary truthiness-based control flow
* `match` / `case` / `default` for structural branching

`if` evaluates the condition for truthiness (see
[Truthiness](values.md#truthiness)).

## `if` / `else`

```nodia
if count > 0 {
  emit "positive"
} else {
  emit "non-positive"
}
```

Braces are required around both branches. There is no single-expression form.

```bash
./target/release/nodia eval '
val count = 3
if count > 0 {
  emit "positive"
} else {
  emit "non-positive"
}
'
```

```text
positive
```

## `else if`

`else if` is expressed by nesting an `if` directly inside an `else` branch:

```nodia
if input.env == "prod" {
  emit "prod"
} else if input.env == "stage" {
  emit "stage"
} else {
  emit "dev"
}
```

```bash
./target/release/nodia eval '
val env = "stage"
if env == "prod" {
  emit "prod"
} else if env == "stage" {
  emit "stage"
} else {
  emit "dev"
}
'
```

```text
stage
```

The AST represents this as an `else` branch containing a nested `If` node —
there is no dedicated "else if" syntax tree (see [AST](../reference/ast.md)).

## `match` / `case` / `default`

`match` evaluates one value and checks the `case` arms in order. The first
matching arm runs. A `default` arm is required.

```nodia
match input.kind {
  case "ok" {
    emit "success"
  }
  case "warn" {
    emit "warning"
  }
  default {
    emit "unknown"
  }
}
```

### Capture Patterns

An identifier pattern captures the whole matched value:

```nodia
match input {
  case user {
    emit user.name
  }
  default {
    emit "missing"
  }
}
```

Use `_` when you want a wildcard without binding:

```nodia
match input.status {
  case "ready" { emit "ok" }
  case _ {
    emit "fallback"
  }
  default {
    emit "unreachable"
  }
}
```

### List And Map Patterns

Patterns can destructure fixed-length lists and maps with required keys:

```nodia
match payload {
  case ["user", name] {
    emit name
  }
  case {kind: "user", name} {
    emit name
  }
  default {
    emit "unsupported"
  }
}
```

Map shorthand like `{name}` means "require the key `name` and bind its value to
the local name `name`".

## No Ternary

Nodia v0.7 has no ternary or conditional expression. Use a regular `if` or
return a value from a small function:

```nodia
func label(env) {
  if env == "prod" { return "P" }
  return "?"
}

emit label("prod")
```

## Idioms

### Default With `or`

`or` returns the first truthy operand, so it doubles as a default:

```bash
./target/release/nodia eval '
val name = "" or "anonymous"
emit name
'
```

```text
anonymous
```

### Guard Then Emit

```bash
./target/release/nodia eval '
use collections
val items = ["a", "b"]
if collections.len(items) > 0 {
  emit "first={items[0]}"
}
'
```

```text
first=a
```
