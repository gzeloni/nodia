# Grammar

This is the normative EBNF-like grammar for Nodia v0.7. The `0.7.4` release
keeps the v0.6 surface syntax and extends the explicit text-semantics line
with normalization, case-folding, explicit UTF-8 codec helpers, bytes-aware
data parsing, and unit-aware text access helpers.

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
break_stmt    = "break" ;
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
               | lambda_expr
               | regex_expr
               | "(" expression ")"
               | list_literal
               | map_literal ;

lambda_expr    = "lambda" "(" [parameters] ")" block ;

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

* Identifiers: `[A-Za-z_][A-Za-z0-9_]*`.
* Integer literals: base-10 signed 64-bit. The `-` is a unary operator.
* Float literals: digits on both sides of `.` and optional scientific notation
  (`1e10`, `1.5e-3`).
* String literals: `"..."`, `'...'`, `r"..."`, `r'...'`, `"""..."""`.
* Triple-quoted strings and `r`-prefixed strings are raw: no escapes, no
  interpolation.
* Comments: `# ...` and `// ...`.

See [Source Files](../language/source.md) for narrative coverage.
