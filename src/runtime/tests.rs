// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regression tests for runtime behavior.

use super::*;
use std::fs;

const TEST_STDLIB_PRELUDE: &str = r#"use text as __text
use collections as __col
use format as __fmt
use re as __re
use io as __io
use system as __sys
use datetime as __dt
use json as __json
use csv as __csv
"#;

fn stdlib_source(source: &str) -> String {
    format!("{TEST_STDLIB_PRELUDE}\n{source}")
}

fn run_source(source: &str, input: BTreeMap<String, Value>) -> NodiaResult<String> {
    crate::run_source(&stdlib_source(source), input)
}

fn run_source_with_options(
    source: &str,
    input: BTreeMap<String, Value>,
    options: RuntimeOptions,
) -> NodiaResult<String> {
    crate::run_source_with_options(&stdlib_source(source), input, options)
}

#[test]
fn emits_interpolated_input() {
    let mut input = BTreeMap::new();
    input.insert("name".to_string(), Value::String("Ana".to_string()));
    let output = run_source("val name = input.name\nemit \"Hello, {name}\"", input).unwrap();
    assert_eq!(output, "Hello, Ana");
}

#[test]
fn stdlib_globals_require_use() {
    let err = crate::run_source_with_options(
        "emit upper(\"ana\")",
        BTreeMap::new(),
        RuntimeOptions::default(),
    )
    .unwrap_err();
    assert!(err.message.contains("undefined variable 'upper'"));

    let err =
        crate::run_source_with_options("emit args[0]", BTreeMap::new(), RuntimeOptions::default())
            .unwrap_err();
    assert!(err.message.contains("undefined variable 'args'"));

    let err =
        crate::run_source_with_options("emit stdout", BTreeMap::new(), RuntimeOptions::default())
            .unwrap_err();
    assert!(err.message.contains("undefined variable 'stdout'"));
}

#[test]
fn division_always_returns_float() {
    let output = run_source("emit 9 / 3\nemit 10 / 3\n", BTreeMap::new()).unwrap();
    assert_eq!(output, "3.0\n3.3333333333333335");
}

#[test]
fn nested_functions_capture_outer_bindings() {
    let source = r#"func foo(a) {
  func bar() {
    return a + 1
  }
  return bar()
}

emit foo(4)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "5");
}

#[test]
fn returned_nested_functions_keep_captured_bindings() {
    let source = r#"func make_greeter(prefix) {
  func greet(name) {
    return "{prefix}, {name}"
  }
  return greet
}

val greet = make_greeter("Hi")
emit greet("Ana")
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "Hi, Ana");
}

#[test]
fn returned_nested_functions_keep_self_recursion() {
    let source = r#"func make_fact() {
  func fact(n) {
    if n <= 1 {
      return 1
    }
    return n * fact(n - 1)
  }
  return fact
}

val fact = make_fact()
emit fact(5)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "120");
}

#[test]
fn captured_var_counter_persists_across_calls() {
    let source = r#"func counter() {
  var n = 0
  func tick() {
    n = n + 1
    return n
  }
  return tick
}

val t = counter()
emit t()
emit t()
emit t()
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "1\n2\n3");
}

#[test]
fn captured_var_map_state_persists_across_calls() {
    let source = r#"func counter() {
  var state = {n: 0}
  func tick() {
    state.n = state.n + 1
    return state.n
  }
  return tick
}

val t = counter()
emit t()
emit t()
emit t()
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "1\n2\n3");
}

#[test]
fn map_assignment_and_pair_iteration_work() {
    let source = r#"var counts = {}
counts["ana"] = 2
counts.bruno = 3

for (name, total) in counts {
  emit "{name}={total}"
}

for (name, total) in __col.entries(counts) {
  emit "{name}:{total}"
}
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "ana=2\nbruno=3\nana:2\nbruno:3");
}

