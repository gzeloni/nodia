# Conditionals

`if` is the only branching construct in Nodia. It evaluates the condition for
truthiness (see [Truthiness](values.md#truthiness)).

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

## No Ternary

Nodia v0.6 has no ternary or conditional expression. Use a regular `if` or
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
val items = ["a", "b"]
if len(items) > 0 {
  emit "first={items[0]}"
}
'
```

```text
first=a
```
