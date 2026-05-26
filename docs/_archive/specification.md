# Nodia Language Specification Baseline v0.5

This document defines the v0.5 baseline language contract for Nodia. Its purpose
is to freeze the implemented behavior before the language moves toward the
static, safe, self-hosted direction described in [roadmap.md](roadmap.md).

The v0.5 baseline is intentionally descriptive: it records the current identity syntax and behavior. Earlier `let`/`const`/`fn`/`import`/`show` syntax is not part of this language version.

## 1. Scope

Nodia v0.5 is the specification baseline for the existing dynamic implementation.
It defines:

- source files and lexical grammar;
- reserved words;
- statements and expressions;
- operator precedence;
- runtime values;
- binding and mutability rules;
- module/use behavior;
- function behavior;
- control flow behavior;
- IO behavior;
- diagnostics;
- formatter guarantees;
- the public AST shape.

Nodia v0.5 does not yet define the future static type system, effect checker,
ownership model, IR, bytecode, REPL, or certification profiles. Those are roadmap
requirements, not v0.5 language behavior.

## 2. Source Files

Nodia source files use the `.nod` extension.

A source file is a sequence of statements. Statements are separated by newlines
or semicolons. Blocks are delimited by braces.

```nod
val name = "Nodia"
emit name
```

## 3. Lexical Grammar

### 3.1 Whitespace

Spaces, tabs, and carriage returns are ignored outside strings. Newlines are
significant as statement separators.

### 3.2 Comments

Nodia supports line comments with `#` and `//`.

```nod
# comment
// comment
```

The lexer preserves comments as tokens. The parser represents comments as
statements so the formatter can preserve them.

### 3.3 Identifiers

Identifiers begin with an ASCII letter or `_`, followed by ASCII letters, digits,
or `_`.

```text
[A-Za-z_][A-Za-z0-9_]*
```

Non-ASCII identifiers are not part of the v0.5 baseline.

### 3.4 Integer Literals

Integer literals are base-10 signed 64-bit values. The minus sign is parsed as a
unary operator, not as part of the literal.

```nod
val count = 42
val offset = -3
```

### 3.5 Float Literals

Float literals are decimal literals containing digits on both sides of `.`.

```nod
val ratio = 0.5
val total = 10.0
```

A trailing-dot form such as `10.` is not part of v0.5.

### 3.6 String Literals

Nodia supports double-quoted strings, single-quoted strings, and triple-quoted
strings.

```nod
val a = "text"
val b = 'text'
val c = """
multiline text
"""
```

Escapes in single-line strings:

| Escape | Meaning |
|---|---|
| `\n` | newline |
| `\r` | carriage return |
| `\t` | tab |
| `\"` | double quote |
| `\'` | single quote |
| `\\` | backslash |

Unknown escapes resolve to the escaped character itself in v0.5.

Triple-quoted strings preserve their contents until the next `"""` delimiter.

### 3.7 String Interpolation

At runtime, string values are interpolated when evaluated. Interpolation uses
braced expressions supported by the current runtime evaluator.

```nod
val name = "Nodia"
emit "hello {name}"
```

Interpolation is runtime behavior in v0.5, not a separate AST node.

## 4. Keywords

Implemented keywords:

```text
var val func return emit
if else for in while break continue
true false null
and or not
use as pick hide
```

Removed legacy keywords:

```text
let const fn import show
```

These keywords are intentionally rejected in v0.5. There is no compatibility
mode for the old surface syntax.

Reserved for future versions:

```text
from match case default
try catch throw defer
type enum struct namespace
```

A reserved future keyword cannot be used as an identifier and produces a parse
error when used as a statement keyword.

## 5. Grammar

The following grammar is normative for the v0.5 baseline, using an EBNF-like
notation.

```text
program        = { statement terminator } EOF ;
terminator     = newline | semicolon | EOF ;

statement      = comment
               | use_stmt
               | var_stmt
               | val_stmt
               | func_stmt
               | return_stmt
               | emit_stmt
               | if_stmt
               | for_stmt
               | while_stmt
               | break_stmt
               | continue_stmt
               | assign_stmt
               | expr_stmt ;

use_stmt       = "use" string { use_clause } ;
use_clause     = "as" identifier
               | "pick" use_names
               | "hide" use_names ;
use_names      = identifier { "," identifier } [","] ;

var_stmt       = "var" identifier "=" expression ;
val_stmt       = "val" identifier "=" expression ;
assign_stmt    = identifier "=" expression ;

func_stmt      = "func" identifier "(" [params] ")" block ;
params         = identifier { "," identifier } [","] ;

return_stmt    = "return" [expression] ;
emit_stmt      = "emit" expression ;

if_stmt        = "if" expression block ["else" (if_stmt | block)] ;
for_stmt       = "for" identifier "in" expression block ;
while_stmt     = "while" expression block ;
break_stmt     = "break" ;
continue_stmt  = "continue" ;
expr_stmt      = expression ;

block          = "{" { statement terminator } "}" ;

expression     = or_expr ;
or_expr        = and_expr { "or" and_expr } ;
and_expr       = equality_expr { "and" equality_expr } ;
equality_expr  = compare_expr { ("==" | "!=") compare_expr } ;
compare_expr   = term_expr { ("<" | "<=" | ">" | ">=") term_expr } ;
term_expr      = factor_expr { ("+" | "-") factor_expr } ;
factor_expr    = unary_expr { ("*" | "/" | "%") unary_expr } ;
unary_expr     = ("-" | "not") unary_expr | call_expr ;
call_expr      = primary_expr { call_suffix | get_suffix | index_suffix } ;
call_suffix    = "(" [args] ")" ;
get_suffix     = "." identifier ;
index_suffix   = "[" expression "]" ;
args           = expression { "," expression } [","] ;

primary_expr   = literal
               | identifier
               | "(" expression ")"
               | list_literal
               | map_literal ;

list_literal   = "[" [args] "]" ;
map_literal    = "{" [map_pair { "," map_pair } [","]] "}" ;
map_pair       = (identifier | string) ":" expression ;
```

