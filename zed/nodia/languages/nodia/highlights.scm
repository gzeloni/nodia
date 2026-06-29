(comment) @comment

(boolean) @constant.builtin
(null) @constant.builtin

[
  "use"
  "as"
  "pick"
  "hide"
  "val"
  "var"
  "func"
  "emit"
  "lambda"
  "regex"
] @keyword

[
  "if"
  "else"
  "for"
  "in"
  "while"
  "return"
] @keyword.control

(break_statement) @keyword.control

(continue_statement) @keyword.control

[
  "not"
  "and"
  "or"
] @operator

(function_declaration
  name: (identifier) @function)

(parameter_list
  (identifier) @variable.parameter)

(binding_statement
  name: (identifier) @variable)

(for_binding
  (identifier) @variable)

(use_statement
  target: (identifier) @module)

(use_statement
  target: (string) @string.special)

(use_statement
  target: (raw_string) @string.special)

(use_clause
  alias: (identifier) @module)

(call_expression
  function: (identifier) @function)

(member_expression
  object: (identifier) @variable)

(member_expression
  property: (identifier) @property)

(string) @string
(raw_string) @string
(triple_string) @string
(bytes_string) @string.special

(escape_sequence) @string.escape

(number) @number
(regex_number) @number

(regex_literal) @string.regex
(regex_anchor) @keyword
(regex_class) @type
(regex_any) @keyword
(regex_backtracking_verb) @keyword.control
(regex_quantifier_mode) @keyword
(regex_until_clear) @keyword

(regex_flags
  (regex_name) @attribute)

(regex_flag_names
  (regex_name) @attribute)

(regex_group
  name: (regex_name) @variable.parameter)

(regex_reference
  name: (regex_name) @variable)

(regex_subroutine_call
  name: (regex_name) @function)

(regex_conditional
  reference: (regex_name) @variable)

(regex_literal_call
  value: (string) @string.regex)

(regex_literal_call
  value: (raw_string) @string.regex)

(regex_raw_insert
  pattern: (string) @string.regex)

(regex_raw_insert
  pattern: (raw_string) @string.regex)

(regex_property
  name: (string) @string.special)

(regex_property
  name: (raw_string) @string.special)

(regex_char_set_property
  name: (string) @string.special)

(regex_char_set_property
  name: (raw_string) @string.special)
