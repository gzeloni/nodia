// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Sequence manipulation helpers used by the standard library.

use super::*;
use std::cmp::Ordering;

pub(super) fn slice(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 3, "slice")?;
    let start = to_int(&args[1])?;
    let end = to_int(&args[2])?;
    match &args[0] {
        Value::List(values) => {
            let (start, end) = normalize_bounds(values.len(), start, end);
            Ok(Value::List(values[start..end].to_vec()))
        }
        Value::Bytes(value) => {
            let (start, end) = normalize_bounds(value.len(), start, end);
            Ok(Value::Bytes(value[start..end].to_vec()))
        }
        Value::String(value) => {
            let chars = value.chars().collect::<Vec<_>>();
            let (start, end) = normalize_bounds(chars.len(), start, end);
            Ok(Value::String(chars[start..end].iter().collect()))
        }
        other => Err(NodiaError::runtime(format!(
            "slice() expects list, bytes or string, got {}",
            other.type_name()
        ))),
    }
}

pub(super) fn reverse(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "reverse")?;
    match &args[0] {
        Value::List(values) => {
            let mut values = values.clone();
            values.reverse();
            Ok(Value::List(values))
        }
        Value::Bytes(value) => {
            let mut value = value.clone();
            value.reverse();
            Ok(Value::Bytes(value))
        }
        Value::String(value) => Ok(Value::String(value.chars().rev().collect())),
        other => Err(NodiaError::runtime(format!(
            "reverse() expects list, bytes or string, got {}",
            other.type_name()
        ))),
    }
}

pub(super) fn sort(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "sort")?;
    let mut values = expect_list(&args[0], "sort", "first")?.clone();
    values.sort_by(compare_values);
    Ok(Value::List(values))
}

pub(super) fn unique(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "unique")?;
    let values = expect_list(&args[0], "unique", "first")?;
    let mut out = Vec::new();
    for value in values {
        if !out.contains(value) {
            out.push(value.clone());
        }
    }
    Ok(Value::List(out))
}

pub(super) fn number_result(value: f64, args: &[Value]) -> Value {
    if args.iter().all(|arg| matches!(arg, Value::Int(_))) && value.fract() == 0.0 {
        Value::Int(value as i64)
    } else {
        Value::Float(value)
    }
}

fn normalize_bounds(len: usize, start: i64, end: i64) -> (usize, usize) {
    let len = len as i64;
    let start = normalize_index(len, start);
    let end = normalize_index(len, end);
    let start = start.clamp(0, len) as usize;
    let end = end.clamp(0, len) as usize;
    if end < start {
        (start, start)
    } else {
        (start, end)
    }
}

fn normalize_index(len: i64, index: i64) -> i64 {
    if index < 0 {
        len + index
    } else {
        index
    }
}

pub(super) fn compare_values(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal),
        (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal),
        (Value::String(a), Value::String(b)) => a.cmp(b),
        (Value::Bytes(a), Value::Bytes(b)) => a.cmp(b),
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Date(a), Value::Date(b)) => a.cmp(b),
        (Value::DateTime(a), Value::DateTime(b)) => a.cmp(b),
        (Value::Duration(a), Value::Duration(b)) => a.cmp(b),
        _ => value_rank(left)
            .cmp(&value_rank(right))
            .then_with(|| left.to_string().cmp(&right.to_string())),
    }
}

fn value_rank(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Int(_) | Value::Float(_) => 2,
        Value::String(_) => 3,
        Value::Bytes(_) => 4,
        Value::List(_) => 5,
        Value::Map(_) => 6,
        Value::Result(_) => 7,
        Value::Date(_) => 8,
        Value::DateTime(_) => 9,
        Value::Duration(_) => 10,
        Value::Regex(_) => 11,
        Value::Stream(_) => 12,
        Value::UseBinding(_, _) => 13,
        Value::BuiltinFunction(_) => 14,
        Value::Function(_) => 15,
    }
}
