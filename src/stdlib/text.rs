use super::*;
use crate::regex::{self, RegexMatch, RuntimeRegex};
use std::collections::BTreeMap;

pub(super) fn unary_string(
    args: &[Value],
    name: &str,
    f: impl FnOnce(String) -> String,
) -> DobraResult<Value> {
    expect_arity(&args, 1, name)?;
    Ok(Value::String(f(args[0].to_string())))
}

pub(super) fn replace_text(args: &[Value], name: &str) -> DobraResult<Value> {
    expect_arity(&args, 3, name)?;
    let text = args[0].to_string();
    let replacement = args[2].to_string();
    let replaced = match &args[1] {
        Value::Regex(pattern) => pattern.replace_all(&text, &replacement)?,
        other => text.replace(&other.to_string(), &replacement),
    };
    Ok(Value::String(replaced))
}

pub(super) fn split_text(args: &[Value], name: &str) -> DobraResult<Value> {
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

pub(super) fn regex_test(args: &[Value]) -> DobraResult<Value> {
    expect_arity(&args, 2, "test")?;
    let pattern = expect_regex(&args[1], "test", "second")?;
    Ok(Value::Bool(pattern.is_match(&args[0].to_string())?))
}

pub(super) fn regex_full_match(args: &[Value]) -> DobraResult<Value> {
    expect_arity(&args, 2, "full_match")?;
    let pattern = expect_regex(&args[1], "full_match", "second")?;
    Ok(Value::Bool(pattern.is_full_match(&args[0].to_string())?))
}

pub(super) fn regex_find(args: &[Value]) -> DobraResult<Value> {
    expect_arity(&args, 2, "find")?;
    let text = args[0].to_string();
    let pattern = expect_regex(&args[1], "find", "second")?;
    Ok(pattern
        .find(&text)?
        .map(regex_match_value)
        .unwrap_or(Value::Null))
}

pub(super) fn regex_find_all(args: &[Value]) -> DobraResult<Value> {
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

pub(super) fn contains_text(text: &str, needle: &Value) -> DobraResult<bool> {
    match needle {
        Value::Regex(pattern) => pattern.is_match(text),
        other => Ok(text.contains(&other.to_string())),
    }
}

pub(super) fn text_starts_with(text: &str, prefix: &Value) -> DobraResult<bool> {
    match prefix {
        Value::Regex(pattern) => Ok(pattern
            .find(text)?
            .is_some_and(|matched| matched.start == 0)),
        other => Ok(text.starts_with(&other.to_string())),
    }
}

pub(super) fn text_ends_with(text: &str, suffix: &Value) -> DobraResult<bool> {
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

fn expect_regex(value: &Value, name: &str, position: &str) -> DobraResult<RuntimeRegex> {
    match value {
        Value::Regex(pattern) => Ok(pattern.clone()),
        Value::String(pattern) => regex::compile_text(pattern),
        other => Err(DobraError::runtime(format!(
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

pub(super) fn indent(args: &[Value]) -> DobraResult<Value> {
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
