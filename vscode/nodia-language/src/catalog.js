"use strict";

// Keep this catalog aligned with src/stdlib.rs.
const STDLIB_MODULES = {
  text: {
    summary: "Text helpers",
    members: {
      utf8: null,
      strict: null,
      lossy: null,
      lf: null,
      crlf: null,
      nfc: null,
      nfd: null,
      nfkc: null,
      nfkd: null,
      byte: null,
      scalar: null,
      grapheme: null,
      upper: [1],
      lower: [1],
      casefold: [1],
      capitalize: [1],
      trim: [1],
      normalize: [2],
      replace: [3],
      split: [2],
      join: [2],
      lines: [1],
      unlines: [1],
      words: [1],
      contains: [2],
      starts: [2],
      ends: [2],
      indent: [2],
      dedent: [1],
      len: [2],
      encode: [2],
      decode: [2, 3],
      strip_bom: [1],
      drop_nul: [1],
      offset: [4],
      at: [3],
      slice: [4]
    }
  },
  numbers: {
    summary: "Numeric helpers",
    members: {
      int: [1],
      float: [1],
      range: [1, 2],
      abs: [1],
      floor: [1],
      ceil: [1],
      round: [1],
      sqrt: [1],
      pow: [2],
      min: [2],
      max: [2],
      clamp: [3],
      sum: [1],
      avg: [1]
    }
  },
  conversion: {
    summary: "Scalar conversions",
    members: {
      string: [1],
      bool: [1],
      int: [1],
      float: [1]
    }
  },
  collections: {
    summary: "List and map helpers",
    members: {
      len: [1],
      keys: [1],
      values: [1],
      entries: [1],
      contains: [2],
      get: [3],
      push: [2],
      pop: [1],
      first: [1],
      last: [1],
      slice: [3],
      reverse: [1],
      sort: [1],
      unique: [1],
      map: [2],
      filter: [2],
      reduce: [3],
      group_by: [2],
      sort_by: [2]
    }
  },
  format: {
    summary: "Formatting helpers",
    members: {
      left: null,
      right: null,
      format: [2],
      pad: [3, 4],
      fixed: [2]
    }
  },
  re: {
    summary: "Regex helpers",
    members: {
      any: null,
      full: null,
      first: null,
      all: null,
      test: [2, 3],
      find: [2, 3],
      replace: [3],
      split: [2]
    }
  },
  io: {
    summary: "File, path, directory, and stream helpers",
    members: {
      stdin: null,
      stdout: null,
      stderr: null,
      text: null,
      bytes: null,
      open: [2],
      close: [1],
      flush: [1],
      eof: [1],
      read: [1, 2, 3],
      readln: [1],
      write: [2],
      writeln: [2],
      append: [2],
      basename: [1],
      dirname: [1],
      exists: [1],
      is_file: [1],
      is_dir: [1],
      list_dir: [1],
      glob: [1]
    }
  },
  system: {
    summary: "Process and environment helpers",
    members: {
      args: null,
      env: [1, 2],
      exit: [0, 1],
      exec: [1, 2]
    }
  },
  datetime: {
    summary: "Date, time, and duration helpers",
    members: {
      as_date: null,
      as_datetime: null,
      as_duration: null,
      seconds: null,
      milliseconds: null,
      days: null,
      months: null,
      years: null,
      span: null,
      start: null,
      end: null,
      now: [0, 1],
      today: [0, 1],
      date: [1, 3],
      datetime: [1, 6, 7],
      duration: [1],
      parse: [2],
      isoformat: [1],
      strftime: [2],
      from_epoch: [2, 3],
      epoch: [2],
      year: [1],
      month: [1],
      day: [1],
      hour: [1],
      minute: [1],
      second: [1],
      nanosecond: [1],
      weekday: [1],
      weekday_name: [1],
      month_name: [1],
      ordinal_day: [1],
      iso_week: [1],
      offset_minutes: [1],
      days_in_month: [1, 2],
      is_leap_year: [1],
      date_only: [1],
      with_offset: [2],
      add: [2, 3],
      diff: [3],
      bound: [2]
    }
  },
  json: {
    summary: "JSON encode and decode",
    members: {
      read: [1],
      write: [1, 2]
    }
  },
  csv: {
    summary: "CSV encode and decode",
    members: {
      read: [1, 2],
      write: [1]
    }
  }
};

const KEYWORD_SNIPPETS = [
  { label: "use", insertText: "use ${1:text}", detail: "Import a stdlib namespace or .nod module" },
  { label: "val", insertText: "val ${1:name} = ${0:value}", detail: "Immutable binding" },
  { label: "var", insertText: "var ${1:name} = ${0:value}", detail: "Mutable binding" },
  { label: "func", insertText: "func ${1:name}(${2}) {\n  $0\n}", detail: "Function declaration" },
  { label: "if", insertText: "if ${1:condition} {\n  $0\n}", detail: "Conditional block" },
  { label: "for", insertText: "for ${1:item} in ${2:items} {\n  $0\n}", detail: "For loop" },
  { label: "while", insertText: "while ${1:condition} {\n  $0\n}", detail: "While loop" },
  { label: "lambda", insertText: "lambda(${1:x}) { $0 }", detail: "Inline callback" },
  { label: "emit", insertText: "emit ${0:value}", detail: "Write to program output" },
  { label: "regex", insertText: "regex {\n  $0\n}", detail: "Regex DSL literal" }
];

