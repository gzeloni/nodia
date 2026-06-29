// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Text and regex-backed standard-library helpers.

use super::expect_arity;
use super::result;
use crate::regex::{self, RegexMatch};
use crate::textcodec;
use crate::value::Value;
use crate::{NodiaError, NodiaResult};
use caseless::default_case_fold_str;
use std::collections::BTreeMap;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextUnit {
    Byte,
    Scalar,
    Grapheme,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextCodec {
    Utf8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DecodeMode {
    Strict,
    Lossy,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NormalizationForm {
    Lf,
    Crlf,
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RegexTestMode {
    Any,
    Full,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RegexFindMode {
    First,
    All,
}

pub(super) fn unary_string(
    args: &[Value],
    name: &str,
    f: impl FnOnce(String) -> String,
) -> NodiaResult<Value> {
    expect_arity(&args, 1, name)?;
    Ok(Value::String(f(args[0].to_string())))
}

pub(super) fn replace_text(args: &[Value], name: &str) -> NodiaResult<Value> {
    expect_arity(&args, 3, name)?;
    let text = args[0].to_string();
    let replacement = args[2].to_string();
    let replaced = match &args[1] {
        Value::Regex(pattern) => pattern
            .replace_all(&text, &replacement)
            .map_err(|error| error.with_context(format!("text.{name}")))?,
        other => text.replace(&other.to_string(), &replacement),
    };
    Ok(Value::String(replaced))
}

pub(super) fn split_text(args: &[Value], name: &str) -> NodiaResult<Value> {
    expect_arity(&args, 2, name)?;
    let text = args[0].to_string();
    let parts = match &args[1] {
        Value::Regex(pattern) => pattern
            .split(&text)
            .map_err(|error| error.with_context(format!("text.{name}")))?,
        other => text
            .split(&other.to_string())
            .map(|part| part.to_string())
            .collect(),
    };
    Ok(Value::List(parts.into_iter().map(Value::String).collect()))
}

pub(super) fn regex_test(args: &[Value], context: &str) -> NodiaResult<Value> {
    if !matches!(args.len(), 2 | 3) {
        return Err(NodiaError::runtime(format!(
            "test() expects 2 or 3 argument(s), got {}",
            args.len()
        )));
    }
    let text = args[0].to_string();
    let mode = if args.len() == 3 {
        expect_regex_test_mode(&args[2], "test", "third")?
    } else {
        RegexTestMode::Any
    };
    let outcome = match &args[1] {
        Value::Regex(pattern) => match mode {
            RegexTestMode::Any => pattern.is_match(&text).map(Value::Bool),
            RegexTestMode::Full => pattern.is_full_match(&text).map(Value::Bool),
        },
        Value::String(pattern) => {
            let pattern = regex::compile_text(pattern)?;
            match mode {
                RegexTestMode::Any => pattern.is_match(&text).map(Value::Bool),
                RegexTestMode::Full => pattern.is_full_match(&text).map(Value::Bool),
            }
        }
        other => {
            return Err(NodiaError::runtime(format!(
                "test() expects regex or string as second argument, got {}",
                other.type_name()
            )))
        }
    };
    Ok(result::capture_outcome_in_context(context, outcome))
}

pub(super) fn regex_find(args: &[Value], context: &str) -> NodiaResult<Value> {
    if !matches!(args.len(), 2 | 3) {
        return Err(NodiaError::runtime(format!(
            "find() expects 2 or 3 argument(s), got {}",
            args.len()
        )));
    }
    let text = args[0].to_string();
    let mode = if args.len() == 3 {
        expect_regex_find_mode(&args[2], "find", "third")?
    } else {
        RegexFindMode::First
    };
    let outcome = match &args[1] {
        Value::Regex(pattern) => match mode {
            RegexFindMode::First => Ok(pattern
                .find(&text)?
                .map(regex_match_value)
                .unwrap_or(Value::Null)),
            RegexFindMode::All => Ok(Value::List(
                pattern
                    .find_all(&text)?
                    .into_iter()
                    .map(regex_match_value)
                    .collect(),
            )),
        },
        Value::String(pattern) => {
            let pattern = regex::compile_text(pattern)?;
            match mode {
                RegexFindMode::First => Ok(pattern
                    .find(&text)?
                    .map(regex_match_value)
                    .unwrap_or(Value::Null)),
                RegexFindMode::All => Ok(Value::List(
                    pattern
                        .find_all(&text)?
                        .into_iter()
                        .map(regex_match_value)
                        .collect(),
                )),
            }
        }
        other => {
            return Err(NodiaError::runtime(format!(
                "find() expects regex or string as second argument, got {}",
                other.type_name()
            )))
        }
    };
    Ok(result::capture_outcome_in_context(context, outcome))
}

pub(super) fn contains_text(text: &str, needle: &Value) -> NodiaResult<bool> {
    match needle {
        Value::Regex(pattern) => pattern
            .is_match(text)
            .map_err(|error| error.with_context("text.contains")),
        other => Ok(text.contains(&other.to_string())),
    }
}

pub(super) fn text_starts_with(text: &str, prefix: &Value) -> NodiaResult<bool> {
    match prefix {
        Value::Regex(pattern) => Ok(pattern
            .find(text)
            .map_err(|error| error.with_context("text.starts"))?
            .is_some_and(|matched| matched.start == 0)),
        other => Ok(text.starts_with(&other.to_string())),
    }
}

pub(super) fn text_ends_with(text: &str, suffix: &Value) -> NodiaResult<bool> {
    match suffix {
        Value::Regex(pattern) => {
            let end = text.chars().count();
            Ok(pattern
                .find_all(text)
                .map_err(|error| error.with_context("text.ends"))?
                .into_iter()
                .any(|matched| matched.end == end))
        }
        other => Ok(text.ends_with(&other.to_string())),
    }
}

fn regex_match_value(matched: RegexMatch) -> Value {
    let mut named = BTreeMap::new();
    for (name, value) in matched.named {
        named.insert(name, option_string_value(value));
    }

    let mut fields = BTreeMap::new();
    fields.insert("text".to_string(), Value::String(matched.text));
    fields.insert("start".to_string(), Value::Int(matched.start as i64));
    fields.insert("end".to_string(), Value::Int(matched.end as i64));
    fields.insert(
        "groups".to_string(),
        Value::List(
            matched
                .groups
                .into_iter()
                .map(option_string_value)
                .collect(),
        ),
    );
    fields.insert("named".to_string(), Value::Map(named));
    Value::Map(fields)
}

fn option_string_value(value: Option<String>) -> Value {
    match value {
        Some(value) => Value::String(value),
        None => Value::Null,
    }
}

pub(super) fn indent(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 2, "indent")?;
    let text = args[0].to_string();
    let prefix = match &args[1] {
        Value::Int(size) => " ".repeat((*size).max(0) as usize),
        other => other.to_string(),
    };
    let mut out = String::with_capacity(text.len() + prefix.len() * text.lines().count());
    for (index, line) in text.lines().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&prefix);
        out.push_str(line);
    }
    Ok(Value::String(out))
}

pub(super) fn dedent(text: &str) -> String {
    let min_indent = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.chars()
                .take_while(|ch| *ch == ' ' || *ch == '\t')
                .count()
        })
        .min()
        .unwrap_or(0);

    text.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line.chars().skip(min_indent).collect()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn strip_bom(args: &[Value]) -> NodiaResult<Value> {
    unary_string(args, "strip_bom", |text| textcodec::strip_bom(&text))
}

pub(super) fn drop_nul(args: &[Value]) -> NodiaResult<Value> {
    unary_string(args, "drop_nul", |text| textcodec::drop_nul(&text))
}

pub(super) fn casefold(args: &[Value]) -> NodiaResult<Value> {
    unary_string(args, "casefold", |text| default_case_fold_str(&text))
}

pub(super) fn normalize(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "normalize")?;
    let text = expect_string(&args[0], "normalize", "first")?;
    let form = expect_normalization_form(&args[1], "normalize", "second")?;
    Ok(Value::String(match form {
        NormalizationForm::Lf => textcodec::normalize_lf(text),
        NormalizationForm::Crlf => textcodec::normalize_crlf(text),
        NormalizationForm::Nfc => text.nfc().collect(),
        NormalizationForm::Nfd => text.nfd().collect(),
        NormalizationForm::Nfkc => text.nfkc().collect(),
        NormalizationForm::Nfkd => text.nfkd().collect(),
    }))
}

