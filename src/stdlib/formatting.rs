// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Text formatting standard-library functions.

use super::{expect_arity, expect_list, to_float, to_int};
use crate::error::{DobraError, DobraResult};
use crate::value::Value;

pub fn format(args: &[Value]) -> DobraResult<Value> {
    expect_arity(&args, 2, "format")?;
    let template = expect_string(&args[0], "format", "first")?;
    let values = expect_list(&args[1], "format", "second")?;
    let mut out = String::new();
    let chars = template.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut arg_index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        if ch != '%' {
            out.push(ch);
            index += 1;
            continue;
        }
        if chars.get(index + 1) == Some(&'%') {
            out.push('%');
            index += 2;
            continue;
        }

        index += 1;
        let mut left_align = false;
        let mut zero_pad = false;
        if chars.get(index) == Some(&'-') {
            left_align = true;
            index += 1;
        }
        if chars.get(index) == Some(&'0') {
            zero_pad = true;
            index += 1;
        }

        let width_start = index;
        while matches!(chars.get(index), Some('0'..='9')) {
            index += 1;
        }
        let width = if index > width_start {
            Some(parse_usize(
                &chars[width_start..index].iter().collect::<String>(),
                "format",
            )?)
        } else {
            None
        };

        let mut precision = None;
        if chars.get(index) == Some(&'.') {
            index += 1;
            let precision_start = index;
            while matches!(chars.get(index), Some('0'..='9')) {
                index += 1;
            }
            if index == precision_start {
                return Err(DobraError::runtime("format() expects digits after '.'"));
            }
            precision = Some(parse_usize(
                &chars[precision_start..index].iter().collect::<String>(),
                "format",
            )?);
        }

        let Some(spec) = chars.get(index).copied() else {
            return Err(DobraError::runtime(
                "format() found unterminated format specifier",
            ));
        };
        index += 1;

        let value = values.get(arg_index).ok_or_else(|| {
            DobraError::runtime(format!(
                "format() expected at least {} value(s), got {}",
                arg_index + 1,
                values.len()
            ))
        })?;
        arg_index += 1;

        let rendered = match spec {
            's' => format_string(value, precision),
            'd' => to_int(value)?.to_string(),
            'f' => format_float(to_float(value)?, precision.unwrap_or(6)),
            other => {
                return Err(DobraError::runtime(format!(
                    "format() does not support '%{other}'"
                )))
            }
        };
        let pad = if zero_pad && !left_align { "0" } else { " " };
        out.push_str(&apply_width(&rendered, width, left_align, pad)?);
    }

    if arg_index != values.len() {
        return Err(DobraError::runtime(format!(
            "format() used {} value(s), got {}",
            arg_index,
            values.len()
        )));
    }

    Ok(Value::String(out))
}

pub fn pad_left(args: &[Value]) -> DobraResult<Value> {
    pad(args, "pad_left", false)
}

pub fn pad_right(args: &[Value]) -> DobraResult<Value> {
    pad(args, "pad_right", true)
}

pub fn fixed(args: &[Value]) -> DobraResult<Value> {
    expect_arity(&args, 2, "fixed")?;
    let digits = expect_non_negative_usize(&args[1], "fixed", "second")?;
    Ok(Value::String(format_float(to_float(&args[0])?, digits)))
}

fn pad(args: &[Value], name: &str, right: bool) -> DobraResult<Value> {
    if args.len() != 2 && args.len() != 3 {
        return Err(DobraError::runtime(format!(
            "{name}() expects 2 or 3 argument(s), got {}",
            args.len()
        )));
    }
    let text = args[0].to_string();
    let width = expect_non_negative_usize(&args[1], name, "second")?;
    let pad = if let Some(value) = args.get(2) {
        expect_string(value, name, "third")?
    } else {
        " ".to_string()
    };
    let padded = apply_width(&text, Some(width), right, &pad)?;
    Ok(Value::String(padded))
}

fn format_string(value: &Value, precision: Option<usize>) -> String {
    let text = value.to_string();
    if let Some(precision) = precision {
        text.chars().take(precision).collect()
    } else {
        text
    }
}

fn format_float(value: f64, precision: usize) -> String {
    format!("{value:.precision$}")
}

fn apply_width(text: &str, width: Option<usize>, right: bool, pad: &str) -> DobraResult<String> {
    let Some(width) = width else {
        return Ok(text.to_string());
    };
    let len = text.chars().count();
    if len >= width {
        return Ok(text.to_string());
    }
    if pad.is_empty() {
        return Err(DobraError::runtime("padding string cannot be empty"));
    }

    let fill = repeated_pad(width - len, pad);
    if right {
        Ok(format!("{text}{fill}"))
    } else {
        Ok(format!("{fill}{text}"))
    }
}

fn repeated_pad(width: usize, pad: &str) -> String {
    let mut out = String::new();
    while out.chars().count() < width {
        out.push_str(pad);
    }
    out.chars().take(width).collect()
}

fn expect_string(value: &Value, name: &str, position: &str) -> DobraResult<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        other => Err(DobraError::runtime(format!(
            "{name}() expects string as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_non_negative_usize(value: &Value, name: &str, position: &str) -> DobraResult<usize> {
    match value {
        Value::Int(value) if *value >= 0 => Ok(*value as usize),
        Value::Int(_) => Err(DobraError::runtime(format!(
            "{name}() expects non-negative int as {position} argument"
        ))),
        other => Err(DobraError::runtime(format!(
            "{name}() expects int as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn parse_usize(text: &str, name: &str) -> DobraResult<usize> {
    text.parse::<usize>().map_err(|_| {
        DobraError::runtime(format!("{name}() could not parse numeric width/precision"))
    })
}