#[test]
fn nested_assignment_updates_used_module_maps() {
    let dir = std::env::temp_dir().join(format!("nodia-use-map-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let lib = dir.join("lib.nod");
    let main = dir.join("main.nod");
    fs::write(&lib, "var counts = {}\n").unwrap();
    fs::write(
            &main,
            "use './lib' as lib\nlib.counts.ana = 2\nlib.counts[\"bruno\"] = 3\nemit lib.counts.ana\nemit lib.counts.bruno\n",
        )
        .unwrap();

    let output = crate::run_file(&main, BTreeMap::new()).unwrap();
    assert_eq!(output, "2\n3");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn used_functions_keep_module_bindings() {
    let dir = std::env::temp_dir().join(format!("nodia-use-capture-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let lib = dir.join("lib.nod");
    let main = dir.join("main.nod");
    fs::write(
        &lib,
        "val prefix = \"Hi\"\nfunc greet(name) {\n  return \"{prefix}, {name}\"\n}\n",
    )
    .unwrap();
    fs::write(&main, "use './lib' as lib\nemit lib.greet(\"Ana\")\n").unwrap();

    let output = crate::run_file(&main, BTreeMap::new()).unwrap();
    assert_eq!(output, "Hi, Ana");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn direct_use_of_var_can_be_assigned() {
    let dir = std::env::temp_dir().join(format!("nodia-use-var-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let bar = dir.join("bar.nod");
    let main = dir.join("main.nod");
    fs::write(&bar, "var n = 0\n").unwrap();
    fs::write(
        &main,
        "use './bar' pick n\nwhile n < 3 {\n  emit n\n  n = n + 1\n}\n",
    )
    .unwrap();

    let output = crate::run_file(&main, BTreeMap::new()).unwrap();
    assert_eq!(output, "0\n1\n2");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn direct_use_of_val_cannot_be_assigned() {
    let dir = std::env::temp_dir().join(format!("nodia-use-val-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let bar = dir.join("bar.nod");
    let main = dir.join("main.nod");
    fs::write(&bar, "val n = 0\n").unwrap();
    fs::write(&main, "use './bar' pick n\nn = n + 1\n").unwrap();

    let err = crate::run_file(&main, BTreeMap::new()).unwrap_err();
    assert!(err.to_string().contains("cannot assign to val 'n'"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn circular_uses_are_cached_and_resolved_lazily() {
    let dir = std::env::temp_dir().join(format!("nodia-circular-use-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let a = dir.join("a.nod");
    let b = dir.join("b.nod");
    let main = dir.join("main.nod");
    fs::write(
        &a,
        "use './b' as b\nval name = \"A\"\nfunc pair() {\n  return \"{name}/{b.name}\"\n}\n",
    )
    .unwrap();
    fs::write(
        &b,
        "use './a' as a\nval name = \"B\"\nfunc pair() {\n  return \"{name}/{a.name}\"\n}\n",
    )
    .unwrap();
    fs::write(
        &main,
        "use './a' as a\nuse './b' as b\nemit a.pair()\nemit b.pair()\n",
    )
    .unwrap();

    let output = crate::run_file(&main, BTreeMap::new()).unwrap();
    assert_eq!(output, "A/B\nB/A");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_streams_read_and_write_lines() {
    let dir = std::env::temp_dir().join(format!("nodia-io-streams-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.txt");
    let output = dir.join("output.txt");
    fs::write(&input, "ana\nbruno\n").unwrap();

    let source = format!(
        r#"val src = __io.open("{}", "read")
val out = __io.open("{}", "write")

var line = __io.readln(src)
while line != null {{
  __io.writeln(out, __text.upper(line))
  line = __io.readln(src)
}}

__io.close(src)
__io.close(out)
emit __io.read("{}")
"#,
        input.display(),
        output.display(),
        output.display()
    );
    let output_text = run_source_with_options(
        &source,
        BTreeMap::new(),
        RuntimeOptions {
            allow_write: true,
            ..RuntimeOptions::default()
        },
    )
    .unwrap();

    assert_eq!(output_text, "ANA\nBRUNO");
    assert_eq!(fs::read_to_string(&output).unwrap(), "ANA\nBRUNO\n");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn readln_handles_final_line_without_trailing_newline() {
    let dir = std::env::temp_dir().join(format!("nodia-io-readln-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.txt");
    fs::write(&input, "ana\r\nbruno").unwrap();

    let source = format!(
        r#"val src = __io.open("{}", "read")
emit __io.readln(src)
emit __io.readln(src)
emit __io.readln(src)
"#,
        input.display(),
    );
    let output = run_source(&source, BTreeMap::new()).unwrap();

    assert_eq!(output, "ana\nbruno\nnull");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_reads_support_path_and_chunked_stream_access() {
    let dir = std::env::temp_dir().join(format!("nodia-io-read-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.txt");
    fs::write(&input, "abcdef").unwrap();

    let source = format!(
        r#"emit __io.read("{}")
val src = __io.open("{}", "read")
emit __io.read(src, 2)
emit __io.read(src, 2)
emit __io.read(src, 10)
emit __io.read(src, 10)
emit __io.eof(src)
"#,
        input.display(),
        input.display(),
    );
    let output = run_source(&source, BTreeMap::new()).unwrap();

    assert_eq!(output, "abcdef\nab\ncd\nef\n\ntrue");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn chunked_reads_keep_utf8_scalar_boundaries_and_zero_size_is_a_no_op() {
    let dir = std::env::temp_dir().join(format!("nodia-io-utf8-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.txt");
    fs::write(&input, "aéb").unwrap();

    let source = format!(
        r#"val src = __io.open("{}", "read")
emit __io.read(src, 0)
emit __io.eof(src)
emit __io.read(src, 1)
emit __io.read(src, 1)
emit __io.read(src, 1)
emit __io.read(src, 1)
emit __io.eof(src)
"#,
        input.display(),
    );
    let output = run_source(&source, BTreeMap::new()).unwrap();

    assert_eq!(output, "\nfalse\na\né\nb\n\ntrue");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_reads_reject_invalid_utf8_consistently() {
    let dir = std::env::temp_dir().join(format!("nodia-io-invalid-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();

    let bad_path = dir.join("bad-path.bin");
    fs::write(&bad_path, [0xff, b'a']).unwrap();
    let err = run_source(
        &format!(r#"emit __io.read("{}")"#, bad_path.display()),
        BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(err.code, "E3000");
    assert!(err.message.contains("invalid utf-8"));

    let bad_chunk = dir.join("bad-chunk.bin");
    fs::write(&bad_chunk, [0xff, b'a']).unwrap();
    let err = run_source(
        &format!(
            r#"val src = __io.open("{}", "read")
emit __io.read(src, 1)
"#,
            bad_chunk.display(),
        ),
        BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(err.code, "E3000");
    assert!(err.message.contains("invalid utf-8"));

    let bad_line = dir.join("bad-line.bin");
    fs::write(&bad_line, [b'a', 0xff, b'\n']).unwrap();
    let err = run_source(
        &format!(
            r#"val src = __io.open("{}", "read")
emit __io.readln(src)
"#,
            bad_line.display(),
        ),
        BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(err.code, "E3000");
    assert!(err.message.contains("invalid utf-8"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_writes_require_permission() {
    let dir = std::env::temp_dir().join(format!("nodia-io-denied-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let output = dir.join("output.txt");
    let source = format!("__io.write(\"{}\", \"blocked\")", output.display());

    let err = run_source_with_options(
        &source,
        BTreeMap::new(),
        RuntimeOptions {
            allow_write: false,
            ..RuntimeOptions::default()
        },
    )
    .unwrap_err();

    assert_eq!(err.code, "E3001");
    assert!(!output.exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn path_and_fs_builtins_cover_lexical_and_directory_queries() {
    let dir = std::env::temp_dir().join(format!("nodia-paths-{}", std::process::id()));
    let nested = dir.join("nested");
    fs::create_dir_all(&nested).unwrap();
    let alpha = dir.join("alpha.txt");
    let zeta = dir.join("zeta.log");
    let nested_file = nested.join("beta.txt");
    fs::write(&alpha, "a").unwrap();
    fs::write(&zeta, "z").unwrap();
    fs::write(&nested_file, "b").unwrap();

    let source = format!(
        r#"emit __io.basename("{alpha}")
emit __io.basename("/")
emit __io.dirname("{alpha}")
emit __io.dirname("plain.txt")
emit __io.exists("{alpha}")
emit __io.exists("{missing}")
emit __io.is_file("{alpha}")
emit __io.is_dir("{dir}")
emit __io.list_dir("{dir}")
emit __io.glob("{dir}/*.txt")
emit __io.glob("{dir}/**/*.txt")
"#,
        alpha = alpha.display(),
        missing = dir.join("missing.txt").display(),
        dir = dir.display(),
    );

    let output = run_source(&source, BTreeMap::new()).unwrap();
    assert_eq!(
            output,
            format!(
                "alpha.txt\n/\n{}\n.\ntrue\nfalse\ntrue\ntrue\n[\"alpha.txt\", \"nested\", \"zeta.log\"]\n[\"{}\"]\n[\"{}\", \"{}\"]",
                dir.display(),
                alpha.display(),
                alpha.display(),
                nested_file.display()
            )
        );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn regex_expression_renders_to_classic_regex_text() {
    let source = r#"emit regex(case_insensitive, multiline) {
  start
  named year {
    exactly 4 digit
  }
  "-"
  one_or_more char_set {
    range "a" to "z"
    digit
  }
  followed_by {
    ".log"
  }
  end
}"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, r"(?im)^(?<year>\d{4})-[a-z0-9]+(?=\.log)$");
}

#[test]
fn explicit_regex_forms_render_correctly() {
    let source = r#"emit regex {
  with_flags(case_insensitive) {
    literal("abc")
  }
  one_or_more any_codepoint
  char_set {
    char(".")
    digit
  }
}"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, r"(?i:abc)[\s\S]+[.0-9]");
}

#[test]
fn regex_builtins_execute_against_regex_values() {
    let source = r#"val pat = regex(case_insensitive) {
  named scheme {
    either {
      branch {
        "http"
      }
      branch {
        "https"
      }
    }
  }
  "://"
  named host {
    one_or_more {
      char_set {
        letter
        digit
        "."
        "-"
      }
    }
  }
}

val hit = __re.find("go to https://example.com now", pat)
emit __re.test("go to https://example.com now", pat)
emit __re.full_match("https://example.com", pat)
emit hit.text
emit hit.named.scheme
emit hit.named.host
emit hit.start
emit hit.end
emit __col.len(__re.find_all("http://a https://b", pat))
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(
        output,
        "true\ntrue\nhttps://example.com\nhttps\nexample.com\n6\n25\n2"
    );
}

#[test]
fn regex_find_reports_char_offsets() {
    let source = r#"val hit = __re.find("é ana", regex {
  named word {
    one_or_more letter
  }
})

emit hit.start
emit hit.end
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "2\n5");
}

#[test]
fn regex_offsets_align_with_slice_on_unicode_scalar_positions() {
    let source = r#"val text = "éx"
val hit = __re.find(text, regex {
  "x"
})

emit hit.start
emit hit.end
emit __col.slice(text, 0, hit.start)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "2\n3\né");
}

#[test]
fn text_semantics_expose_explicit_byte_offsets() {
    let source = r#"val text = "aéb"
emit __col.len(text)
emit __text.byte_len(text)
emit __text.byte_offset(text, 0)
emit __text.byte_offset(text, 1)
emit __text.byte_offset(text, 2)
emit __text.byte_offset(text, 3)
emit __text.scalar_offset(text, 0)
emit __text.scalar_offset(text, 1)
emit __text.scalar_offset(text, 3)
emit __text.scalar_offset(text, 4)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "3\n4\n0\n1\n3\n4\n0\n1\n2\n3");
}

#[test]
fn text_semantics_normalize_unicode_forms_explicitly() {
    let source = r#"val composed = "é"
val decomposed = "é"

emit composed == decomposed
emit __text.nfc(composed)
emit __text.nfc(decomposed)
emit __text.nfd(composed)
emit __text.nfd(decomposed)
emit __text.nfkc("①")
emit __text.nfkd("①")
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "false\né\né\né\né\n1\n1");
}

#[test]
fn text_semantics_casefolds_and_keeps_comparisons_explicit() {
    let source = r#"val composed = "é"
val decomposed = "é"
val words = ["Z", "é", "é", "ECLAIR", "éclair"]

func normalized_key(value) {
  return __text.casefold(__text.nfc(value))
}

emit __text.lower("Straße")
emit __text.casefold("Straße")
emit __text.casefold("STRASSE")
emit __text.casefold("Straße") == __text.casefold("STRASSE")
emit __text.contains(composed, decomposed)
emit __text.contains(__text.nfc(composed), __text.nfc(decomposed))
emit __col.unique([composed, decomposed])
emit __col.sort(words)
emit __col.sort_by(normalized_key, words)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(
        output,
        "straße\nstrasse\nstrasse\ntrue\nfalse\ntrue\n[\"é\", \"é\"]\n[\"ECLAIR\", \"Z\", \"é\", \"é\", \"éclair\"]\n[\"ECLAIR\", \"Z\", \"é\", \"é\", \"éclair\"]"
    );
}

#[test]
fn text_semantics_reject_invalid_offset_boundaries() {
    let err = run_source(r#"emit __text.scalar_offset("é", 1)"#, BTreeMap::new()).unwrap_err();
    assert!(err.message.contains("does not point to a UTF-8 boundary"));

    let err = run_source(r#"emit __text.byte_offset("é", 2)"#, BTreeMap::new()).unwrap_err();
    assert!(err
        .message
        .contains("scalar offset 2 is out of range for text with 1 scalar value(s)"));
}

#[test]
fn regex_builtins_accept_string_patterns() {
    let source = r#"emit __re.test("abc-42", "^[a-z]+-\\d+$")
emit __re.full_match("abc-42", "^[a-z]+-\\d+$")
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "true\ntrue");
}

#[test]
fn text_builtins_polymorphize_regex_needles() {
    let source = r#"emit __text.contains("abc42def", regex { one_or_more digit })
emit __text.starts("42x", regex { one_or_more digit })
emit __text.ends("x42", regex { one_or_more digit })
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "true\ntrue\ntrue");
}

#[test]
fn keyword_names_work_in_map_keys_and_named_groups() {
    let source = r#"val m = {from: "x", val: "y"}
val mirror = regex {
  named val {
    one_or_more letter
  }
  "-"
  same_as val
}
val digits = regex {
  named val {
    one_or_more digit
  }
}
val hit = __re.find("42", digits)

emit m.from
emit m.val
emit hit.named.val
emit __text.replace("ana-ana", mirror, "[$(val)]")
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "x\ny\n42\n[ana]");
}

#[test]
fn regex_replace_and_split_work_through_text_builtins() {
    let source = r#"val text = "go to https://example.com and http://ana.dev"
val url = regex {
  named scheme {
    either {
      branch {
        "http"
      }
      branch {
        "https"
      }
    }
  }
  "://"
  named host {
    one_or_more {
      char_set {
        letter
        digit
        "."
        "-"
      }
    }
  }
}

emit __text.replace(text, url, "<$(scheme):$(host)>")
emit __text.replace_all(text, url, "<$(host)>")
emit __text.split("ana   bruno\tcarla", regex {
  one_or_more whitespace
})
emit __text.split_regex("ana   bruno\tcarla", regex {
  one_or_more whitespace
})
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(
            output,
            "go to <https:example.com> and <http:ana.dev>\ngo to <example.com> and <ana.dev>\n[\"ana\", \"bruno\", \"carla\"]\n[\"ana\", \"bruno\", \"carla\"]"
        );
}

#[test]
fn regex_replace_reports_missing_capture_names() {
    let source = r#"emit __text.replace("ana", regex {
  named word {
    one_or_more letter
  }
}, "$(missing)")
"#;

    let err = run_source(source, BTreeMap::new()).unwrap_err();
    assert!(err
        .to_string()
        .contains("regex replacement refers to missing named capture 'missing'"));
}

#[test]
fn regex_replace_expands_unmatched_branch_capture_to_empty_string() {
    let source = r#"emit __text.replace("ana 42", regex {
  either {
    branch {
      named word {
        one_or_more letter
      }
    }
    branch {
      named num {
        one_or_more digit
      }
    }
  }
}, "<$(word):$(num)>")
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "<ana:> <:42>");
}

#[test]
fn regex_replace_handles_zero_width_matches_predictably() {
    let output = run_source(
        r#"emit __text.replace("abc", regex { start }, "<")
emit __text.replace("abc", regex { end }, ">")
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "<abc\nabc>");
}

#[test]
fn nested_string_values_render_with_quotes() {
    let output = run_source(
        r#"emit __text.split("/usr/local/bin", "/")
emit {name: "Ana", "full name": "Ana Maria", empty: ""}
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
            output,
            "[\"\", \"usr\", \"local\", \"bin\"]\n{empty: \"\", \"full name\": \"Ana Maria\", name: \"Ana\"}"
        );
}

#[test]
fn json_builtins_parse_and_stringify_structured_values() {
    let output = run_source(
            r#"val parsed = __json.read("{{\"name\":\"Ana\",\"flags\":[true,false],\"meta\":{{\"count\":2}},\"note\":\"line\\nnext\"}}")
emit parsed.name
emit parsed.flags
emit __json.write(parsed)
"#,
            BTreeMap::new(),
        )
        .unwrap();

    assert_eq!(
            output,
            "Ana\n[true, false]\n{\"flags\":[true,false],\"meta\":{\"count\":2},\"name\":\"Ana\",\"note\":\"line\\nnext\"}"
        );
}

#[test]
fn json_read_rejects_duplicate_object_keys() {
    let err = run_source(
        r#"emit __json.read(r'{"name":"Ana","name":"Bia"}')"#,
        BTreeMap::new(),
    )
    .unwrap_err();

    assert_eq!(err.code, "E2000");
    assert!(err.message.contains("duplicate object key 'name'"));
}

#[test]
fn json_stringify_supports_pretty_print() {
    let output = run_source(
        r#"emit __json.write({
  name: "Ana",
  scores: [1, 2],
}, 2)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "{\n  \"name\": \"Ana\",\n  \"scores\": [\n    1,\n    2\n  ]\n}"
    );
}

#[test]
fn json_stringify_zero_indent_stays_compact() {
    let output = run_source(
        r#"emit __json.write({
  name: "Ana",
  age: 30,
}, 0)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "{\"age\":30,\"name\":\"Ana\"}");
}

#[test]
fn json_stringify_accepts_indent_option_map() {
    let output = run_source(
        r#"emit __json.write({
  name: "Ana",
  scores: [1, 2],
}, {indent: 2})
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "{\n  \"name\": \"Ana\",\n  \"scores\": [\n    1,\n    2\n  ]\n}"
    );
}

#[test]
fn json_stringify_rejects_unknown_option_keys() {
    let err = run_source(
        r#"emit __json.write({name: "Ana"}, {indent: 2, mode: "wide"})"#,
        BTreeMap::new(),
    )
    .unwrap_err();

    assert_eq!(err.code, "E2000");
    assert!(err.message.contains("does not accept option 'mode'"));
}

#[test]
fn csv_builtins_read_headers_and_write_maps() {
    let output = run_source(
        r#"val rows = __csv.read("name,role\nAna,dev\n\"Bia, Jr\",ops", true)
emit rows[0].name
emit rows[1].name
emit __csv.write(rows)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "Ana\nBia, Jr\nname,role\nAna,dev\n\"Bia, Jr\",ops");
}

#[test]
fn csv_read_supports_header_and_type_options() {
    let output = run_source(
        r#"val users = __csv.read("name,age,active\nAna,30,true", {
  header: true,
  types: true,
})
val rows = __csv.read("1,2\n3,4", {types: true})
emit users[0].age + 2
emit users[0].active and true
emit rows[1][0] + rows[1][1]
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "32\ntrue\n7");
}

#[test]
fn csv_read_rejects_unknown_options_and_duplicate_headers() {
    let err = run_source(
        r#"emit __csv.read("name,age\nAna,30", {header: true, skip_empty: true})"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(err.code, "E2000");
    assert!(err.message.contains("does not accept option 'skip_empty'"));

    let err = run_source(
        r#"emit __csv.read("name,name\nAna,30", true)"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert_eq!(err.code, "E2000");
    assert!(err.message.contains("duplicate header 'name'"));
}

#[test]
fn csv_handles_embedded_newlines_and_escaped_quotes() {
    let output = run_source(
        r#"val rows = __csv.read("name,note\n\"Ana\",\"line 1\nline 2\"\n\"Bia\",\"say \"\"hi\"\"\"", true)
emit rows[0].note
emit rows[1].note
emit __csv.write(rows)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "line 1\nline 2\nsay \"hi\"\nname,note\nAna,\"line 1\nline 2\"\nBia,\"say \"\"hi\"\"\""
    );
}

#[test]
fn csv_write_uses_sorted_union_headers_and_empty_fields_for_missing_keys() {
    let output = run_source(
        r#"emit __csv.write([
  {role: "dev", name: "Ana"},
  {team: "core", name: "Bia"},
])"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "name,role,team\nAna,dev,\nBia,,core");
}

#[test]
fn stdlib_data_modules_are_available_via_use() {
    let output = run_source(
        r#"use json
use csv as table
val decode = json.read
val encode = json.write
val rows = table.read("name,age\nAna,30", {header: true, types: true})
emit decode(r'{"ok":true}').ok
emit rows[0].age + 1
emit encode(rows[0], 2)
emit __col.map(encode, [{n: 1}, {n: 2}])
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "true\n31\n{\n  \"age\": 30,\n  \"name\": \"Ana\"\n}\n[\"{\\\"n\\\":1}\", \"{\\\"n\\\":2}\"]"
    );
}

#[test]
fn stdlib_core_modules_are_available_via_use() {
    let output = run_source_with_options(
        r#"use text
use numbers
use collections as col
use conversion as conv
use format as fmt
use re
use io
use system
use datetime as dt

emit text.upper("ana")
emit numbers.abs(-4)
emit conv.string(3)
emit fmt.fixed(3.14, 1)
emit re.find("ana 42", regex { one_or_more digit }).text
emit io.basename("/tmp/report.txt")
emit system.args[1]
emit dt.year(dt.date(2026, 6, 3))
emit col.map(numbers.int, ["1", "2"])
"#,
        BTreeMap::new(),
        RuntimeOptions {
            args: vec!["zero".to_string(), "one".to_string()],
            ..RuntimeOptions::default()
        },
    )
    .unwrap();

    assert_eq!(output, "ANA\n4\n3\n3.1\n42\nreport.txt\none\n2026\n[1, 2]");
}

#[test]
fn stdlib_data_modules_work_from_file_execution() {
    let dir = std::env::temp_dir().join(format!("nodia-stdlib-use-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let main = dir.join("main.nod");
    fs::write(
        &main,
        "use json\nuse csv\n\nval decode = json.read\nval encode = json.write\nval rows = csv.read(\"name,age\\nAna,30\", {\n  header: true,\n  types: true,\n})\n\nemit decode(r'{\"ok\":true,\"name\":\"Ana\"}').name\nemit rows[0].age + 1\nemit encode(rows[0], 2)\nemit csv.write(rows)\n",
    )
    .unwrap();

    let output = crate::run_file(&main, BTreeMap::new()).unwrap();
    assert_eq!(
        output,
        "Ana\n31\n{\n  \"age\": 30,\n  \"name\": \"Ana\"\n}\nage,name\n30,Ana"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn get_builtin_returns_default_for_missing_entries() {
    let output = run_source(
        r#"var counts = {}
counts["ana"] = __col.get(counts, "ana", 0) + 1
counts["bia"] = __col.get(counts, "bia", 0) + 1
emit __col.get(counts, "ana", 0)
emit __col.get(counts, "carla", 0)
emit __col.get(["a", "b"], 3, "missing")
emit __col.get("nodia", -1, "?")
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "1\n0\nmissing\na");
}

#[test]
fn text_indexing_and_slicing_follow_current_contract() {
    let output = run_source(
        r#"emit ["a", "b", "c"][-1]
emit "nodia"[0]
emit "nodia"[-1]
emit __col.get("nodia", -1, "?")
emit __col.slice("nodia", -99, 99)
emit __col.len(__col.slice("nodia", 4, 2))
emit __col.slice("éx", 0, 1)
emit __col.slice("éx", 0, 2)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "c\nn\na\na\nnodia\n0\ne\né");
}

#[test]
fn direct_string_index_reports_bounds_with_length() {
    let err = run_source(r#"emit "nodia"[-9]"#, BTreeMap::new()).unwrap_err();

    assert!(err
        .message
        .contains("string index -9 out of bounds for length 5"));
}

#[test]
fn interpolation_handles_nested_braces_and_inner_string_literals() {
    let output = run_source(
        r#"emit "{ {name: \"Ana\"}[\"name\"] }"
emit "{regex { one_or_more digit }}"
emit "{__text.replace(\"xx\", \"x\", \"}\")}"
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "Ana\n\\d+\n}}");
}

#[test]
fn formatting_builtins_cover_padding_and_numeric_output() {
    let output = run_source(
        r#"emit __fmt.format("%05d %.2f %-6s", [7, 3.5, "ok"])
emit __fmt.pad_left("42", 5, "0")
emit __fmt.pad_right("ok", 5, ".")
emit __fmt.fixed(3.14159, 3)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "00007 3.50 ok    \n00042\nok...\n3.142");
}

#[test]
fn formatting_builtins_cover_percent_string_precision_and_multichar_padding() {
    let output = run_source(
        r#"emit __fmt.format("%% %.3s", ["Nodia"])
emit __fmt.pad_left("7", 5, "ab")
emit __fmt.pad_right("7", 5, "ab")
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "% Nod\nabab7\n7abab");
}

#[test]
fn args_binding_and_env_builtin_work_with_runtime_options() {
    let key = format!("NODIA_TEST_ENV_{}", std::process::id());
    std::env::set_var(&key, "present");

    let output = run_source_with_options(
            &format!(
                "emit __sys.args\nemit __sys.args[1]\nemit __sys.env(\"{key}\")\nemit __sys.env(\"{key}_MISSING\", \"fallback\")\n"
            ),
            BTreeMap::new(),
            RuntimeOptions {
                allow_env: true,
                args: vec!["one".to_string(), "two".to_string()],
                ..RuntimeOptions::default()
            },
        )
        .unwrap();

    assert_eq!(output, "[\"one\", \"two\"]\ntwo\npresent\nfallback");
    std::env::remove_var(key);
}

#[test]
fn env_builtin_requires_permission() {
    let err = run_source("emit __sys.env(\"HOME\")", BTreeMap::new()).unwrap_err();
    assert_eq!(err.code, "E3002");
}

#[test]
fn exit_builtin_returns_special_exit_error_with_output() {
    let err = run_source_with_options(
        "emit \"before\"\n__sys.exit(7)\nemit \"after\"\n",
        BTreeMap::new(),
        RuntimeOptions::default(),
    )
    .unwrap_err();

    assert_eq!(err.exit_status, Some(7));
    assert_eq!(err.output.as_deref(), Some("before"));
}

#[test]
fn exec_builtin_returns_stdout_stderr_and_status() {
    let output = run_source_with_options(
        r#"val result = __sys.exec("/bin/sh", [
  "-c",
  "printf out; printf err 1>&2; exit 7",
])
emit result.stdout
emit result.stderr
emit result.status
"#,
        BTreeMap::new(),
        RuntimeOptions {
            allow_process: true,
            ..RuntimeOptions::default()
        },
    )
    .unwrap();

    assert_eq!(output, "out\nerr\n7");
}

#[test]
fn exec_builtin_returns_recoverable_error_for_missing_binary() {
    let output = run_source_with_options(
        r#"val result = __sys.exec("nonexistent_xyz", [])
emit result.status
emit result.stdout == ""
emit result.stderr == ""
emit result.error != ""
"#,
        BTreeMap::new(),
        RuntimeOptions {
            allow_process: true,
            ..RuntimeOptions::default()
        },
    )
    .unwrap();

    assert_eq!(output, "-1\ntrue\ntrue\ntrue");
}

#[test]
fn exec_builtin_requires_permission() {
    let err = run_source(
        r#"emit __sys.exec("/bin/sh", [
  "-c",
  "exit 0",
]).status
"#,
        BTreeMap::new(),
    )
    .unwrap_err();

    assert_eq!(err.code, "E3003");
}

#[test]
fn higher_order_builtins_transform_and_group_lists() {
    let output = run_source(
        r#"func double(x) {
  return x * 2
}

func odd(x) {
  return x % 2 != 0
}

func add(acc, x) {
  return acc + x
}

func bucket(x) {
  if x < 10 {
    return "small"
  }
  return "big"
}

func age(user) {
  return user.age
}

val users = [
  {name: "Bia", age: 30},
  {name: "Ana", age: 20},
  {name: "Caio", age: 25},
]

emit __col.map(double, [1, 2, 3])
emit __col.filter(odd, [1, 2, 3, 4])
emit __col.reduce(add, 0, [1, 2, 3, 4])
emit __col.group_by(bucket, [3, 12, 8, 20])
emit __col.sort_by(age, users)[0].name
emit __col.sort_by(age, users)[2].name
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "[2, 4, 6]\n[1, 3]\n10\n{big: [12, 20], small: [3, 8]}\nAna\nBia"
    );
}

#[test]
fn lambda_expressions_work_inline_and_capture_bindings() {
    let output = run_source(
        r#"val factor = 3
emit __col.map(lambda(x) { x * factor }, [1, 2, 3])
emit __col.filter(lambda(x) { x % 2 != 0 }, [1, 2, 3, 4])
emit __col.reduce(lambda(acc, x) { acc + x }, 0, [1, 2, 3, 4])
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "[3, 6, 9]\n[1, 3]\n10");
}

#[test]
fn raw_and_triple_strings_preserve_literal_braces() {
    let output = run_source(
        r#"val doc = __json.read(r'{"a":1,"tpl":"hello {world}"}')
emit doc.a
emit doc.tpl
emit """{"nested":true}"""
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "1\nhello {world}\n{\"nested\":true}");
}

#[test]
fn scientific_notation_literals_work_in_source_and_json() {
    let output = run_source(
        r#"emit 1e3
emit 1.5e2
emit __json.write({big: 1e10})
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "1000.0\n150.0\n{\"big\":10000000000.0}");
}

#[test]
fn temporal_builtins_parse_format_and_expose_components() {
    let output = run_source(
        r#"val d = __dt.date(2026, 5, 27)
val dt = __dt.datetime({
  year: 2026,
  month: 5,
  day: 27,
  hour: 14,
  minute: 30,
  second: 5,
  nanosecond: 120000000,
  offset: "+05:30",
})

emit __dt.isoformat(d)
emit __dt.isoformat(dt)
emit __dt.strftime(dt, "%F %T %:z")
emit __dt.weekday_name(__dt.parse_date("2024-02-29"))
emit __dt.month_name(__dt.parse_date("2024-02-29"))
emit __dt.ordinal_day(__dt.parse_date("2024-02-29"))
emit __dt.iso_week(__dt.parse_date("2021-01-01")).week
emit __dt.offset_minutes(dt)
emit __dt.days_in_month(2024, 2)
emit __dt.is_leap_year(2024)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "2026-05-27\n2026-05-27T14:30:05.12+05:30\n2026-05-27 14:30:05 +05:30\nThursday\nFebruary\n60\n53\n330\n29\ntrue"
    );
}

#[test]
fn temporal_builtins_cover_epoch_arithmetic_and_json() {
    let output = run_source(
        r#"val end_of_jan = __dt.date(2024, 1, 31)
val stamp = __dt.parse_datetime("2024-01-31T23:00:00Z")
val jump = __dt.duration({hours: 2, minutes: 30})

emit __dt.isoformat(__dt.add_months(end_of_jan, 1))
emit __dt.isoformat(__dt.add_duration(stamp, jump))
emit __dt.isoformat(__dt.from_unix(0.5))
emit __dt.unix_seconds(__dt.from_unix(0.5))
emit __dt.isoformat(__dt.from_unix_ms(1500))
emit __dt.diff_days(__dt.date(2024, 3, 5), __dt.date(2024, 3, 1))
emit __dt.diff_seconds(__dt.parse_datetime("1970-01-01T00:00:01.5Z"), __dt.parse_datetime("1970-01-01T00:00:00Z"))
emit __json.write({
  d: __dt.date(2024, 2, 29),
  dt: __dt.parse_datetime("2024-02-29T12:00:00Z"),
  dur: __dt.duration({minutes: 90}),
})
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "2024-02-29\n2024-02-01T01:30:00Z\n1970-01-01T00:00:00.5Z\n0.5\n1970-01-01T00:00:01.5Z\n4\n1.5\n{\"d\":\"2024-02-29\",\"dt\":\"2024-02-29T12:00:00Z\",\"dur\":\"PT1H30M\"}"
    );
}

#[test]
fn temporal_values_compare_and_sort_consistently() {
    let output = run_source(
        r#"emit __col.sort([
  __dt.date(2024, 1, 3),
  __dt.date(2024, 1, 1),
  __dt.date(2024, 1, 2),
])
emit __dt.parse_datetime("2024-01-01T00:00:00+02:00") == __dt.parse_datetime("2023-12-31T22:00:00Z")
emit __dt.parse_datetime("2024-01-01T00:00:00Z") < __dt.parse_datetime("2024-01-02T00:00:00Z")
emit __dt.parse_duration("PT90S") > __dt.parse_duration("PT1M")
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "[2024-01-01, 2024-01-02, 2024-01-03]\ntrue\ntrue\ntrue"
    );
}
