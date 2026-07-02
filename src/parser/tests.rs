// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regression tests for parser behavior.

use super::*;
use crate::lexer::Lexer;

#[test]
fn parses_emit_and_bind() {
    let tokens = Lexer::new("val name = \"Ana\"\nemit \"Hi {name}\"")
        .tokenize()
        .unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn parses_multiline_maps_lists_and_calls() {
    let source = r#"
val user = {
  name: "Ana",
  tags: [
    "dev",
    "ops",
  ],
}

emit join(
  user.tags,
  ",",
)
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn parses_regex_expression() {
    let source = r#"
val date = regex(case_insensitive) {
  start
  named year {
    exactly 4 digit
  }
  "-"
  exactly 2 digit
  end
}
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn parses_explicit_regex_literals_and_scoped_flags() {
    let source = r#"
val pat = regex {
  with_flags(case_insensitive) {
    literal("abc")
  }
  any_codepoint
  char_set {
    char(".")
    digit
  }
}
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn parses_composite_assignment_and_map_pair_loop() {
    let source = r#"
var counts = {}
counts["ana"] = 1
for (key, value) in counts {
  emit "{key}={value}"
}
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 3);
}

#[test]
fn parses_keyword_map_keys_and_named_groups() {
    let source = r#"
val m = {from: "x", val: "y"}
val pat = regex {
  named val {
    one_or_more letter
  }
  same_as val
}
val hit = find("42", regex {
  named val {
    one_or_more digit
  }
})
emit m.from
emit hit.named.val
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 5);
}

#[test]
fn parses_lambda_expressions() {
    let tokens = Lexer::new("emit map(lambda(x) { x * 2 }, [1, 2, 3])")
        .tokenize()
        .unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn parses_bytes_literals() {
    let tokens = Lexer::new(r#"emit b"a\xff\0""#).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn parses_try_catch_and_throw_statements() {
    let source = r#"
try {
  throw {code: "E8000", message: "boom"}
} catch err {
  emit err.code
}
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();

    assert_eq!(program.statements.len(), 1);
    assert!(matches!(
        &program.statements[0],
        Stmt::Try {
            catch_name,
            catch_branch,
            ..
        } if catch_name == "err" && catch_branch.len() == 1
    ));
}

#[test]
fn parses_stdlib_use_modules() {
    let source = r#"
use text
use numbers
use collections as col

emit text.upper("hi")
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 4);
}

#[test]
fn parses_regex_namespace_calls_and_text_normalization_items() {
    let source = r#"
val pat = regex {
  r"\d{2}"
}
emit regex.find("42", pat)
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn parses_regex_conditionals() {
    let source = r#"
val pat = regex {
  optional group {
    "a"
  }
  "b"
  if_capture 1 then {
    "c"
  } else {
    "d"
  }
}
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn parses_extended_regex_dsl_surface() {
    let source = r#"
val pat = regex {
  start_text
  property "Greek"
  until {
    "END"
  }
  call_group 1
  if_matches {
    digit
  }
  define {
    named word {
      one_or_more letter
    }
  }
  fail
  end_text
}
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn parses_compound_assignment_operators() {
    let source = r#"
var x = 0
x += 5
x -= 2
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 3);
    assert!(matches!(&program.statements[1], Stmt::Assign { .. }));
    assert!(matches!(&program.statements[2], Stmt::Assign { .. }));
}

#[test]
fn parses_bitwise_precedence() {
    let source = r#"
emit ~1
emit 1 | 2 ^ 3 & 4 << 1
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn parses_namespace_declaration() {
    let source = r#"
namespace http {
  val timeout = 30
  func get(url) {
    return url
  }
}
emit http.timeout
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(&program.statements[0], Stmt::Namespace { name, .. } if name == "http"));
}

#[test]
fn parses_struct_declaration() {
    let source = r#"
struct Point {
  x: 0
  y: 0
}
emit Point.x
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 2);
    assert!(
        matches!(&program.statements[0], Stmt::Struct { name, fields } if name == "Point" && fields.len() == 2)
    );
}

#[test]
fn parses_struct_without_defaults() {
    let source = r#"
struct User {
  name
  age
}
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert!(
        matches!(&program.statements[0], Stmt::Struct { name, fields } if name == "User" && fields.len() == 2)
    );
}

#[test]
fn parses_enum_declaration() {
    let source = r#"
enum Status {
  active,
  inactive,
  pending,
}
emit Status.active
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 2);
    assert!(
        matches!(&program.statements[0], Stmt::Enum { name, variants } if name == "Status" && variants.len() == 3)
    );
}

#[test]
fn parses_type_alias() {
    let source = r#"
type Url = string
type Point = {x: float, y: float}
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(&program.statements[0], Stmt::TypeAlias { name, .. } if name == "Url"));
    assert!(matches!(&program.statements[1], Stmt::TypeAlias { name, .. } if name == "Point"));
}
