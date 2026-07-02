// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Runtime bindings for the low-level `scan` module.

use super::*;
use crate::scanner::{ScannerPattern, ScannerPosition, ScannerSpan, ScannerValue};

impl Runtime {
    pub(super) fn call_scan_builtin(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> NodiaResult<Option<Value>> {
        let result = match name {
            "scan.cursor" => {
                self.expect_arity(args, 1, "scan.cursor")?;
                let text = self.expect_string(&args[0], "scan.cursor", "first")?;
                Value::Scanner(ScannerValue::new(text))
            }
            "scan.at_end" => {
                self.expect_arity(args, 1, "scan.at_end")?;
                let scanner = self.expect_scanner(&args[0], "scan.at_end", "first")?;
                Value::Bool(scanner.at_end())
            }
            "scan.pos" => {
                self.expect_arity(args, 1, "scan.pos")?;
                let scanner = self.expect_scanner(&args[0], "scan.pos", "first")?;
                position_value(scanner.position())
            }
            "scan.lookahead" => self.lookahead_builtin(args)?,
            "scan.advance" => self.advance_builtin(args)?,
            "scan.match" => self.match_builtin(args)?,
            "scan.expect" => self.expect_builtin(args)?,
            "scan.take_while" => self.take_while_builtin(args)?,
            "scan.take_until" => self.take_until_builtin(args)?,
            "scan.span" => self.span_builtin(args)?,
            "scan.token" => self.token_builtin(args)?,
            "scan.error" => return self.scan_error_builtin(args).map(Some),
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn lookahead_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        match args {
            [scanner] => {
                let scanner = self.expect_scanner(scanner, "scan.lookahead", "first")?;
                Ok(scanner
                    .lookahead(1)
                    .map(Value::String)
                    .unwrap_or(Value::Null))
            }
            [scanner, count] => {
                let scanner = self.expect_scanner(scanner, "scan.lookahead", "first")?;
                let count = self.expect_non_negative_size(count, "scan.lookahead", "second")?;
                if count == 0 {
                    return Ok(Value::String(String::new()));
                }
                Ok(scanner
                    .lookahead(count)
                    .map(Value::String)
                    .unwrap_or(Value::Null))
            }
            _ => Err(NodiaError::runtime(format!(
                "scan.lookahead() expects 1 or 2 argument(s), got {}",
                args.len()
            ))),
        }
    }

    fn advance_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        match args {
            [scanner] => {
                let scanner = self.expect_scanner(scanner, "scan.advance", "first")?;
                Ok(Value::String(scanner.advance(1)))
            }
            [scanner, count] => {
                let scanner = self.expect_scanner(scanner, "scan.advance", "first")?;
                let count = self.expect_non_negative_size(count, "scan.advance", "second")?;
                Ok(Value::String(scanner.advance(count)))
            }
            _ => Err(NodiaError::runtime(format!(
                "scan.advance() expects 1 or 2 argument(s), got {}",
                args.len()
            ))),
        }
    }

    fn match_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "scan.match")?;
        let scanner = self.expect_scanner(&args[0], "scan.match", "first")?;
        let pattern = scan_pattern(&args[1], "scan.match", "second")?;
        Ok(scanner
            .take_match(pattern)?
            .map(span_value)
            .unwrap_or(Value::Null))
    }

    fn expect_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        if args.len() != 2 && args.len() != 3 {
            return Err(NodiaError::runtime(format!(
                "scan.expect() expects 2 or 3 argument(s), got {}",
                args.len()
            )));
        }
        let scanner = self.expect_scanner(&args[0], "scan.expect", "first")?;
        let pattern = scan_pattern(&args[1], "scan.expect", "second")?;
        let label = args
            .get(2)
            .map(|value| self.expect_string(value, "scan.expect", "third"))
            .transpose()?
            .unwrap_or_else(|| default_pattern_label(&args[1]));
        match scanner.take_match(pattern)? {
            Some(span) => Ok(span_value(span)),
            None => Err(scan_parse_error(
                "scan.expect",
                scanner.position(),
                format!("expected {label}"),
            )),
        }
    }

    fn take_while_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "scan.take_while")?;
        let scanner = self.expect_scanner(&args[0], "scan.take_while", "first")?;
        let pattern = scan_pattern(&args[1], "scan.take_while", "second")?;
        scanner.take_while(pattern).map(span_value)
    }

    fn take_until_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "scan.take_until")?;
        let scanner = self.expect_scanner(&args[0], "scan.take_until", "first")?;
        let pattern = scan_pattern(&args[1], "scan.take_until", "second")?;
        scanner.take_until(pattern).map(span_value)
    }

    fn span_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "scan.span")?;
        let scanner = self.expect_scanner(&args[0], "scan.span", "first")?;
        let start = expect_position_value(&args[1], "scan.span", "second")?;
        scanner.span_from(&start).map(span_value)
    }

    fn token_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "scan.token")?;
        let kind = self.expect_string(&args[0], "scan.token", "first")?;
        let span = expect_span_value(&args[1], "scan.token", "second")?;
        let text = span.text.clone();
        Ok(Value::Map(BTreeMap::from([
            ("kind".to_string(), Value::String(kind)),
            ("text".to_string(), Value::String(text)),
            ("span".to_string(), span_value(span)),
        ])))
    }

    fn scan_error_builtin(&self, args: &[Value]) -> NodiaResult<Value> {
        self.expect_arity(args, 2, "scan.error")?;
        let scanner = self.expect_scanner(&args[0], "scan.error", "first")?;
        let message = self.expect_string(&args[1], "scan.error", "second")?;
        Err(scan_parse_error("scan.error", scanner.position(), message))
    }
}