const KEYWORDS = [
  "use",
  "as",
  "pick",
  "hide",
  "val",
  "var",
  "func",
  "return",
  "emit",
  "if",
  "else",
  "for",
  "in",
  "while",
  "break",
  "continue",
  "lambda",
  "and",
  "or",
  "not",
  "true",
  "false",
  "null",
  "regex"
];

const REGEX_FLAGS = [
  "case_insensitive",
  "multiline",
  "dot_all",
  "unicode",
  "ignore_whitespace",
  "ungreedy"
];

const REGEX_DSL_ITEMS = [
  { label: "start", detail: "Regex anchor" },
  { label: "end", detail: "Regex anchor" },
  { label: "word_boundary", detail: "Regex anchor" },
  { label: "not_word_boundary", detail: "Regex anchor" },
  { label: "digit", detail: "Regex class" },
  { label: "not_digit", detail: "Regex class" },
  { label: "whitespace", detail: "Regex class" },
  { label: "not_whitespace", detail: "Regex class" },
  { label: "word_char", detail: "Regex class" },
  { label: "not_word_char", detail: "Regex class" },
  { label: "letter", detail: "Regex class" },
  { label: "lowercase", detail: "Regex class" },
  { label: "uppercase", detail: "Regex class" },
  { label: "hex_digit", detail: "Regex class" },
  { label: "alnum", detail: "Regex class" },
  { label: "space", detail: "Regex class" },
  { label: "tab", detail: "Regex class" },
  { label: "newline", detail: "Regex class" },
  { label: "any_char", detail: "Regex item" },
  { label: "any_codepoint", detail: "Regex item" },
  { label: "literal", insertText: "literal(${1:\"text\"})", detail: "Escaped literal text" },
  { label: "raw_regex", insertText: "raw_regex ${1:\"\\\\d+\"}", detail: "Raw regex insert" },
  { label: "optional", insertText: "optional ${1:digit}", detail: "Quantifier" },
  { label: "zero_or_more", insertText: "zero_or_more ${1:digit}", detail: "Quantifier" },
  { label: "one_or_more", insertText: "one_or_more ${1:digit}", detail: "Quantifier" },
  { label: "exactly", insertText: "exactly ${1:2} ${2:digit}", detail: "Quantifier" },
  { label: "at_least", insertText: "at_least ${1:1} ${2:digit}", detail: "Quantifier" },
  { label: "between", insertText: "between ${1:1} and ${2:3} ${3:digit}", detail: "Quantifier" },
  { label: "group", insertText: "group {\n  $0\n}", detail: "Capturing group" },
  { label: "capture", insertText: "capture {\n  $0\n}", detail: "Capturing group" },
  { label: "non_capture", insertText: "non_capture {\n  $0\n}", detail: "Non-capturing group" },
  { label: "named", insertText: "named ${1:name} {\n  $0\n}", detail: "Named group" },
  { label: "atomic", insertText: "atomic {\n  $0\n}", detail: "Atomic group" },
  { label: "either", insertText: "either {\n  branch {\n    $1\n  }\n  branch {\n    $0\n  }\n}", detail: "Alternation" },
  { label: "branch", insertText: "branch {\n  $0\n}", detail: "Alternation branch" },
  { label: "char_set", insertText: "char_set { ${1:letter} }", detail: "Character set" },
  { label: "not_char_set", insertText: "not_char_set { ${1:whitespace} }", detail: "Negated character set" },
  { label: "range", insertText: "range ${1:\"a\"} to ${2:\"z\"}", detail: "Character range" },
  { label: "char", insertText: "char(${1:\"_\"})", detail: "Single character inside char_set" },
  { label: "followed_by", insertText: "followed_by {\n  $0\n}", detail: "Lookahead" },
  { label: "not_followed_by", insertText: "not_followed_by {\n  $0\n}", detail: "Negative lookahead" },
  { label: "preceded_by", insertText: "preceded_by {\n  $0\n}", detail: "Lookbehind" },
  { label: "not_preceded_by", insertText: "not_preceded_by {\n  $0\n}", detail: "Negative lookbehind" },
  { label: "same_as", insertText: "same_as ${1:name}", detail: "Named backreference" },
  { label: "same_as_group", insertText: "same_as_group ${1:1}", detail: "Indexed backreference" },
  { label: "with_flags", insertText: "with_flags(${1:case_insensitive}) {\n  $0\n}", detail: "Scoped flags" },
  { label: "without_flags", insertText: "without_flags(${1:multiline}) {\n  $0\n}", detail: "Scoped flags" },
  { label: "lazy", detail: "Quantifier mode" },
  { label: "possessive", detail: "Quantifier mode" }
];

function moduleNames() {
  return Object.keys(STDLIB_MODULES);
}

module.exports = {
  KEYWORDS,
  KEYWORD_SNIPPETS,
  REGEX_DSL_ITEMS,
  REGEX_FLAGS,
  STDLIB_MODULES,
  moduleNames
};
