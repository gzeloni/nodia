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
fn parses_stdlib_use_modules() {
    let source = r#"
use json
use csv as table pick read hide write
use re
use format as fmt

emit json.read("{}")
"#;
    let tokens = Lexer::new(source).tokenize().unwrap();
    let program = Parser::new(tokens).parse_program().unwrap();
    assert_eq!(program.statements.len(), 5);
}
