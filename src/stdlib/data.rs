// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! JSON and CSV standard-library functions.

use super::{expect_arity, expect_list};
use crate::error::{NodiaError, NodiaResult};
use crate::value::Value;
use std::collections::BTreeMap;

pub fn json_read(args: &[Value]) -> NodiaResult<Value> {
    json_read_named(args, "json.read")
}

pub fn json_write(args: &[Value]) -> NodiaResult<Value> {
    json_write_named(args, "json.write")
}

pub fn csv_read(args: &[Value]) -> NodiaResult<Value> {
    csv_read_named(args, "csv.read")
}

pub fn csv_write(args: &[Value]) -> NodiaResult<Value> {
    csv_write_named(args, "csv.write")
}

fn json_read_named(args: &[Value], name: &str) -> NodiaResult<Value> {
    expect_arity(&args, 1, name)?;
    let text = expect_string(&args[0], name, "first")?;
    JsonParser::new(&text).parse()
}

fn json_write_named(args: &[Value], name: &str) -> NodiaResult<Value> {
    if args.len() != 1 && args.len() != 2 {
        return Err(NodiaError::runtime(format!(
            "{name}() expects 1 or 2 argument(s), got {}",
            args.len()
        )));
    }

    let options = if args.len() == 2 {
        json_stringify_options(&args[1], name)?
    } else {
        JsonStringifyOptions::default()
    };

    Ok(Value::String(stringify_json(&args[0], &options, 0, name)?))
}

fn csv_read_named(args: &[Value], name: &str) -> NodiaResult<Value> {
    if args.len() != 1 && args.len() != 2 {
        return Err(NodiaError::runtime(format!(
            "{name}() expects 1 or 2 argument(s), got {}",
            args.len()
        )));
    }

    let text = expect_string(&args[0], name, "first")?;
    let options = if args.len() == 2 {
        csv_read_options(&args[1], name)?
    } else {
        CsvReadOptions::default()
    };

    let rows = parse_csv_rows(&text, name)?;
    if !options.header {
        return Ok(Value::List(
            rows.into_iter()
                .map(|row| {
                    Value::List(
                        row.into_iter()
                            .map(|value| csv_field_value(value, options.types))
                            .collect(),
                    )
                })
                .collect(),
        ));
    }

    if rows.is_empty() {
        return Ok(Value::List(Vec::new()));
    }

    let headers = rows[0].clone();
    let mut mapped = Vec::new();
    for row in rows.into_iter().skip(1) {
        if row.len() != headers.len() {
            return Err(NodiaError::runtime(format!(
                "{name}() row has {} field(s), expected {} from header",
                row.len(),
                headers.len()
            )));
        }
        let mut map = BTreeMap::new();
        for (header, value) in headers.iter().zip(row) {
            map.insert(header.clone(), csv_field_value(value, options.types));
        }
        mapped.push(Value::Map(map));
    }
    Ok(Value::List(mapped))
}

fn csv_write_named(args: &[Value], name: &str) -> NodiaResult<Value> {
    expect_arity(&args, 1, name)?;
    let rows = expect_list(&args[0], name, "first")?;
    if rows.is_empty() {
        return Ok(Value::String(String::new()));
    }

    if rows.iter().all(|row| matches!(row, Value::List(_))) {
        return Ok(Value::String(write_list_rows(rows, name)?));
    }

    if rows.iter().all(|row| matches!(row, Value::Map(_))) {
        return Ok(Value::String(write_map_rows(rows, name)?));
    }

    Err(NodiaError::runtime(format!(
        "{name}() expects a list of rows where each row is a list or map"
    )))
}

fn write_list_rows(rows: &[Value], name: &str) -> NodiaResult<String> {
    let mut encoded = String::new();
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            encoded.push('\n');
        }
        let values = expect_list(row, name, "row")?;
        write_csv_record(
            &values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
            &mut encoded,
        );
    }
    Ok(encoded)
}

fn write_map_rows(rows: &[Value], _name: &str) -> NodiaResult<String> {
    let mut headers = Vec::new();
    for row in rows {
        let Value::Map(values) = row else {
            unreachable!();
        };
        for key in values.keys() {
            if !headers.contains(key) {
                headers.push(key.clone());
            }
        }
    }

    let mut encoded = String::new();
    write_csv_record(&headers, &mut encoded);
    for row in rows {
        encoded.push('\n');
        let Value::Map(values) = row else {
            unreachable!();
        };
        let fields = headers
            .iter()
            .map(|header| values.get(header).map(Value::to_string).unwrap_or_default())
            .collect::<Vec<_>>();
        write_csv_record(&fields, &mut encoded);
    }
    Ok(encoded)
}

