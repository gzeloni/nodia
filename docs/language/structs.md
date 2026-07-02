# Structs, Enums & Namespaces

Nodia provides three declaration forms for grouping data and names:
`namespace` for scoped name collections, `struct` for data shapes with
optional defaults, and `enum` for tagged variants. A `type` alias gives a
name to any value-kind shape.

## Namespaces

A `namespace` groups declarations into a named scope. At runtime, the
namespace is a map value whose keys are the declared names:

```nodia
namespace http {
  val timeout = 30
  func get(url) { return "GET " + url }
}
```

```bash
./target/release/nodia eval '
namespace http {
  val timeout = 30
  func get(url) { return "GET " + url }
}
emit http.timeout
emit http.get("/")
'
```

```text
30
GET /
```

Fields of a namespace are accessed with dot notation, like any map:

```bash
./target/release/nodia eval '
namespace http {
  val timeout = 30
}
emit http["timeout"]
'
```

```text
30
```

### Nested Namespaces

Namespaces can be nested to build deeper hierarchies:

```nodia
namespace app {
  namespace http {
    val timeout = 30
  }
  namespace db {
    val host = "localhost"
  }
}

emit app.http.timeout  # 30
emit app.db.host       # localhost
```

The formatter preserves the nesting structure. Each nested namespace is a
nested map at runtime.

### Namespaces As Values

Because a namespace is a map, you can pass it to functions, iterate over
its keys, and inspect it with stdlib helpers:

```bash
./target/release/nodia eval '
use collections
namespace cfg {
  val host = "127.0.0.1"
  val port = 8080
}
emit collections.keys(cfg)
emit collections.len(cfg)
'
```

```text
["host", "port"]
2
```

A namespace is **not** a module — it lives inline in the same file and is
not resolved through the `use` system. Use `namespace` when you want to
group related declarations inside a single file; use `use` when you want
to split code across files.

## Structs

A `struct` defines a data shape with named fields. Fields are separated by
newlines (no commas). Each field can optionally declare a default value:

```nodia
struct Point {
  x: 0
  y: 0
}

struct User {
  name
  age
}
```

At runtime, a struct is a map whose keys are the field names. Fields
without a default are `null`:

```bash
./target/release/nodia eval '
struct Point {
  x: 0
  y: 0
}
struct User {
  name
  age
}
emit Point.x
emit Point.y
emit User.name == null
emit User.age == null
'
```

```text
0
0
true
true
```

### Default Values

A field default is any expression. It is evaluated once when the struct is
defined:

```nodia
struct Config {
  timeout: 30
  host: "localhost"
  enabled: true
  tags: []
}
```

Fields with defaults behave like a `val` binding — the value is stored in
the struct map and accessed through dot or index notation.

### Structs As Values

Like namespaces, a struct is a map at runtime:

```bash
./target/release/nodia eval '
use collections
struct Point {
  x: 0
  y: 0
}
emit collections.keys(Point)
emit collections.values(Point)
'
```

```text
["x", "y"]
[0, 0]
```

### Struct Usage Notes

Structs are **not** constructors — there is no `new Point(...)` syntax.
The struct itself is the canonical instance with the declared defaults. To
create variations, copy from the struct with map helpers:

```nodia
use collections
val origin = collections.merge(Point, {x: 5})
emit origin.x  # 5
emit origin.y  # 0
```

Future versions may add constructor calls; for now, structs serve as named
shapes with defaults that can be spread into maps.

## Enums

An `enum` defines a set of tagged variants. Variants are comma-separated
and each variant is a map with a `kind` field:

```nodia
enum Status {
  active,
  inactive,
  pending,
}
```

```bash
./target/release/nodia eval '
enum Status {
  active,
  inactive,
  pending,
}
emit Status.active.kind
emit Status.inactive.kind
emit Status.pending.kind
'
```

```text
active
inactive
pending
```

Each variant is a map `{kind: "variant_name"}`. The `kind` field is
always a string matching the variant name.

### Using Enums

Enums work well with `match` for dispatching:

```bash
./target/release/nodia eval '
enum Status {
  active,
  inactive,
  pending,
}
var state = Status.active
match state.kind {
  "active"   { emit "running" }
  "inactive" { emit "stopped" }
  default    { emit "unknown" }
}
'
```

```text
running
```

The enum namespace is a map whose keys are the variant names:

```bash
./target/release/nodia eval '
use collections
enum Color { red, green, blue }
emit collections.keys(Color)
'
```

```text
["red", "green", "blue"]
```

### Enum Values

Each variant value is a plain map, so you can add extra fields when you
need more data:

```nodia
val state = Status.active
# state is {kind: "active"}
```

## Type Aliases

A `type` declaration creates a name for a type shape. It produces no
runtime value — it exists for the checker and for documentation:

```nodia
type Url = string
type Point = {x: float, y: float}
type Handler = func(string) string
```

The target expression on the right-hand side is **not** validated at the
point of declaration. This allows forward references:

```nodia
type Tree = {value: int, children: list<Tree>}
```

Type aliases are checked only when they are used — for example, when a
variable is declared with an explicit type annotation or when a function
parameter type is verified.

Without type aliases, the same shapes can still be expressed inline, but
`type` gives them a name you can use consistently across the program.
