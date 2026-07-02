# Source Files & Comments

## Extension

Nodia source files use the `.nod` extension. There is no other accepted
extension and no fallback resolution to non-`.nod` paths.

```text
main.nod
lib/text.nod
lib/format.nod
showcase/index.nod
```

## Statement Separators

Statements are separated by newlines. Semicolons are accepted as explicit
statement separators but are not idiomatic — the formatter rewrites
single-line `a; b` into separate lines.

```nodia
val a = 1
val b = 2
emit a + b
```

Equivalent (but non-canonical):

```nodia
val a = 1; val b = 2; emit a + b
```

## Comments

Line comments use `#` or `//`. Both forms are accepted equally; the formatter
preserves whichever you wrote.

```nodia
# preferred for docs-like comments
// also accepted
emit "ok"
```

Block comments use `/*` and `*/` and can span multiple lines:

```nodia
/*
  This is a multi-line
  block comment.
*/
emit "ok"
```

Single-line block comments are also accepted:

```nodia
emit 1 /* inline */ + 2
```

Nested block comments are **not** supported. A `/*` inside a block comment
is treated as plain text until the first `*/` closes the comment.

Multi-line documentation is still commonly written as a stack of
single-line `#` or `//` comments — the formatter preserves whichever style
you choose.

The lexer preserves comments as tokens, and the parser represents them as
statements, so the formatter never loses or reorders them.

## Identifiers

Identifiers start with `_` or any Unicode letter, and may continue with
Unicode letters, Unicode digits, or `_`:

```text
[_\p{L}][_\p{L}\p{N}]*
```

Examples:

```nodia
val name = "Ana"
val user_id = 42
val über = 7
emit name
emit user_id
emit über
```

Keywords remain reserved ASCII words and cannot be reused as identifiers.

## Reserved Words

Currently reserved (and used by the language):

```text
val var func return
try catch throw
match case default
if else for in while break continue
emit use as pick hide
true false null
and or not
regex
type enum struct namespace
```

Reserved for future versions (rejected as identifiers):

```text
from
defer
```

Legacy keywords from earlier prototypes (`let`, `const`, `fn`, `import`,
`show`) are also rejected — there is no compatibility mode for the old
surface syntax.

## Related Pages

* [Conditionals & Match](conditionals.md)
* [Errors](errors.md)
* [Modules](modules.md)
* [Namespaces, Structs, Enums, Types](structs.md)
* [Operators](operators.md)
