// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regression tests for runtime behavior.

use super::*;
use std::fs;

const TEST_STDLIB_PRELUDE: &str = r#"use text as __text
use collections as __col
use io as __io
use system as __sys
use datetime as __dt

func __error(task) {
  try {
    task()
    return null
  } catch err {
    return err
  }
}
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
fn stdlib_resolution_finds_json_module() {
    std::env::set_var("NODIA_ROOT", ".");
    let output = run_source(
        r#"use json
emit "loaded"
"#,
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(output, "loaded");
    std::env::remove_var("NODIA_ROOT");
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
fn bitwise_integer_operators_work() {
    let output = run_source(
        "emit ~1\nemit 5 & 3\nemit 5 | 2\nemit 5 ^ 1\nemit 1 << 3\nemit 8 >> 2\n",
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(output, "-2\n1\n7\n4\n8\n2");
}

#[test]
fn modules_export_namespace_struct_and_enum_values() {
    let dir = std::env::temp_dir().join(format!("nodia-module-exports-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let lib = dir.join("lib.nod");
    let main = dir.join("main.nod");
    fs::write(
        &lib,
        r#"namespace http {
  val timeout = 30
}

struct Point {
  x: 0
  y: 0
}

enum Status {
  active,
  inactive,
}
"#,
    )
    .unwrap();
    fs::write(
        &main,
        r#"use "./lib" as lib
emit lib.http.timeout
emit lib.Point.x
emit lib.Status.active.kind
"#,
    )
    .unwrap();

    let output = crate::run_file(&main, BTreeMap::new()).unwrap();
    assert_eq!(output, "30\n0\nactive");
    let _ = fs::remove_dir_all(dir);
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
fn stdout_stream_writes_share_the_program_output_channel() {
    let output = run_source(
        r#"__io.write(__io.stdout, "What")
__io.write(__io.stdout, " ")
__io.writeln(__io.stdout, "now?")
emit "Done"
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "What now?\nDone");
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
    let output = run_source(
        &format!(
            r#"emit __error(lambda() {{ __io.read("{}") }}).code"#,
            bad_path.display()
        ),
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(output, "E3000");

    let bad_chunk = dir.join("bad-chunk.bin");
    fs::write(&bad_chunk, [0xff, b'a']).unwrap();
    let output = run_source(
        &format!(
            r#"val src = __io.open("{}", "read")
emit __error(lambda() {{ __io.read(src, 1) }}).code
"#,
            bad_chunk.display(),
        ),
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(output, "E3000");

    let bad_line = dir.join("bad-line.bin");
    fs::write(&bad_line, [b'a', 0xff, b'\n']).unwrap();
    let output = run_source(
        &format!(
            r#"val src = __io.open("{}", "read")
emit __error(lambda() {{ __io.readln(src) }}).code
"#,
            bad_line.display(),
        ),
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(output, "E3000");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn explicit_codec_and_sanitation_helpers_are_available_through_text_module() {
    let output = run_source(
        r#"val encoded = __text.encode("aéb", __text.utf8)
emit encoded
emit __text.decode(encoded, __text.utf8)
emit __text.decode(b"a\xffb", __text.utf8, __text.lossy)
emit __text.normalize("a\r\nb\rc\n", __text.lf)
emit __text.normalize("a\r\nb\rc\n", __text.crlf)
emit __text.strip_bom(__text.decode(b"\xef\xbb\xbfhi", __text.utf8))
emit __text.drop_nul(__text.decode(b"a\0b\0", __text.utf8))
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "b\"aéb\"\naéb\na�b\na\nb\nc\n\na\r\nb\r\nc\r\n\nhi\nab"
    );
}

#[test]
fn byte_io_surfaces_raw_bytes_without_implicit_decoding() {
    let dir = std::env::temp_dir().join(format!("nodia-io-bytes-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("payload.bin");
    let source = format!(
        r#"__io.write("{}", b"a\0\xffb")
val raw = __io.read("{}", __io.bytes)
emit raw
emit __text.decode(raw, __text.utf8, __text.lossy)
"#,
        path.display(),
        path.display(),
    );

    let output = run_source_with_options(
        &source,
        BTreeMap::new(),
        RuntimeOptions {
            allow_write: true,
            ..RuntimeOptions::default()
        },
    )
    .unwrap();

    assert_eq!(output, "b\"a\\0\\xffb\"\na\0�b");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn bytes_are_first_class_sequence_values() {
    let output = run_source(
        r#"var raw = b"a\0\xffb"
emit __col.len(raw)
emit raw[2]
emit __col.get(raw, -1, null)
emit __col.contains(raw, 0)
emit __col.contains(raw, b"\xffb")
emit __col.slice(raw, 1, 3)
emit __col.reverse(raw)
raw[1] = 120
emit raw
for byte in raw {
  emit byte
}
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "4\n255\n98\ntrue\ntrue\nb\"\\0\\xff\"\nb\"b\\xff\\0a\"\nb\"ax\\xffb\"\n97\n120\n255\n98"
    );
}

#[test]
fn file_writes_require_permission() {
    let dir = std::env::temp_dir().join(format!("nodia-io-denied-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let output_path = dir.join("output.txt");

    let output = run_source_with_options(
        &format!(
            r#"emit __error(lambda() {{ __io.write("{}", "blocked") }}).code"#,
            output_path.display()
        ),
        BTreeMap::new(),
        RuntimeOptions {
            allow_write: false,
            ..RuntimeOptions::default()
        },
    )
    .unwrap();

    assert_eq!(output, "E3001");
    assert!(!output_path.exists());
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

val hit = regex.find("go to https://example.com now", pat)
emit regex.test("go to https://example.com now", pat)
emit regex.test("https://example.com", pat, regex.full)
emit hit.text
emit hit.named.scheme
emit hit.named.host
emit hit.start
emit hit.end
emit __col.len(regex.find("http://a https://b", pat, regex.all))
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(
        output,
        "true\ntrue\nhttps://example.com\nhttps\nexample.com\n6\n25\n2"
    );
}

#[test]
fn regex_find_reports_scalar_offsets() {
    let source = r#"val hit = regex.find("é ana", regex {
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
val hit = regex.find(text, regex {
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
fn text_semantics_expose_explicit_unit_offsets() {
    let source = r#"val text = "aéb"
emit __text.len(text, __text.scalar)
emit __text.len(text, __text.byte)
emit __text.offset(text, __text.scalar, __text.byte, 0)
emit __text.offset(text, __text.scalar, __text.byte, 1)
emit __text.offset(text, __text.scalar, __text.byte, 2)
emit __text.offset(text, __text.scalar, __text.byte, 3)
emit __text.offset(text, __text.byte, __text.scalar, 0)
emit __text.offset(text, __text.byte, __text.scalar, 1)
emit __text.offset(text, __text.byte, __text.scalar, 3)
emit __text.offset(text, __text.byte, __text.scalar, 4)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "3\n4\n0\n1\n3\n4\n0\n1\n2\n3");
}

#[test]
fn text_semantics_normalize_unicode_forms_explicitly() {
    let source = r#"val composed = "é"
val decomposed = "é"

emit composed == decomposed
emit __text.normalize(composed, __text.nfc)
emit __text.normalize(decomposed, __text.nfc)
emit __text.normalize(composed, __text.nfd)
emit __text.normalize(decomposed, __text.nfd)
emit __text.normalize("①", __text.nfkc)
emit __text.normalize("①", __text.nfkd)
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
  return __text.casefold(__text.normalize(value, __text.nfc))
}

emit __text.lower("Straße")
emit __text.casefold("Straße")
emit __text.casefold("STRASSE")
emit __text.casefold("Straße") == __text.casefold("STRASSE")
emit __text.contains(composed, decomposed)
emit __text.contains(
  __text.normalize(composed, __text.nfc),
  __text.normalize(decomposed, __text.nfc),
)
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
fn text_semantics_add_explicit_scalar_byte_and_grapheme_access_modes() {
    let source = r#"val text = "éx"
val bytes = "aéb"

emit __text.at(text, __text.scalar, 0)
emit __text.at(text, __text.scalar, 1)
emit __text.len(text, __text.grapheme)
emit __text.at(text, __text.grapheme, 0)
emit __text.slice(bytes, __text.byte, 1, 3)
emit __text.slice(text, __text.scalar, 0, 2)
emit __text.slice(text, __text.grapheme, 0, 1)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "e\ń\n2\né\né\né\né");
}

#[test]
fn text_semantics_report_invalid_unit_boundaries_precisely() {
    let err = run_source(
        r#"emit __text.at("nodia", __text.scalar, 9)"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert!(err
        .message
        .contains("at() scalar index 9 is out of range for text with 5 scalar value(s)"));

    let err = run_source(
        r#"emit __text.at("éx", __text.grapheme, 9)"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert!(err
        .message
        .contains("at() grapheme index 9 is out of range for text with 2 grapheme(s)"));

    let err = run_source(
        r#"emit __text.slice("é", __text.byte, 1, 2)"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert!(err
        .message
        .contains("slice() byte offset 1 is not a UTF-8 boundary in text with 2 byte(s)"));

    let err = run_source(
        r#"emit __text.slice("nodia", __text.scalar, 4, 2)"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert!(err
        .message
        .contains("slice() start scalar offset 4 cannot be greater than end scalar offset 2"));

    let err = run_source(
        r#"emit __text.slice("éx", __text.grapheme, 3, 4)"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert!(err
        .message
        .contains("slice() grapheme offset 3 is out of range for text with 2 grapheme(s)"));
}

#[test]
fn text_semantics_reject_invalid_offset_boundaries() {
    let err = run_source(
        r#"emit __text.offset("é", __text.byte, __text.scalar, 1)"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert!(err
        .message
        .contains("byte offset 1 is not a UTF-8 boundary in text with 2 byte(s)"));

    let err = run_source(
        r#"emit __text.offset("é", __text.scalar, __text.byte, 2)"#,
        BTreeMap::new(),
    )
    .unwrap_err();
    assert!(err
        .message
        .contains("scalar offset 2 is out of range for text with 1 scalar value(s)"));
}

#[test]
fn regex_builtins_accept_string_patterns() {
    let source = r#"emit regex.test("abc-42", "^[a-z]+-\\d+$")
emit regex.test("abc-42", "^[a-z]+-\\d+$", regex.full)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "true\ntrue");
}

#[test]
fn regex_string_patterns_report_regex_error_codes_at_runtime() {
    let output = run_source(
        r#"val pat = "[A-Z"
emit __error(lambda() { regex.test("abc", pat) }).code"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "E4200");
}

#[test]
fn regex_text_items_normalize_and_execute_as_native_regex_values() {
    let source = r#"val pat = regex {
  r"(?i)^\d{2}$"
}
emit pat
emit regex.test("42", pat, regex.full)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "(?i)^\\d{2}$\ntrue");
}

#[test]
fn regex_conditionals_execute_and_reverse_from_classic_text() {
    let source = r#"val dsl = regex {
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
val reversed = regex {
  r"(a)?b(?(1)c|d)"
}
val python_named = regex {
  r"(?P<word>[A-Za-z]+)\s+(?P=word)"
}

emit regex.test("abc", dsl, regex.full)
emit regex.test("bd", dsl, regex.full)
emit regex.test("abd", dsl, regex.full)
emit regex.test("abc", reversed, regex.full)
emit regex.test("bd", reversed, regex.full)
emit regex.test("ana ana", python_named, regex.full)
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(output, "true\ntrue\nfalse\ntrue\ntrue\ntrue");
}

#[test]
fn regex_raw_text_reversal_supports_fallback_only_features() {
    let source = r#"val scoped = regex {
  r"abc(?i)def"
}
val unicode = regex {
  r"\A\p{Greek}+\x41\Q.+\E\z"
}
val subroutine = regex {
  r"(?<num>\d+) x \g<num>"
}
val until = regex {
  r"(?~END)"
}

emit scoped
emit regex.test("abcDEF", scoped, regex.full)
emit regex.test("ABCdef", scoped, regex.full)
emit unicode
emit regex.test("ΩβA.+", unicode, regex.full)
emit regex.test("ΩβAx", unicode, regex.full)
emit subroutine
emit regex.test("12 x 34", subroutine, regex.full)
emit regex.find("AAENDZZ", until).text
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(
        output,
        "abc(?i:def)\ntrue\nfalse\n\\A\\p{Greek}+A\\.\\+\\z\ntrue\nfalse\n(?<num>\\d+) x \\g<num>\ntrue\nAA"
    );
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
val hit = regex.find("42", digits)

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
emit __text.replace(text, url, "<$(host)>")
emit __text.split("ana   bruno\tcarla", regex {
  one_or_more whitespace
})
"#;

    let output = run_source(source, BTreeMap::new()).unwrap();
    assert_eq!(
        output,
        "go to <https:example.com> and <http:ana.dev>\ngo to <example.com> and <ana.dev>\n[\"ana\", \"bruno\", \"carla\"]"
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
fn throw_and_try_catch_share_the_canonical_error_payload() {
    let output = run_source(
        r#"try {
  throw "boom"
} catch err {
  emit err.code
  emit err.message
}

try {
  throw {
    code: "E8000",
    message: "missing row",
  }
} catch err {
  emit err.code
  emit err.message
}
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "E2000\nboom\nE8000\nmissing row");
}

#[test]
fn try_catch_preserves_recoverable_context_and_span_details() {
    let output = run_source(
        r#"try {
  __text.decode(b"\xff", __text.utf8)
} catch err {
  emit err.context[0]
  emit err.span == null
}

try {
  __dt.parse("2024-99-99", __dt.as_date)
} catch err {
  emit err.context[0]
  emit err.message
}
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "text.decode\ntrue\ndatetime.parse\nmonth must be between 1 and 12"
    );
}

#[test]
fn try_catch_handles_recoverable_surfaces() {
    let missing = std::env::temp_dir()
        .join(format!("nodia-try-missing-{}", std::process::id()))
        .join("missing.txt");
    let source = format!(
        r#"func context_of(task) {{
  try {{
    task()
    return "ok"
  }} catch err {{
    return err.context[0]
  }}
}}

val bad_regex = "[A-Z"
emit context_of(lambda() {{
  __io.read("{}")
}})
emit context_of(lambda() {{
  regex.test("abc", bad_regex)
}})
emit context_of(lambda() {{
  __dt.parse("2024-99-99", __dt.as_date)
}})
"#,
        missing.display(),
    );
    let output = run_source(&source, BTreeMap::new()).unwrap();

    assert_eq!(output, "io.read\nregex.test\ndatetime.parse");
}

#[test]
fn stdlib_pick_can_import_selected_names_directly() {
    let output = run_source(
        r#"use numbers pick range
use conversion pick string

for i in range(3) {
  emit string(i)
}"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "0\n1\n2");
}

#[test]
fn stdlib_data_modules_work_from_file_execution() {
    let dir = std::env::temp_dir().join(format!("nodia-stdlib-use-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let main = dir.join("main.nod");
    fs::write(
        &main,
        "use text\nuse numbers\nuse collections as col\n\nemit text.upper(\"ana\")\nemit numbers.abs(-4)\nemit col.len([1, 2, 3])\n",
    )
    .unwrap();

    let output = crate::run_file(&main, BTreeMap::new()).unwrap();
    assert_eq!(output, "ANA\n4\n3");
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
        .contains("string scalar index -9 is out of range for text with 5 scalar value(s)"));
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
fn mirrored_output_still_preserves_controlled_exit_output() {
    let err = run_source_with_options(
        "__sys.exit(7)\n",
        BTreeMap::new(),
        RuntimeOptions {
            mirror_output: true,
            ..RuntimeOptions::default()
        },
    )
    .unwrap_err();

    assert_eq!(err.exit_status, Some(7));
    assert_eq!(err.output.as_deref(), Some(""));
}

#[test]
fn throw_without_catch_is_a_fatal_error() {
    let err = run_source(
        r#"throw {
  code: "E8000",
  message: "missing row",
}"#,
        BTreeMap::new(),
    )
    .unwrap_err();

    assert_eq!(err.code, "E8000");
    assert_eq!(err.message, "missing row");
}

#[test]
fn try_catch_does_not_intercept_exit_control_flow() {
    let err = run_source_with_options(
        r#"try {
  __sys.exit(7)
} catch err {
  emit err.code
}
"#,
        BTreeMap::new(),
        RuntimeOptions::default(),
    )
    .unwrap_err();

    assert_eq!(err.exit_status, Some(7));
}

#[test]
fn fatal_errors_preserve_partial_output() {
    let err = run_source(
        "emit \"before\"\nthrow {code: \"E8000\", message: \"missing row\"}\n",
        BTreeMap::new(),
    )
    .unwrap_err();

    assert_eq!(err.output.as_deref(), Some("before"));
}

#[test]
fn exec_builtin_returns_stdout_stderr_and_status() {
    let output = run_source_with_options(
        r#"val proc = __sys.exec("/bin/sh", [
  "-c",
  "printf out; printf err 1>&2; exit 7",
])
emit __text.decode(proc.stdout, __text.utf8)
emit __text.decode(proc.stderr, __text.utf8)
emit proc.status
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
        r#"val proc = __sys.exec("nonexistent_xyz", [])
emit proc.status
emit proc.stdout == b""
emit proc.stderr == b""
emit proc.error != ""
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
        r#"val text = r'hello {world}'
emit text
emit """{"nested":true}"""
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "hello {world}\n{\"nested\":true}");
}

#[test]
fn scientific_notation_literals_work_in_source_and_json() {
    let output = run_source(
        r#"emit 1e3
emit 1.5e2
emit {big: 1e10}.big
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "1000.0\n150.0\n10000000000.0");
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
emit __dt.weekday_name(__dt.parse("2024-02-29", __dt.as_date))
emit __dt.month_name(__dt.parse("2024-02-29", __dt.as_date))
emit __dt.ordinal_day(__dt.parse("2024-02-29", __dt.as_date))
emit __dt.iso_week(__dt.parse("2021-01-01", __dt.as_date)).week
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
val stamp = __dt.parse("2024-01-31T23:00:00Z", __dt.as_datetime)
val jump = __dt.duration({hours: 2, minutes: 30})

emit __dt.isoformat(__dt.add(end_of_jan, 1, __dt.months))
emit __dt.isoformat(__dt.add(stamp, jump))
emit __dt.isoformat(__dt.bound(end_of_jan, __dt.start))
emit __dt.isoformat(__dt.bound(end_of_jan, __dt.end))
emit __dt.isoformat(__dt.from_epoch(0.5, __dt.seconds))
emit __dt.epoch(__dt.from_epoch(0.5, __dt.seconds), __dt.seconds)
emit __dt.isoformat(__dt.from_epoch(1500, __dt.milliseconds))
emit __dt.diff(__dt.date(2024, 3, 5), __dt.date(2024, 3, 1), __dt.days)
emit __dt.diff(__dt.parse("1970-01-01T00:00:01.5Z", __dt.as_datetime), __dt.parse("1970-01-01T00:00:00Z", __dt.as_datetime), __dt.seconds)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "2024-02-29\n2024-02-01T01:30:00Z\n2024-01-31T00:00:00Z\n2024-01-31T23:59:59.999999999Z\n1970-01-01T00:00:00.5Z\n0.5\n1970-01-01T00:00:01.5Z\n4\n1.5"
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
emit __dt.parse("2024-01-01T00:00:00+02:00", __dt.as_datetime) == __dt.parse("2023-12-31T22:00:00Z", __dt.as_datetime)
emit __dt.parse("2024-01-01T00:00:00Z", __dt.as_datetime) < __dt.parse("2024-01-02T00:00:00Z", __dt.as_datetime)
emit __dt.parse("PT90S", __dt.as_duration) > __dt.parse("PT1M", __dt.as_duration)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(
        output,
        "[2024-01-01, 2024-01-02, 2024-01-03]\ntrue\ntrue\ntrue"
    );
}

#[test]
fn namespace_executes_body_and_exports_bindings() {
    let output = run_source(
        r#"namespace math {
  val pi = 3.14
  func double(x) { return x * 2 }
}
emit math.pi
emit math.double(21)
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "3.14\n42");
}

#[test]
fn struct_creates_map_with_defaults() {
    let output = run_source(
        r#"struct Point {
  x: 0
  y: 0
}
emit Point.x
emit Point.y
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "0\n0");
}

#[test]
fn struct_without_defaults_uses_null() {
    let output = run_source(
        r#"struct User {
  name
  age
}
emit User.name == null
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "true");
}

#[test]
fn enum_creates_variant_maps() {
    let output = run_source(
        r#"enum Status {
  active,
  inactive,
}
emit Status.active.kind
emit Status.inactive.kind
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "active\ninactive");
}

#[test]
fn type_alias_is_noop_at_runtime() {
    let output = run_source(
        r#"type Url = string
emit "ok"
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "ok");
}

#[test]
fn compound_addition_operator_works() {
    let output = run_source(
        r#"var count = 0
count += 5
count += 3
emit count
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "8");
}

#[test]
fn compound_subtraction_operator_works() {
    let output = run_source(
        r#"var total = 100
total -= 30
emit total
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "70");
}

#[test]
fn non_ascii_identifiers_work_at_runtime() {
    let output = run_source(
        r#"val nome = "Ana"
val idade = 30
emit nome
emit idade
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "Ana\n30");
}

#[test]
fn nested_namespace_produces_nested_maps() {
    let output = run_source(
        r#"namespace outer {
  val x = 1
  namespace inner {
    val y = 2
  }
}
emit outer.x
emit outer.inner.y
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "1\n2");
}

#[test]
fn net_listen_and_close_work() {
    let output = run_source(
        r#"use net
use io
val srv = net.listen("127.0.0.1:0")
io.close(srv)
emit "ok"
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "ok");
}

#[test]
fn math_random_returns_float_between_zero_and_one() {
    let output = run_source(
        r#"use math
val r = math.random()
emit r >= 0.0 and r < 1.0
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "true");
}

#[test]
fn math_random_int_returns_value_in_range() {
    let output = run_source(
        r#"use math
val r = math.random_int(10, 20)
emit r >= 10 and r <= 20
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "true");
}

#[test]
fn base64_roundtrip_preserves_data() {
    let output = run_source(
        r#"use base64
val original = b"Hello, Nodia!"
val encoded = base64.encode(original)
val decoded = base64.decode(encoded)
emit original == decoded
"#,
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(output, "true");
}
