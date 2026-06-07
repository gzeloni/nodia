// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Recoverable result helpers.

use super::expect_arity;
use crate::value::{RecoverableErrorValue, ResultValue, Value};
use crate::{NodiaError, NodiaResult};

pub(super) fn ok(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "ok")?;
    Ok(Value::Result(ResultValue::ok(args[0].clone())))
}

pub(super) fn err(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "err")?;
    let code = expect_string(&args[0], "err", "first")?;
    let message = expect_string(&args[1], "err", "second")?;
    Ok(Value::Result(ResultValue::Err(RecoverableErrorValue::new(
        code, message,
    ))))
}

pub(super) fn is_ok(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "is_ok")?;
    Ok(Value::Bool(
        expect_result(&args[0], "is_ok", "first")?.is_ok(),
    ))
}

pub(super) fn is_err(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "is_err")?;
    Ok(Value::Bool(
        expect_result(&args[0], "is_err", "first")?.is_err(),
    ))
}

pub(super) fn value(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "value")?;
    Ok(expect_result(&args[0], "value", "first")?
        .value()
        .cloned()
        .unwrap_or(Value::Null))
}

pub(super) fn error(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "error")?;
    Ok(expect_result(&args[0], "error", "first")?
        .error()
        .map(|error| Value::Map(error.to_map()))
        .unwrap_or(Value::Null))
}

pub(super) fn raise(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "raise")?;
    match expect_result(&args[0], "raise", "first")? {
        ResultValue::Ok(value) => Ok((**value).clone()),
        ResultValue::Err(error) => Err(NodiaError {
            code: error.code.clone(),
            message: error.message.clone(),
            line: error.line.unwrap_or(0),
            column: error.column.unwrap_or(0),
            file: error.file.clone(),
            exit_status: None,
            output: None,
        }),
    }
}

fn expect_result<'a>(value: &'a Value, name: &str, position: &str) -> NodiaResult<&'a ResultValue> {
    match value {
        Value::Result(result) => Ok(result),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects result as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_string(value: &Value, name: &str, position: &str) -> NodiaResult<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects string as {position} argument, got {}",
            other.type_name()
        ))),
    }
}