fn write_csv_record(fields: &[String], out: &mut String) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        if field.contains([',', '"', '\n', '\r']) {
            out.push('"');
            for ch in field.chars() {
                if ch == '"' {
                    out.push('"');
                }
                out.push(ch);
            }
            out.push('"');
        } else {
            out.push_str(field);
        }
    }
}

fn parse_csv_rows(text: &str, name: &str) -> NodiaResult<Vec<Vec<String>>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }

    let chars = text.chars().collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut index = 0usize;
    let mut in_quotes = false;
    let mut just_closed_quote = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_quotes {
            match ch {
                '"' if chars.get(index + 1) == Some(&'"') => {
                    field.push('"');
                    index += 2;
                }
                '"' => {
                    in_quotes = false;
                    just_closed_quote = true;
                    index += 1;
                }
                _ => {
                    field.push(ch);
                    index += 1;
                }
            }
            continue;
        }

        if just_closed_quote {
            match ch {
                ',' => {
                    row.push(std::mem::take(&mut field));
                    just_closed_quote = false;
                    index += 1;
                }
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                    just_closed_quote = false;
                    index += 1;
                }
                '\r' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                    just_closed_quote = false;
                    index += 1;
                    if chars.get(index) == Some(&'\n') {
                        index += 1;
                    }
                }
                _ => {
                    return Err(NodiaError::runtime(format!(
                        "{name}() found characters after closing quote"
                    )))
                }
            }
            continue;
        }

        match ch {
            '"' if field.is_empty() => {
                in_quotes = true;
                index += 1;
            }
            '"' => {
                return Err(NodiaError::runtime(format!(
                    "{name}() found quote inside unquoted field"
                )))
            }
            ',' => {
                row.push(std::mem::take(&mut field));
                index += 1;
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                index += 1;
            }
            '\r' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                index += 1;
                if chars.get(index) == Some(&'\n') {
                    index += 1;
                }
            }
            _ => {
                field.push(ch);
                index += 1;
            }
        }
    }

    if in_quotes {
        return Err(NodiaError::runtime(format!(
            "{name}() found unterminated quoted field"
        )));
    }

    if just_closed_quote || !field.is_empty() || !row.is_empty() || text.ends_with(',') {
        row.push(field);
        rows.push(row);
    }

    Ok(rows)
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

fn expect_bool(value: &Value, name: &str, position: &str) -> NodiaResult<bool> {
    match value {
        Value::Bool(value) => Ok(*value),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects bool as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

#[derive(Clone, Copy, Default)]
struct CsvReadOptions {
    header: bool,
    types: bool,
}

#[derive(Clone, Copy, Default)]
struct JsonStringifyOptions {
    indent: Option<usize>,
}

fn csv_read_options(value: &Value, name: &str) -> NodiaResult<CsvReadOptions> {
    match value {
        Value::Bool(header) => Ok(CsvReadOptions {
            header: *header,
            types: false,
        }),
        Value::Map(options) => Ok(CsvReadOptions {
            header: option_bool(options, "header", name)?.unwrap_or(false),
            types: option_bool(options, "types", name)?.unwrap_or(false),
        }),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects bool or map as second argument, got {}",
            other.type_name()
        ))),
    }
}

fn json_stringify_options(value: &Value, name: &str) -> NodiaResult<JsonStringifyOptions> {
    match value {
        Value::Int(indent) => Ok(JsonStringifyOptions {
            indent: normalize_indent(expect_non_negative_int(*indent, name, "second")? as usize),
        }),
        Value::Map(options) => Ok(JsonStringifyOptions {
            indent: option_usize(options, "indent", name)?.and_then(normalize_indent),
        }),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects int or map as second argument, got {}",
            other.type_name()
        ))),
    }
}