pub(super) fn len(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "len")?;
    let text = expect_string(&args[0], "len", "first")?;
    let unit = expect_text_unit(&args[1], "len", "second")?;
    Ok(Value::Int(match unit {
        TextUnit::Byte => text.len() as i64,
        TextUnit::Scalar => text.chars().count() as i64,
        TextUnit::Grapheme => text.graphemes(true).count() as i64,
    }))
}

pub(super) fn encode(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "encode")?;
    let text = expect_string(&args[0], "encode", "first")?;
    let codec = expect_text_codec(&args[1], "encode", "second")?;
    match codec {
        TextCodec::Utf8 => Ok(textcodec::string_to_bytes_value(text)),
    }
}

pub(super) fn decode(args: &[Value]) -> NodiaResult<Value> {
    if !matches!(args.len(), 2 | 3) {
        return Err(NodiaError::runtime(format!(
            "decode() expects 2 or 3 argument(s), got {}",
            args.len()
        )));
    }

    let bytes = textcodec::expect_bytes(&args[0], "decode", "first")?;
    let codec = expect_text_codec(&args[1], "decode", "second")?;
    let mode = if args.len() == 3 {
        expect_decode_mode(&args[2], "decode", "third")?
    } else {
        DecodeMode::Strict
    };

    let outcome = match (codec, mode) {
        (TextCodec::Utf8, DecodeMode::Strict) => {
            textcodec::decode_utf8_runtime(bytes).map(Value::String)
        }
        (TextCodec::Utf8, DecodeMode::Lossy) => {
            Ok(Value::String(textcodec::decode_utf8_lossy(&bytes)))
        }
    };
    Ok(result::capture_outcome_in_context("text.decode", outcome))
}

