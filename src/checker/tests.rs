// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regression tests for semantic checking.

use crate::check_source;

const TEST_STDLIB_PRELUDE: &str = r#"use text as __text
use collections as __col
use re as __re
"#;

fn check_stdlib_source(source: &str) -> crate::NodiaResult<()> {
    check_source(&format!("{TEST_STDLIB_PRELUDE}\n{source}"))
}

#[test]
fn checker_accepts_text_builtins() {
    let source = r#"emit __col.len(__text.lines("a
b"))
emit __text.unlines(["up", "down"])
emit __col.len(__text.words("one  two   three"))
emit __re.test("abc", regex { one_or_more letter })
emit __re.full_match("abc", regex { one_or_more letter })
emit __re.find("abc", regex { one_or_more letter })
emit __re.find_all("abc", regex { one_or_more letter })
emit __text.replace("abc123", regex { one_or_more digit }, '#')
emit __text.replace_all("abc123", regex { one_or_more digit }, '#')
emit __text.split("ana   bruno", regex { one_or_more whitespace })
emit __text.split_regex("ana   bruno", regex { one_or_more whitespace })
emit __text.byte_len("é")
emit __text.byte_offset("aéb", 2)
emit __text.scalar_offset("aéb", 3)
"#;

    assert!(check_stdlib_source(source).is_ok());
}

#[test]
fn checker_rejects_stdlib_globals_without_use() {
    let err = check_source(r#"emit upper("ana")"#).unwrap_err();
    assert_eq!(err.code, "E4100");
    assert!(err.message.contains("undefined variable 'upper'"));

    let err = check_source(r#"emit args"#).unwrap_err();
    assert_eq!(err.code, "E4100");
    assert!(err.message.contains("undefined variable 'args'"));

    let err = check_source(r#"emit stdout"#).unwrap_err();
    assert_eq!(err.code, "E4100");
    assert!(err.message.contains("undefined variable 'stdout'"));
}

#[test]
fn checker_rejects_empty_interpolation() {
    let err = check_source(r#"emit "{}""#).unwrap_err();

    assert_eq!(err.code, "E4106");
    assert!(err.message.contains("empty interpolation"));
}

#[test]
fn checker_wraps_invalid_interpolation_expressions() {
    let err = check_source(r#"emit "{name +}""#).unwrap_err();

    assert_eq!(err.code, "E4106");
    assert!(err.message.contains("invalid interpolation"));
}

#[test]
fn checker_accepts_interpolation_with_nested_balanced_braces() {
    let source = r#"emit "{ {name: \"Ana\"}[\"name\"] }"
emit "{regex { one_or_more digit }}"
"#;

    assert!(check_source(source).is_ok());
}

#[test]
fn checker_reports_first_identifier_occurrence() {
    let err = check_source("emit missing()\nemit missing()\n").unwrap_err();

    assert_eq!(err.code, "E4100");
    assert_eq!((err.line, err.column), (1, 6));
}

#[test]
fn checker_rejects_missing_known_map_key_in_index_access() {
    let err = check_source("val user = {name: \"Ana\"}\nemit user[\"role\"]\n").unwrap_err();

    assert_eq!(err.code, "E4105");
    assert!(err.message.contains("key 'role' not found"));
}

#[test]
fn checker_accepts_map_field_assignment_that_builds_shape() {
    let source = r#"var user = {}
user.name = "Ana"
emit user.name
"#;

    assert!(check_source(source).is_ok());
}

#[test]
fn checker_rejects_nested_assignment_through_val_root() {
    let err = check_source("val user = {}\nuser.name = \"Ana\"\n").unwrap_err();

    assert_eq!(err.code, "E4101");
    assert_eq!((err.line, err.column), (1, 5));
}

#[test]
fn checker_reports_first_keyword_occurrence() {
    let err = check_source("return\nreturn\n").unwrap_err();

    assert_eq!(err.code, "E4103");
    assert_eq!((err.line, err.column), (1, 1));
}

#[test]
fn checker_accepts_stdlib_use_modules_and_first_class_native_functions() {
    let source = r#"use text
use numbers
use collections as col
use conversion as conv
use format as fmt
use re
use io
use system
use datetime as dt
use json
use csv as table
val decode = json.read
val encode = json.write
val to_int = numbers.int
val out = io.stdout
emit text.upper("ana")
emit to_int("42")
emit conv.string(3)
emit fmt.fixed(3.14, 1)
emit re.find("ana 42", regex { one_or_more digit }).text
emit io.basename("/tmp/report.txt")
emit system.args[0]
emit dt.year(dt.date(2026, 6, 3))
emit col.map(numbers.int, ["1", "2"])
emit decode(r'{"ok":true}').ok
emit table.read("name,age\nAna,30", {header: true, types: true})[0].age + 1
emit encode({ok: true}, 2)
"#;

    assert!(check_source(source).is_ok());
}

#[test]
fn checker_rejects_missing_named_capture_in_literal_regex_replacement() {
    let err = check_source(
        r#"use text
emit text.replace("ana", regex { named word { one_or_more letter } }, "$(missing)")"#,
    )
    .unwrap_err();

    assert_eq!(err.code, "E4200");
    assert!(err
        .message
        .contains("regex replacement refers to missing named capture 'missing'"));
}

#[test]
fn checker_allows_dollar_text_in_literal_text_replacement() {
    let source = r#"use text
emit text.replace("ana", "a", "$name")"#;

    assert!(check_source(source).is_ok());
}

#[test]
fn checker_rejects_invalid_literal_regex_text_for_regex_builtins() {
    let err = check_stdlib_source(r#"emit __re.test("ana", "[A-Z")"#).unwrap_err();

    assert_eq!(err.code, "E4200");
    assert!(err.message.contains("cannot compile regex"));
}

#[test]
fn checker_rejects_regex_keyword_as_stdlib_module_name() {
    let err = check_source("use regex\n").unwrap_err();

    assert!(err.message.contains("use re for regex helpers"));
}
