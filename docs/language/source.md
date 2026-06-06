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

Block comments are **not** part of Nodia 0.7. Multi-line documentation is
written as a stack of single-line comments.

The lexer preserves comments as tokens, and the parser represents them as
statements, so the formatter never loses or reorders them.

## Identifiers

Identifiers begin with an ASCII letter or `_`, followed by ASCII letters,
digits, or `_`:

```text
[A-Za-z_][A-Za-z0-9_]*
```

Non-ASCII identifiers are not part of the 0.7 baseline.

## Reserved Words

Currently reserved (and used by the language):

```text
val var func return
if else for in while break continue
emit use as pick hide
true false null
and or not
regex
```

Reserved for future versions (rejected as identifiers):

```text
from match case default
try catch throw defer
type enum struct namespace
```

Legacy keywords from earlier prototypes (`let`, `const`, `fn`, `import`,
`show`) are also rejected — there is no compatibility mode for the old
surface syntax.