fn option_bool(
    options: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> NodiaResult<Option<bool>> {
    options
        .get(key)
        .map(|value| expect_bool(value, name, &format!("option '{key}'")).map(Some))
        .unwrap_or(Ok(None))
}

fn option_usize(
    options: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> NodiaResult<Option<usize>> {
    match options.get(key) {
        Some(Value::Int(value)) => {
            Ok(Some(
                expect_non_negative_int(*value, name, &format!("option '{key}'"))? as usize,
            ))
        }
        Some(other) => Err(NodiaError::runtime(format!(
            "{name}() expects int as option '{key}', got {}",
            other.type_name()
        ))),
        None => Ok(None),
    }
}

fn expect_non_negative_int(value: i64, name: &str, position: &str) -> NodiaResult<i64> {
    if value < 0 {
        return Err(NodiaError::runtime(format!(
            "{name}() expects non-negative int as {position} argument"
        )));
    }
    Ok(value)
}

fn normalize_indent(indent: usize) -> Option<usize> {
    if indent == 0 {
        None
    } else {
        Some(indent)
    }
}

fn csv_field_value(value: String, coerce_types: bool) -> Value {
    if !coerce_types {
        return Value::String(value);
    }

    match value.as_str() {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ => value
            .parse::<i64>()
            .map(Value::Int)
            .or_else(|_| value.parse::<f64>().map(Value::Float))
            .unwrap_or(Value::String(value)),
    }
}

fn stringify_json(
    value: &Value,
    options: &JsonStringifyOptions,
    depth: usize,
    name: &str,
) -> NodiaResult<String> {
    match value {
        Value::Null => Ok("null".to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Int(value) => Ok(value.to_string()),
        Value::Float(value) => {
            if !value.is_finite() {
                return Err(NodiaError::runtime(format!(
                    "{name}() cannot encode NaN or infinite numbers"
                )));
            }
            Ok(format_float(*value))
        }
        Value::String(value) => Ok(format!("\"{}\"", escape_json_string(value))),
        Value::Date(value) => Ok(format!("\"{}\"", value.isoformat())),
        Value::DateTime(value) => Ok(format!("\"{}\"", value.isoformat())),
        Value::Duration(value) => Ok(format!("\"{}\"", value.isoformat())),
        Value::List(values) => {
            if values.is_empty() {
                return Ok("[]".to_string());
            }
            let mut encoded = Vec::new();
            for value in values {
                encoded.push(stringify_json(value, options, depth + 1, name)?);
            }
            if let Some(indent) = options.indent {
                let pad = " ".repeat(indent * (depth + 1));
                let close = " ".repeat(indent * depth);
                Ok(format!(
                    "[\n{pad}{}\n{close}]",
                    encoded.join(&format!(",\n{pad}"))
                ))
            } else {
                Ok(format!("[{}]", encoded.join(",")))
            }
        }
        Value::Map(values) => {
            if values.is_empty() {
                return Ok("{}".to_string());
            }
            let mut encoded = Vec::new();
            for (key, value) in values {
                let value = stringify_json(value, options, depth + 1, name)?;
                if options.indent.is_some() {
                    encoded.push(format!("\"{}\": {}", escape_json_string(key), value));
                } else {
                    encoded.push(format!("\"{}\":{}", escape_json_string(key), value));
                }
            }
            if let Some(indent) = options.indent {
                let pad = " ".repeat(indent * (depth + 1));
                let close = " ".repeat(indent * depth);
                Ok(format!(
                    "{{\n{pad}{}\n{close}}}",
                    encoded.join(&format!(",\n{pad}"))
                ))
            } else {
                Ok(format!("{{{}}}", encoded.join(",")))
            }
        }
        other => Err(NodiaError::runtime(format!(
            "{name}() does not accept {}",
            other.type_name()
        ))),
    }
}

fn format_float(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let text = value.to_string();
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

fn escape_json_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            ch if ch.is_control() => {
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            _ => out.push(ch),
        }
    }
    out
}

struct JsonParser<'a> {
    source: &'a str,
    chars: Vec<char>,
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            index: 0,
        }
    }

    fn parse(mut self) -> NodiaResult<Value> {
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.index != self.chars.len() {
            return Err(self.error("unexpected trailing characters"));
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> NodiaResult<Value> {
        self.skip_whitespace();
        let Some(ch) = self.peek() else {
            return Err(self.error("unexpected end of input"));
        };

        match ch {
            'n' => {
                self.consume_keyword("null")?;
                Ok(Value::Null)
            }
            't' => {
                self.consume_keyword("true")?;
                Ok(Value::Bool(true))
            }
            'f' => {
                self.consume_keyword("false")?;
                Ok(Value::Bool(false))
            }
            '"' => self.parse_string().map(Value::String),
            '[' => self.parse_array(),
            '{' => self.parse_object(),
            '-' | '0'..='9' => self.parse_number(),
            _ => Err(self.error("unexpected token")),
        }
    }

    fn parse_array(&mut self) -> NodiaResult<Value> {
        self.index += 1;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.peek() == Some(']') {
            self.index += 1;
            return Ok(Value::List(values));
        }

        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.index += 1;
                    self.skip_whitespace();
                }
                Some(']') => {
                    self.index += 1;
                    break;
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }

        Ok(Value::List(values))
    }

    fn parse_object(&mut self) -> NodiaResult<Value> {
        self.index += 1;
        self.skip_whitespace();
        let mut values = BTreeMap::new();
        if self.peek() == Some('}') {
            self.index += 1;
            return Ok(Value::Map(values));
        }

        loop {
            if self.peek() != Some('"') {
                return Err(self.error("expected string key"));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.peek() != Some(':') {
                return Err(self.error("expected ':' after object key"));
            }
            self.index += 1;
            self.skip_whitespace();
            let value = self.parse_value()?;
            values.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.index += 1;
                    self.skip_whitespace();
                }
                Some('}') => {
                    self.index += 1;
                    break;
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }

        Ok(Value::Map(values))
    }

    fn parse_string(&mut self) -> NodiaResult<String> {
        if self.peek() != Some('"') {
            return Err(self.error("expected string"));
        }
        self.index += 1;
        let mut out = String::new();

        while let Some(ch) = self.peek() {
            self.index += 1;
            match ch {
                '"' => return Ok(out),
                '\\' => out.push(self.parse_escape()?),
                ch if ch.is_control() => return Err(self.error("control character in string")),
                _ => out.push(ch),
            }
        }

        Err(self.error("unterminated string"))
    }

    fn parse_escape(&mut self) -> NodiaResult<char> {
        let Some(ch) = self.peek() else {
            return Err(self.error("unterminated escape"));
        };
        self.index += 1;
        match ch {
            '"' => Ok('"'),
            '\\' => Ok('\\'),
            '/' => Ok('/'),
            'b' => Ok('\u{08}'),
            'f' => Ok('\u{0C}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' => self.parse_unicode_escape(),
            _ => Err(self.error("invalid escape sequence")),
        }
    }

    fn parse_unicode_escape(&mut self) -> NodiaResult<char> {
        let code = self.parse_hex_code_unit()?;
        if (0xD800..=0xDBFF).contains(&code) {
            let saved = self.index;
            if self.peek() == Some('\\') && self.chars.get(self.index + 1) == Some(&'u') {
                self.index += 2;
                let low = self.parse_hex_code_unit()?;
                if (0xDC00..=0xDFFF).contains(&low) {
                    let combined =
                        0x10000 + (((code - 0xD800) as u32) << 10) + ((low - 0xDC00) as u32);
                    return char::from_u32(combined)
                        .ok_or_else(|| self.error("invalid unicode escape"));
                }
            }
            self.index = saved;
        }

        char::from_u32(code as u32).ok_or_else(|| self.error("invalid unicode escape"))
    }

    fn parse_hex_code_unit(&mut self) -> NodiaResult<u16> {
        let start = self.index;
        let end = start + 4;
        if end > self.chars.len() {
            return Err(self.error("incomplete unicode escape"));
        }

        let value = self.chars[start..end]
            .iter()
            .collect::<String>()
            .chars()
            .try_fold(0u16, |acc, ch| {
                ch.to_digit(16)
                    .map(|digit| (acc << 4) + digit as u16)
                    .ok_or(())
            })
            .map_err(|_| self.error("invalid unicode escape"))?;
        self.index = end;
        Ok(value)
    }

    fn parse_number(&mut self) -> NodiaResult<Value> {
        let start = self.index;
        if self.peek() == Some('-') {
            self.index += 1;
        }

        match self.peek() {
            Some('0') => {
                self.index += 1;
            }
            Some('1'..='9') => {
                self.index += 1;
                while matches!(self.peek(), Some('0'..='9')) {
                    self.index += 1;
                }
            }
            _ => return Err(self.error("invalid number")),
        }

        let mut is_float = false;
        if self.peek() == Some('.') {
            is_float = true;
            self.index += 1;
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.error("invalid number"));
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.index += 1;
            }
        }

        if matches!(self.peek(), Some('e' | 'E')) {
            is_float = true;
            self.index += 1;
            if matches!(self.peek(), Some('+' | '-')) {
                self.index += 1;
            }
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.error("invalid number"));
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.index += 1;
            }
        }

        let text = self.chars[start..self.index].iter().collect::<String>();
        if is_float {
            text.parse::<f64>()
                .map(Value::Float)
                .map_err(|_| self.error("invalid number"))
        } else {
            text.parse::<i64>()
                .map(Value::Int)
                .map_err(|_| self.error("invalid integer"))
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> NodiaResult<()> {
        if self
            .chars
            .get(self.index..self.index + keyword.len())
            .is_some_and(|chars| chars.iter().collect::<String>() == keyword)
        {
            self.index += keyword.len();
            Ok(())
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.index += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn error(&self, message: &str) -> NodiaError {
        let column = self.source[..self.byte_index()].chars().count() + 1;
        NodiaError::runtime(format!("invalid JSON: {message} at column {column}"))
    }

    fn byte_index(&self) -> usize {
        self.chars[..self.index]
            .iter()
            .map(|ch| ch.len_utf8())
            .sum()
    }
}