pub(super) fn offset(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 4, "offset")?;
    let text = expect_string(&args[0], "offset", "first")?;
    let from = expect_text_unit(&args[1], "offset", "second")?;
    let to = expect_text_unit(&args[2], "offset", "third")?;
    let offset = expect_non_negative_offset(&args[3], "offset", "fourth")?;
    let byte_offset = unit_to_byte_offset(text, from, offset, "offset")?;
    Ok(Value::Int(
        byte_to_unit_offset(text, to, byte_offset, "offset")? as i64,
    ))
}

pub(super) fn at(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 3, "at")?;
    let text = expect_string(&args[0], "at", "first")?;
    let unit = expect_text_unit(&args[1], "at", "second")?;
    let index = expect_non_negative_index(&args[2], "at", "third")?;
    Ok(match unit {
        TextUnit::Byte => Value::Int(byte_at(text, index, "at")? as i64),
        TextUnit::Scalar => Value::String(scalar_at(text, index, "at")?),
        TextUnit::Grapheme => Value::String(grapheme_at(text, index, "at")?),
    })
}

pub(super) fn slice(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 4, "slice")?;
    let text = expect_string(&args[0], "slice", "first")?;
    let unit = expect_text_unit(&args[1], "slice", "second")?;
    let start = expect_non_negative_offset(&args[2], "slice", "third")?;
    let end = expect_non_negative_offset(&args[3], "slice", "fourth")?;
    validate_offset_order(start, end, unit_name(unit), "slice")?;
    let start_byte = unit_to_byte_offset(text, unit, start, "slice")?;
    let end_byte = unit_to_byte_offset(text, unit, end, "slice")?;
    Ok(Value::String(text[start_byte..end_byte].to_string()))
}

