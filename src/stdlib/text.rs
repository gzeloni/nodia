// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Text and regex-backed standard-library helpers.

use super::expect_arity;
use crate::regex::{self, RegexMatch, RuntimeRegex};
use crate::value::Value;
use crate::{NodiaError, NodiaResult};
use std::collections::BTreeMap;

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
        Value::Regex(pattern) => pattern.replace_all(&text, &replacement)?,
        other => text.replace(&other.to_string(), &replacement),
    };
    Ok(Value::String(replaced))
}

pub(super) fn split_text(args: &[Value], name: &str) -> NodiaResult<Value> {
    expect_arity(&args, 2, name)?;
    let text = args[0].to_string();
    let parts = match &args[1] {
        Value::Regex(pattern) => pattern.split(&text)?,
        other => text
            .split(&other.to_string())
            .map(|part| part.to_string())
            .collect(),
    };
    Ok(Value::List(parts.into_iter().map(Value::String).collect()))
}

pub(super) fn regex_test(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 2, "test")?;
    let pattern = expect_regex(&args[1], "test", "second")?;
    Ok(Value::Bool(pattern.is_match(&args[0].to_string())?))
}

pub(super) fn regex_full_match(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 2, "full_match")?;
    let pattern = expect_regex(&args[1], "full_match", "second")?;
    Ok(Value::Bool(pattern.is_full_match(&args[0].to_string())?))
}

pub(super) fn regex_find(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 2, "find")?;
    let text = args[0].to_string();
    let pattern = expect_regex(&args[1], "find", "second")?;
    Ok(pattern
        .find(&text)?
        .map(regex_match_value)
        .unwrap_or(Value::Null))
}

pub(super) fn regex_find_all(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 2, "find_all")?;
    let text = args[0].to_string();
    let pattern = expect_regex(&args[1], "find_all", "second")?;
    Ok(Value::List(
        pattern
            .find_all(&text)?
            .into_iter()
            .map(regex_match_value)
            .collect(),
    ))
}

pub(super) fn contains_text(text: &str, needle: &Value) -> NodiaResult<bool> {
    match needle {
        Value::Regex(pattern) => pattern.is_match(text),
        other => Ok(text.contains(&other.to_string())),
    }
}

pub(super) fn text_starts_with(text: &str, prefix: &Value) -> NodiaResult<bool> {
    match prefix {
        Value::Regex(pattern) => Ok(pattern
            .find(text)?
            .is_some_and(|matched| matched.start == 0)),
        other => Ok(text.starts_with(&other.to_string())),
    }
}

pub(super) fn text_ends_with(text: &str, suffix: &Value) -> NodiaResult<bool> {
    match suffix {
        Value::Regex(pattern) => {
            let end = text.chars().count();
            Ok(pattern
                .find_all(text)?
                .into_iter()
                .any(|matched| matched.end == end))
        }
        other => Ok(text.ends_with(&other.to_string())),
    }
}

fn expect_regex(value: &Value, name: &str, position: &str) -> NodiaResult<RuntimeRegex> {
    match value {
        Value::Regex(pattern) => Ok(pattern.clone()),
        Value::String(pattern) => regex::compile_text(pattern),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects regex or string as {position} argument, got {}",
            other.type_name()
        ))),
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

pub(super) fn byte_len(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "byte_len")?;
    let text = expect_string(&args[0], "byte_len", "first")?;
    Ok(Value::Int(text.len() as i64))
}

pub(super) fn byte_offset(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "byte_offset")?;
    let text = expect_string(&args[0], "byte_offset", "first")?;
    let offset = expect_non_negative_offset(&args[1], "byte_offset", "second")?;
    Ok(Value::Int(
        scalar_to_byte_offset(text, offset, "byte_offset")? as i64,
    ))
}

pub(super) fn scalar_offset(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "scalar_offset")?;
    let text = expect_string(&args[0], "scalar_offset", "first")?;
    let offset = expect_non_negative_offset(&args[1], "scalar_offset", "second")?;
    Ok(Value::Int(
        byte_to_scalar_offset(text, offset, "scalar_offset")? as i64,
    ))
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

fn byte_to_scalar_offset(text: &str, offset: usize, name: &str) -> NodiaResult<usize> {
    if offset > text.len() {
        return Err(NodiaError::runtime(format!(
            "{name}() byte offset {offset} is out of range for text with {} byte(s)",
            text.len()
        )));
    }

    if !text.is_char_boundary(offset) {
        return Err(NodiaError::runtime(format!(
            "{name}() byte offset {offset} does not point to a UTF-8 boundary"
        )));
    }

    Ok(text[..offset].chars().count())
}