## 6. Operator Precedence

Operators are listed from lowest to highest precedence.

| Level | Operators | Associativity |
|---|---|---|
| 1 | `or` | left |
| 2 | `and` | left |
| 3 | `==`, `!=` | left |
| 4 | `<`, `<=`, `>`, `>=` | left |
| 5 | `+`, `-` | left |
| 6 | `*`, `/`, `%` | left |
| 7 | unary `-`, `not` | right |
| 8 | call, field access, index access | left |
| 9 | literals, identifiers, grouped expressions | n/a |

## 7. Runtime Values

Nodia v0.5 has the following runtime values:

| Value | Description |
|---|---|
| `null` | Absence value. |
| `bool` | `true` or `false`. |
| `int` | Signed 64-bit integer. |
| `float` | 64-bit floating point value. |
| `string` | Unicode scalar string as provided by the host implementation. |
| `list` | Ordered collection of values. |
| `map` | Ordered string-keyed map. |
| `stream` | Standard or file-backed stream handle. |
| `function` | User-defined function. |
| `use` | Lazy used binding reference. |

Truthiness in v0.5:

| Value | False when |
|---|---|
| `null` | always false |
| `bool` | value is `false` |
| `int` | value is `0` |
| `float` | value is `0.0` |
| `string` | empty |
| `list` | empty |
| `map` | empty |
| `stream` | never |
| `function` | never |
| `use` | never |

Future static versions should remove arbitrary truthiness in stricter profiles.

## 8. Bindings And Mutability

`val` defines an immutable binding. `var` defines a mutable binding.

```nod
val name = "Nodia"
var count = 0
count = count + 1
```

Assignment searches existing scopes and updates the first matching mutable
binding. Assigning to an immutable binding is a runtime error. Assigning to an
undefined binding is a runtime error.

Function parameters and loop variables are mutable in v0.5.

## 9. Scopes

Nodia v0.5 uses lexical block scopes at runtime.

- The root scope contains top-level bindings and built-in runtime bindings.
- Blocks create nested scopes.
- Functions create local scopes for parameters and body execution.
- Used module functions capture exported root bindings from their module.

Top-level module declarations are exported when a file is used.

## 10. Functions

Functions are declared with `func`.

```nod
func add(left, right) {
  return left + right
}
```

Rules:

- parameters are positional;
- arity is checked at runtime;
- `return` without an expression returns `null`;
- falling off the end of a function returns `null`;
- `return` outside a function is a runtime error;
- `break` or `continue` escaping a function is a runtime error.

## 11. Control Flow

### 11.1 If

`if` evaluates the condition using v0.5 truthiness.

```nod
if count > 0 {
  emit "positive"
} else {
  emit "zero"
}
```

`else if` is represented as an `else` branch containing a nested `if` statement.

### 11.2 For

`for` iterates over lists, strings, and maps.

| Iterable | Iteration value |
|---|---|
| list | each list value |
| string | one-character string values |
| map | string keys |

Other values produce a runtime error.

### 11.3 While

`while` repeats while the condition is truthy. v0.5 enforces a runtime safety cap
of 100000 iterations.

### 11.4 Break And Continue

`break` exits the nearest loop. `continue` starts the next iteration of the
nearest loop. Using either outside a loop is a runtime error.

## 12. Expressions

### 12.1 Addition

`+` adds numbers or concatenates strings. String concatenation is selected when
at least one operand is a string.

### 12.2 Numeric Operators

`-`, `*`, `/`, and `%` operate on numeric values. Numeric conversion behavior is
runtime-defined in v0.5 and should be tightened in future static versions.

### 12.3 Equality

Equality is structural for `null`, bools, numbers, strings, lists, maps,
functions, streams, and use bindings as currently represented by the runtime.

### 12.4 Field Access

Field access works on maps.

```nod
val user = { name: "Ana" }
emit user.name
```

Accessing a missing field or a field on a non-map value is a runtime error.

### 12.5 Index Access

Index access works on lists, strings, and maps.

```nod
emit items[0]
emit name[1]
emit user["name"]
```

Invalid index operations produce runtime errors.

## 13. Modules And Uses

`use` declarations use string paths.