fn expect_string<'a>(value: &'a Value, name: &str, position: &str) -> NodiaResult<&'a str> {
    match value {
        Value::String(text) => Ok(text),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects string as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_named_string<'a>(
    value: &'a Value,
    name: &str,
    position: &str,
    kind: &str,
) -> NodiaResult<&'a str> {
    match value {
        Value::String(text) => Ok(text),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects {kind} as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_text_unit(value: &Value, name: &str, position: &str) -> NodiaResult<TextUnit> {
    match expect_named_string(value, name, position, "text unit")? {
        "byte" => Ok(TextUnit::Byte),
        "scalar" => Ok(TextUnit::Scalar),
        "grapheme" => Ok(TextUnit::Grapheme),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects byte, scalar, or grapheme as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_text_codec(value: &Value, name: &str, position: &str) -> NodiaResult<TextCodec> {
    match expect_named_string(value, name, position, "codec")? {
        "utf8" => Ok(TextCodec::Utf8),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects supported codec as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_decode_mode(value: &Value, name: &str, position: &str) -> NodiaResult<DecodeMode> {
    match expect_named_string(value, name, position, "decode mode")? {
        "strict" => Ok(DecodeMode::Strict),
        "lossy" => Ok(DecodeMode::Lossy),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects strict or lossy as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_normalization_form(
    value: &Value,
    name: &str,
    position: &str,
) -> NodiaResult<NormalizationForm> {
    match expect_named_string(value, name, position, "normalization form")? {
        "lf" => Ok(NormalizationForm::Lf),
        "crlf" => Ok(NormalizationForm::Crlf),
        "nfc" => Ok(NormalizationForm::Nfc),
        "nfd" => Ok(NormalizationForm::Nfd),
        "nfkc" => Ok(NormalizationForm::Nfkc),
        "nfkd" => Ok(NormalizationForm::Nfkd),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects lf, crlf, nfc, nfd, nfkc, or nfkd as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_regex_test_mode(value: &Value, name: &str, position: &str) -> NodiaResult<RegexTestMode> {
    match expect_named_string(value, name, position, "regex test mode")? {
        "any" => Ok(RegexTestMode::Any),
        "full" => Ok(RegexTestMode::Full),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects any or full as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_regex_find_mode(value: &Value, name: &str, position: &str) -> NodiaResult<RegexFindMode> {
    match expect_named_string(value, name, position, "regex find mode")? {
        "first" => Ok(RegexFindMode::First),
        "all" => Ok(RegexFindMode::All),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects first or all as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_non_negative_offset(value: &Value, name: &str, position: &str) -> NodiaResult<usize> {
    match value {
        Value::Int(value) if *value >= 0 => Ok(*value as usize),
        Value::Int(_) => Err(NodiaError::runtime(format!(
            "{name}() expects non-negative int as {position} argument"
        ))),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects int as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_non_negative_index(value: &Value, name: &str, position: &str) -> NodiaResult<usize> {
    match value {
        Value::Int(value) if *value >= 0 => Ok(*value as usize),
        Value::Int(_) => Err(NodiaError::runtime(format!(
            "{name}() expects non-negative int as {position} argument"
        ))),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects int as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn scalar_at(text: &str, index: usize, name: &str) -> NodiaResult<String> {
    let scalar_len = text.chars().count();
    text.chars().nth(index).map(|ch| ch.to_string()).ok_or_else(|| {
        NodiaError::runtime(format!(
            "{name}() scalar index {index} is out of range for text with {scalar_len} scalar value(s)"
        ))
    })
}

fn byte_at(text: &str, index: usize, name: &str) -> NodiaResult<u8> {
    text.as_bytes().get(index).copied().ok_or_else(|| {
        NodiaError::runtime(format!(
            "{name}() byte index {index} is out of range for text with {} byte(s)",
            text.len()
        ))
    })
}

fn grapheme_at(text: &str, index: usize, name: &str) -> NodiaResult<String> {
    let grapheme_len = text.graphemes(true).count();
    text.graphemes(true)
        .nth(index)
        .map(str::to_string)
        .ok_or_else(|| {
            NodiaError::runtime(format!(
                "{name}() grapheme index {index} is out of range for text with {grapheme_len} grapheme(s)"
            ))
        })
}

fn validate_offset_order(start: usize, end: usize, unit: &str, name: &str) -> NodiaResult<()> {
    if start > end {
        return Err(NodiaError::runtime(format!(
            "{name}() start {unit} offset {start} cannot be greater than end {unit} offset {end}"
        )));
    }
    Ok(())
}

fn unit_name(unit: TextUnit) -> &'static str {
    match unit {
        TextUnit::Byte => "byte",
        TextUnit::Scalar => "scalar",
        TextUnit::Grapheme => "grapheme",
    }
}

fn unit_to_byte_offset(
    text: &str,
    unit: TextUnit,
    offset: usize,
    name: &str,
) -> NodiaResult<usize> {
    match unit {
        TextUnit::Byte => {
            validate_byte_offset(text, offset, name)?;
            Ok(offset)
        }
        TextUnit::Scalar => scalar_to_byte_offset(text, offset, name),
        TextUnit::Grapheme => grapheme_to_byte_offset(text, offset, name),
    }
}

fn byte_to_unit_offset(
    text: &str,
    unit: TextUnit,
    offset: usize,
    name: &str,
) -> NodiaResult<usize> {
    match unit {
        TextUnit::Byte => {
            validate_byte_offset(text, offset, name)?;
            Ok(offset)
        }
        TextUnit::Scalar => byte_to_scalar_offset(text, offset, name),
        TextUnit::Grapheme => byte_to_grapheme_offset(text, offset, name),
    }
}

fn scalar_to_byte_offset(text: &str, offset: usize, name: &str) -> NodiaResult<usize> {
    let scalar_len = text.chars().count();
    if offset > scalar_len {
        return Err(NodiaError::runtime(format!(
            "{name}() scalar offset {offset} is out of range for text with {scalar_len} scalar value(s)"
        )));
    }

    if offset == scalar_len {
        return Ok(text.len());
    }

    Ok(text
        .char_indices()
        .nth(offset)
        .map(|(offset, _)| offset)
        .unwrap_or(text.len()))
}

fn grapheme_to_byte_offset(text: &str, offset: usize, name: &str) -> NodiaResult<usize> {
    let grapheme_len = text.graphemes(true).count();
    if offset > grapheme_len {
        return Err(NodiaError::runtime(format!(
            "{name}() grapheme offset {offset} is out of range for text with {grapheme_len} grapheme(s)"
        )));
    }

    if offset == grapheme_len {
        return Ok(text.len());
    }

    Ok(text
        .grapheme_indices(true)
        .nth(offset)
        .map(|(offset, _)| offset)
        .unwrap_or(text.len()))
}

fn validate_byte_offset(text: &str, offset: usize, name: &str) -> NodiaResult<()> {
    if offset > text.len() {
        return Err(NodiaError::runtime(format!(
            "{name}() byte offset {offset} is out of range for text with {} byte(s)",
            text.len()
        )));
    }

    if !text.is_char_boundary(offset) {
        return Err(NodiaError::runtime(format!(
            "{name}() byte offset {offset} is not a UTF-8 boundary in text with {} byte(s)",
            text.len()
        )));
    }

    Ok(())
}

fn byte_to_scalar_offset(text: &str, offset: usize, name: &str) -> NodiaResult<usize> {
    validate_byte_offset(text, offset, name)?;
    Ok(text[..offset].chars().count())
}

fn byte_to_grapheme_offset(text: &str, offset: usize, name: &str) -> NodiaResult<usize> {
    validate_byte_offset(text, offset, name)?;
    Ok(text[..offset].graphemes(true).count())
}