fn scan_pattern<'a>(
    value: &'a Value,
    name: &str,
    position: &str,
) -> NodiaResult<ScannerPattern<'a>> {
    match value {
        Value::String(value) => Ok(ScannerPattern::Literal(value)),
        Value::Regex(value) => Ok(ScannerPattern::Regex(value)),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects string or regex as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn default_pattern_label(value: &Value) -> String {
    match value {
        Value::String(value) => format!("{value:?}"),
        _ => "pattern".to_string(),
    }
}

fn scan_parse_error(
    context: &str,
    position: ScannerPosition,
    message: impl Into<String>,
) -> NodiaError {
    NodiaError::runtime(message)
        .with_code("E4300")
        .with_context(context)
        .with_span(position.line, position.column)
}

fn position_value(position: ScannerPosition) -> Value {
    Value::Map(BTreeMap::from([
        ("offset".to_string(), Value::Int(position.offset as i64)),
        ("line".to_string(), Value::Int(position.line as i64)),
        ("column".to_string(), Value::Int(position.column as i64)),
    ]))
}

fn span_value(span: ScannerSpan) -> Value {
    Value::Map(BTreeMap::from([
        ("text".to_string(), Value::String(span.text)),
        ("start".to_string(), position_value(span.start)),
        ("end".to_string(), position_value(span.end)),
    ]))
}

fn expect_position_value(
    value: &Value,
    name: &str,
    position: &str,
) -> NodiaResult<ScannerPosition> {
    let Value::Map(fields) = value else {
        return Err(NodiaError::runtime(format!(
            "{name}() expects position map as {position} argument, got {}",
            value.type_name()
        )));
    };
    Ok(ScannerPosition {
        offset: expect_non_negative_field(fields, "offset", name)?,
        line: expect_non_negative_field(fields, "line", name)?,
        column: expect_non_negative_field(fields, "column", name)?,
    })
}

fn expect_span_value(value: &Value, name: &str, position: &str) -> NodiaResult<ScannerSpan> {
    let Value::Map(fields) = value else {
        return Err(NodiaError::runtime(format!(
            "{name}() expects span map as {position} argument, got {}",
            value.type_name()
        )));
    };
    let Some(Value::String(text)) = fields.get("text") else {
        return Err(NodiaError::runtime(format!(
            "{name}() expects span.text as string"
        )));
    };
    let Some(start) = fields.get("start") else {
        return Err(NodiaError::runtime(format!("{name}() expects span.start")));
    };
    let Some(end) = fields.get("end") else {
        return Err(NodiaError::runtime(format!("{name}() expects span.end")));
    };
    Ok(ScannerSpan {
        text: text.clone(),
        start: expect_position_value(start, name, position)?,
        end: expect_position_value(end, name, position)?,
    })
}

fn expect_non_negative_field(
    fields: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> NodiaResult<usize> {
    match fields.get(key) {
        Some(Value::Int(value)) if *value >= 0 => usize::try_from(*value).map_err(|_| {
            NodiaError::runtime(format!("{name}() position field '{key}' overflowed"))
        }),
        _ => Err(NodiaError::runtime(format!(
            "{name}() expects position field '{key}' as non-negative int"
        ))),
    }
}
