# Functions

Functions are declared with `func`. They have positional parameters and a
single `return` expression form.

## Declaration

```nodia
func add(left, right) {
  return left + right
}

emit add(2, 3)
```

```bash
./target/release/nodia eval '
func add(left, right) {
  return left + right
}
emit add(2, 3)
'
```

```text
5
```

## Parameters

* Positional only.
* Arity is checked at runtime (`E4107` from the checker for known builtins
  and user functions).
* Parameters behave like `var` bindings inside the function body — you can
  reassign them.

```bash
./target/release/nodia eval '
func clip(n) {
  if n < 0 { n = 0 }
  return n
}
emit clip(-3)
emit clip(7)
'
```

```text
0
7
```

## Return

* `return expr` returns the expression value.
* `return` without an expression returns `null`.
* Falling off the end of the body returns `null`.

```bash
./target/release/nodia eval '
func noop() {}
func stop() { return }
emit noop()
emit stop()
'
```

```text
null
null
```

`return` outside a function is rejected by the checker (`E4103`).

## Recursion

```bash
./target/release/nodia eval '
func fact(n) {
  if n <= 1 { return 1 }
  return n * fact(n - 1)
}
emit fact(6)
'
```

```text
720
```

There is no explicit tail-call optimization in v0.6, but the recursion depth
required for typical text-automation work is well within the call stack.

## Scope

A function body sees its own parameters, its local bindings, and bindings from
enclosing lexical scopes. Nested `func` declarations capture the bindings
visible at the point where they are defined, including outer parameters and
locals.

```bash
./target/release/nodia eval '
func make_greeter(prefix) {
  func greet(name) {
    return "{prefix}, {name}"
  }
  return greet
}

val greet = make_greeter("Hello")
emit greet("Ana")
'
```

```text
Hello, Ana
```

Module-level bindings still work the same way:

```bash
./target/release/nodia eval '
val greeting = "Hello"

func greet(name) {
  return "{greeting}, {name}"
}

emit greet("Ana")
'
```

```text
Hello, Ana
```

Captured bindings preserve mutability:

* captured `var` stays shared and mutable across calls
* captured `val` stays read-only

```bash
./target/release/nodia eval '
func counter() {
  var n = 0
  func tick() {
    n = n + 1
    return n
  }
  return tick
}

val t = counter()
emit t()
emit t()
emit t()
'
```

```text
1
2
3
```

If you need isolated state, create a fresh closure. If you need explicit shared
state across unrelated functions, pass the state as a parameter or keep it in a
`var` binding in the scope those functions close over.

## Functions As Values

Functions are first-class values: you can pass them as arguments, return them
from other functions, and store them in lists or maps. There is no method
syntax — call style is always `f(x, y)`, never `x.f(y)`. This is the same
convention used for stdlib builtins (see [Standard Library](../stdlib/index.md)).