```nod
use "./lib"
use "./lib" as lib
use "./lib" pick title, version
use "./lib" hide internal
```

Resolution rules:

- relative paths resolve from the file containing the `use` declaration;
- absolute paths are accepted by the runtime;
- paths with an extension are used directly;
- paths without an extension try `.nod`, then `index.nod`, then the raw path;
- resolved modules are cached by canonical path;
- circular uses are allowed structurally;
- reading a used binding before initialization is a runtime error.

Use selection:

- `as name` uses selected bindings into a namespace map;
- no alias uses selected bindings directly into the current scope;
- `pick` restricts selected names;
- `hide` removes selected names;
- requesting a missing `pick` name is a semantic/runtime error.

## 14. IO And Streams

Nodia v0.5 supports real file and standard stream IO through builtins.

Standard stream bindings:

```text
stdin
stdout
stderr
```

IO builtins:

| Builtin | Behavior |
|---|---|
| `open(path, mode)` | Opens a stream with `read`, `write`, or `append`. |
| `close(stream)` | Closes a stream. |
| `flush(stream)` | Flushes a stream. |
| `eof(stream)` | Returns whether a stream reached EOF. |
| `read(path)` | Reads an entire file path. |
| `read(stream)` | Reads an entire stream. |
| `read(stream, size)` | Reads a stream chunk. |
| `readln(stream)` | Reads one line or returns `null` at EOF. |
| `write(path, text)` | Writes text to a path. |
| `write(stream, text)` | Writes text to a stream. |
| `writeln(stream, text)` | Writes text plus newline to a stream. |
| `append(path, text)` | Appends text to a path. |

File writes require the runtime option exposed by the CLI as `--allow-write`.

In v0.5, IO is runtime-enforced. Future versions must make IO an explicit static
effect.

## 15. Diagnostics

Nodia errors have a stable shape:

```text
error[E1000]: message
  at file.nod:line:column
```

Error classes:

| Code | Class |
|---|---|
| `E1000` | lexical or parse error |
| `E2000` | runtime language error |
| `E3000` | IO error |
| `E4000` | generic semantic check error |
| `E4100` | undefined variable |
| `E4101` | assignment to immutable binding |
| `E4102` | duplicate binding or parameter |
| `E4103` | invalid control-flow placement |
| `E4104` | invalid use selection |
| `E4105` | missing known field |
| `E4106` | invalid interpolation |
| `E4107` | invalid arity |

Errors may be rendered as JSON through CLI flags where supported.

Future versions must split parse, semantic, type, effect, runtime, and internal
compiler errors into more precise code ranges.

## 15.1 Semantic Checks

Nodia v0.5 performs semantic checks before `check` succeeds and before `run`
executes a program. These checks are intentionally limited, but they make the
language stricter than the v0.3 parse-only model.

The v0.5 checker rejects:

- undefined variables;
- assignment to immutable `val` bindings;
- duplicate bindings in the same scope;
- `return` outside a function;
- `break` or `continue` outside a loop;
- invalid arity for known user functions and builtins;
- missing fields on known used namespaces and literal maps;
- uses that select missing names through `pick`;
- invalid expressions inside string interpolation.

The checker is semantic, not yet static-typed. It tracks declarations, uses, function arity,
known namespace fields, and literal map shapes. It does not prove numeric types,
collection element types, effect safety, ownership, or allocation behavior.

## 16. Formatter Contract

Formatting is canonical and non-configurable.

Current v0.5 formatter rules:

| Rule | Contract |
|---|---|
| Indent | 2 spaces. |
| Line width | Formatter-controlled lines target 60 characters. |
| Final newline | Always emitted. |
| Operators | Spaces around binary operators. |
| Blocks | Braced blocks. |
| Maps | Non-empty maps are multi-line. |
| Lists | Inline when they fit; otherwise multi-line with trailing commas. |
| Calls | Inline when they fit; otherwise multi-line with trailing commas. |
| Comments | Preserved as comment statements. |
| Strings | Long single-line strings may be split with `+`. |

The formatter is part of the language contract. New syntax is incomplete until it
has canonical formatting.

## 17. CLI Contract

The v0.5 baseline recognizes these commands:

| Command | Purpose |
|---|---|
| `nodia run` | Execute a `.nod` file or stdin source. |
| `nodia check` | Lex, parse, and run v0.5 semantic checks. |
| `nodia fmt` | Format `.nod` files. |
| `nodia eval` | Execute source passed as CLI text. |
| `nodia tokens` | Emit token stream. |
| `nodia ast` | Emit parsed AST. |
| `nodia init` | Create a project scaffold. |
| `nodia version` | Print version metadata. |

`check` performs v0.5 semantic checks. It is not yet a static type or effect checker.

## 18. Baseline Stability Rule

Any language change after v0.5 must answer these questions:

1. Does the grammar change?
2. Does the AST schema change?
3. Does formatter output change?
4. Does runtime behavior change?
5. Does the corpus need new valid or invalid examples?
6. Does the change move Nodia toward the roadmap principles?

If a change cannot be represented in the specification, AST schema, formatter,
and corpus, it is not ready to enter the language.
