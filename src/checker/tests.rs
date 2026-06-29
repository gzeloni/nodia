// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regression tests for semantic checking.

use crate::check_source;

const TEST_STDLIB_PRELUDE: &str = r#"use text as __text
use collections as __col
use result as __res
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
emit regex.test("abc", regex { one_or_more letter })
emit regex.test("abc", regex { one_or_more letter }, regex.full)
emit regex.find("abc", regex { one_or_more letter })
emit regex.find("abc", regex { one_or_more letter }, regex.all)
emit __text.replace("abc123", regex { one_or_more digit }, '#')
emit __text.split("ana   bruno", regex { one_or_more whitespace })
emit __text.len("é", __text.byte)
emit __text.normalize("é", __text.nfc)
emit __text.normalize("é", __text.nfd)
emit __text.normalize("①", __text.nfkc)
emit __text.normalize("①", __text.nfkd)
emit __text.casefold("Straße")
emit __text.offset("aéb", __text.scalar, __text.byte, 2)
emit __text.offset("aéb", __text.byte, __text.scalar, 3)
emit __text.at("éx", __text.scalar, 1)
emit __text.len("éx", __text.grapheme)
emit __text.at("éx", __text.grapheme, 0)
emit __text.slice("aéb", __text.byte, 1, 3)
emit __text.slice("éx", __text.scalar, 0, 2)
emit __text.slice("éx", __text.grapheme, 0, 1)
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
use io
use system
use result
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
emit result.raise(regex.find("ana 42", regex { one_or_more digit })).text
emit io.basename("/tmp/report.txt")
emit system.args[0]
emit result.is_err(result.err("E8000", "bad"))
emit dt.year(dt.date(2026, 6, 3))
emit col.map(numbers.int, ["1", "2"])
emit result.raise(decode(r'{"ok":true}')).ok
emit result.raise(table.read("name,age\nAna,30", {header: true, types: true}))[0].age + 1
emit encode({ok: true}, 2)
"#;

    assert!(check_source(source).is_ok());
}

#[test]
fn checker_rejects_direct_result_field_access() {
    let err = check_source(
        r#"use json
emit json.read(r'{"name":"Ana"}').name"#,
    )
    .unwrap_err();

    assert_eq!(err.code, "E4108");
    assert!(err.message.contains("cannot access field on result"));
}

#[test]
fn checker_rejects_direct_result_index_access() {
    let err = check_source(
        r#"use csv
emit csv.read("name,age\nAna,30", true)[0]"#,
    )
    .unwrap_err();

    assert_eq!(err.code, "E4108");
    assert!(err.message.contains("cannot index result"));
}

#[test]
fn checker_accepts_direct_pick_from_stdlib_modules() {
    let source = r#"use numbers pick range
use conversion pick string

for i in range(3) {
  emit string(i)
}"#;

    assert!(check_source(source).is_ok());
}

#[test]
fn checker_accepts_result_module_calls() {
    let source = r#"use result
use text
func fallback(error) {
  return error.message
}
val ok = result.ok("Ana")
val bad = result.err("E8000", "missing row")
emit result.is_ok(ok)
emit result.is_err(bad)
emit result.value(ok)
emit result.value_or(bad, "fallback")
emit result.error(bad).code
emit result.error(bad).context
emit result.error(bad).span.line
emit result.then(ok, text.upper)
emit result.recover(bad, fallback)
emit result.raise(ok)
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
    assert_eq!(
        err.span.as_ref().map(|span| (span.line, span.column)),
        Some((1, 1))
    );
}

#[test]
fn checker_allows_dollar_text_in_literal_text_replacement() {
    let source = r#"use text
emit text.replace("ana", "a", "$name")"#;

    assert!(check_source(source).is_ok());
}

#[test]
fn checker_rejects_invalid_literal_regex_text_for_regex_builtins() {
    let err = check_stdlib_source(r#"emit regex.test("ana", "[A-Z")"#).unwrap_err();

    assert_eq!(err.code, "E4200");
    assert!(err.message.contains("cannot compile regex"));
}

#[test]
fn checker_rejects_regex_keyword_as_stdlib_module_name() {
    let err = check_source("use regex\n").unwrap_err();

    assert!(err
        .message
        .contains("'regex' is built into the language and must not be imported"));
}

#[test]
fn checker_arity_errors_use_surface_call_names() {
    let err = check_source(
        r#"use text as txt
emit txt.upper("a", "b")"#,
    )
    .unwrap_err();

    assert_eq!(err.code, "E4107");
    assert!(err
        .message
        .contains("txt.upper() expects 1 argument(s), got 2"));
}
