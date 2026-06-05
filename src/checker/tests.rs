// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regression tests for semantic checking.

use crate::check_source;

const TEST_STDLIB_PRELUDE: &str = r#"use text as __text
use collections as __col
use re as __re

val len = __col.len
val lines = __text.lines
val unlines = __text.unlines
val words = __text.words
val test = __re.test
val full_match = __re.full_match
val find = __re.find
val find_all = __re.find_all
val replace = __text.replace
val replace_all = __text.replace_all
val split = __text.split
val split_regex = __text.split_regex
"#;

fn check_stdlib_source(source: &str) -> crate::DobraResult<()> {
    check_source(&format!("{TEST_STDLIB_PRELUDE}\n{source}"))
}

#[test]
fn checker_accepts_text_builtins() {
    let source = r#"emit len(lines("a
b"))
emit unlines(["up", "down"])
emit len(words("one  two   three"))
emit test("abc", regex { one_or_more letter })
emit full_match("abc", regex { one_or_more letter })
emit find("abc", regex { one_or_more letter })
emit find_all("abc", regex { one_or_more letter })
emit replace("abc123", regex { one_or_more digit }, '#')
emit replace_all("abc123", regex { one_or_more digit }, '#')
emit split("ana   bruno", regex { one_or_more whitespace })
emit split_regex("ana   bruno", regex { one_or_more whitespace })
"#;

    assert!(check_stdlib_source(source).is_ok());
}

#[test]
fn checker_reports_first_identifier_occurrence() {
    let err = check_source("emit missing()\nemit missing()\n").unwrap_err();

    assert_eq!(err.code, "E4100");
    assert_eq!((err.line, err.column), (1, 6));
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
fn checker_rejects_regex_keyword_as_stdlib_module_name() {
    let err = check_source("use regex\n").unwrap_err();

    assert!(err.message.contains("use re for regex helpers"));
}
