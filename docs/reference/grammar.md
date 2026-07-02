# Grammar

This is the normative EBNF-like grammar for Nodia v0.8.3. The current surface
keeps the `0.7.5` text-semantics baseline, adds `try` / `catch` / `throw`,
structural `match`, inline `namespace` / `struct` / `enum` / `type`, function
defaults, and bitwise operators.

```text
program        = { statement terminator } EOF ;
terminator     = newline | semicolon | EOF ;

statement      = comment
               | use_stmt
               | var_stmt
               | val_stmt
               | func_stmt
               | return_stmt
               | throw_stmt
               | try_stmt
               | match_stmt
               | emit_stmt
               | namespace_stmt
               | struct_stmt
               | enum_stmt
               | type_stmt
               | if_stmt
               | for_stmt
               | while_stmt
               | break_stmt
               | continue_stmt
               | assign_stmt
               | expr_stmt ;

use_stmt       = "use" (string | identifier) { use_clause } ;
use_clause     = "as" identifier
               | "pick" use_names
               | "hide" use_names ;
use_names      = identifier { "," identifier } [","] ;

var_stmt       = "var" identifier "=" expression ;
val_stmt       = "val" identifier "=" expression ;
assign_stmt    = assign_target assign_op expression ;
assign_target  = identifier { ("." identifier) | ("[" expression "]") } ;
assign_op      = "=" | "+=" | "-=" ;

func_stmt      = "func" identifier "(" [params] ")" block ;
params         = param { "," param } [","] ;
param          = identifier [ "=" expression ] ;

return_stmt    = "return" [expression] ;
throw_stmt     = "throw" expression ;
try_stmt       = "try" block "catch" identifier block ;
match_stmt     = "match" expression "{" { match_arm terminator } default_arm "}" ;
match_arm      = "case" match_pattern block ;
default_arm    = "default" block ;
emit_stmt      = "emit" expression ;

namespace_stmt = "namespace" identifier block ;
struct_stmt    = "struct" identifier "{" { struct_field terminator } "}" ;
struct_field   = identifier [ ":" expression ] ;
enum_stmt      = "enum" identifier "{" identifier { "," identifier } [","] "}" ;
type_stmt      = "type" identifier "=" expression ;

if_stmt        = "if" expression block ["else" (if_stmt | block)] ;
for_stmt       = "for" for_binding "in" expression block ;
for_binding    = identifier | "(" identifier "," identifier ")" ;
while_stmt     = "while" expression block ;
break_stmt     = "break" ;
continue_stmt  = "continue" ;
expr_stmt      = expression ;

block          = "{" { statement terminator } "}" ;

match_pattern  = "_"
               | identifier
               | literal
               | "[" [match_pattern { "," match_pattern } [","]] "]"
               | "{"
                   [ map_pattern_entry { "," map_pattern_entry } [","] ]
                 "}" ;
map_pattern_entry = identifier
                  | (identifier | string) ":" match_pattern ;

expression     = or_expr ;
or_expr        = and_expr { "or" and_expr } ;
and_expr       = equality_expr { "and" equality_expr } ;
equality_expr  = compare_expr { ("==" | "!=") compare_expr } ;
compare_expr   = bit_or_expr { ("<" | "<=" | ">" | ">=") bit_or_expr } ;
bit_or_expr    = bit_xor_expr { "|" bit_xor_expr } ;
bit_xor_expr   = bit_and_expr { "^" bit_and_expr } ;
bit_and_expr   = shift_expr { "&" shift_expr } ;
shift_expr     = term_expr { ("<<" | ">>") term_expr } ;
term_expr      = factor_expr { ("+" | "-") factor_expr } ;
factor_expr    = unary_expr { ("*" | "/" | "%") unary_expr } ;
unary_expr     = ("-" | "not" | "~") unary_expr | call_expr ;
call_expr      = primary_expr { call_suffix | get_suffix | index_suffix } ;
call_suffix    = "(" [args] ")" ;
get_suffix     = "." identifier ;
index_suffix   = "[" expression "]" ;
args           = expression { "," expression } [","] ;

primary_expr   = literal
               | identifier
               | lambda_expr
               | regex_expr
               | "(" expression ")"
               | list_literal
               | map_literal ;

lambda_expr    = "lambda" "(" [params] ")" block ;

regex_expr     = "regex" [ "(" regex_flag { "," regex_flag } ")" ] regex_block ;
regex_flag     = identifier ;
regex_block    = "{" { regex_node terminator } "}" ;

list_literal   = "[" [args] "]" ;
map_literal    = "{" [map_pair { "," map_pair } [","]] "}" ;
map_pair       = (identifier | string) ":" expression ;
```

The regex DSL grammar (`regex_node`) is documented in
[Regex DSL](../language/regex.md) — it is large and best understood as a
recursive-descent specification rather than as a flat EBNF block.

## Lexical Notes

* Identifiers: `[_\p{L}][_\p{L}\p{N}]*`.
* Integer literals: base-10 signed 64-bit. The `-` is a unary operator.
* Float literals: digits on both sides of `.` and optional scientific notation
  (`1e10`, `1.5e-3`).
* String literals: `"..."`, `'...'`, `r"..."`, `r'...'`, `"""..."""`.
* Triple-quoted strings and `r`-prefixed strings are raw: no escapes, no
  interpolation.
* Comments: `# ...`, `// ...`, and `/* ... */`.

See [Source Files](../language/source.md) for narrative coverage.
