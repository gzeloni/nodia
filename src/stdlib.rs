// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Built-in standard-library function registry used by the checker and runtime.

mod collections;
mod data;
mod datetime;
mod formatting;
mod numeric;
mod pathing;
mod sequence;
mod text;

use crate::error::{NodiaError, NodiaResult};
use crate::value::Value;
use std::collections::BTreeMap;

/// Export specification for a standard-library module item.
pub type ModuleItemSpec = (&'static str, &'static str, Option<&'static [usize]>);

const TEXT_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("upper", "upper", Some(&[1])),
    ("lower", "lower", Some(&[1])),
    ("capitalize", "capitalize", Some(&[1])),
    ("trim", "trim", Some(&[1])),
    ("replace", "replace", Some(&[3])),
    ("replace_all", "replace_all", Some(&[3])),
    ("split", "split", Some(&[2])),
    ("split_regex", "split_regex", Some(&[2])),
    ("join", "join", Some(&[2])),
    ("lines", "lines", Some(&[1])),
    ("unlines", "unlines", Some(&[1])),
    ("words", "words", Some(&[1])),
    ("contains", "contains", Some(&[2])),
    ("starts", "starts", Some(&[2])),
    ("ends", "ends", Some(&[2])),
    ("indent", "indent", Some(&[2])),
    ("dedent", "dedent", Some(&[1])),
    ("byte_len", "byte_len", Some(&[1])),
    ("byte_offset", "byte_offset", Some(&[2])),
    ("scalar_offset", "scalar_offset", Some(&[2])),
];

const NUMBERS_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("int", "int", Some(&[1])),
    ("float", "float", Some(&[1])),
    ("range", "range", Some(&[1, 2])),
    ("abs", "abs", Some(&[1])),
    ("floor", "floor", Some(&[1])),
    ("ceil", "ceil", Some(&[1])),
    ("round", "round", Some(&[1])),
    ("sqrt", "sqrt", Some(&[1])),
    ("pow", "pow", Some(&[2])),
    ("min", "min", Some(&[2])),
    ("max", "max", Some(&[2])),
    ("clamp", "clamp", Some(&[3])),
    ("sum", "sum", Some(&[1])),
    ("avg", "avg", Some(&[1])),
];

const CONVERSION_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("string", "string", Some(&[1])),
    ("bool", "bool", Some(&[1])),
    ("int", "int", Some(&[1])),
    ("float", "float", Some(&[1])),
];

const COLLECTIONS_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("len", "len", Some(&[1])),
    ("keys", "keys", Some(&[1])),
    ("values", "values", Some(&[1])),
    ("entries", "entries", Some(&[1])),
    ("contains", "contains", Some(&[2])),
    ("get", "get", Some(&[3])),
    ("push", "push", Some(&[2])),
    ("pop", "pop", Some(&[1])),
    ("first", "first", Some(&[1])),
    ("last", "last", Some(&[1])),
    ("slice", "slice", Some(&[3])),
    ("reverse", "reverse", Some(&[1])),
    ("sort", "sort", Some(&[1])),
    ("unique", "unique", Some(&[1])),
    ("map", "map", Some(&[2])),
    ("filter", "filter", Some(&[2])),
    ("reduce", "reduce", Some(&[3])),
    ("group_by", "group_by", Some(&[2])),
    ("sort_by", "sort_by", Some(&[2])),
];

const FORMAT_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("format", "format", Some(&[2])),
    ("pad_left", "pad_left", Some(&[2, 3])),
    ("pad_right", "pad_right", Some(&[2, 3])),
    ("fixed", "fixed", Some(&[2])),
];

const REGEX_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("test", "test", Some(&[2])),
    ("full_match", "full_match", Some(&[2])),
    ("find", "find", Some(&[2])),
    ("find_all", "find_all", Some(&[2])),
    ("replace", "replace", Some(&[3])),
    ("replace_all", "replace_all", Some(&[3])),
    ("split", "split", Some(&[2])),
    ("split_regex", "split_regex", Some(&[2])),
];

const IO_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("stdin", "stdin", None),
    ("stdout", "stdout", None),
    ("stderr", "stderr", None),
    ("open", "open", Some(&[2])),
    ("close", "close", Some(&[1])),
    ("flush", "flush", Some(&[1])),
    ("eof", "eof", Some(&[1])),
    ("read", "read", Some(&[1, 2])),
    ("readln", "readln", Some(&[1])),
    ("write", "write", Some(&[2])),
    ("writeln", "writeln", Some(&[2])),
    ("append", "append", Some(&[2])),
    ("basename", "basename", Some(&[1])),
    ("dirname", "dirname", Some(&[1])),
    ("exists", "exists", Some(&[1])),
    ("is_file", "is_file", Some(&[1])),
    ("is_dir", "is_dir", Some(&[1])),
    ("list_dir", "list_dir", Some(&[1])),
    ("glob", "glob", Some(&[1])),
];

const SYSTEM_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("args", "args", None),
    ("env", "env", Some(&[1, 2])),
    ("exit", "exit", Some(&[0, 1])),
    ("exec", "exec", Some(&[1, 2])),
];

const DATETIME_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("now", "now", Some(&[0, 1])),
    ("today", "today", Some(&[0, 1])),
    ("date", "date", Some(&[1, 3])),
    ("datetime", "datetime", Some(&[1, 6, 7])),
    ("duration", "duration", Some(&[1])),
    ("parse_date", "parse_date", Some(&[1])),
    ("parse_datetime", "parse_datetime", Some(&[1])),
    ("parse_duration", "parse_duration", Some(&[1])),
    ("isoformat", "isoformat", Some(&[1])),
    ("strftime", "strftime", Some(&[2])),
    ("from_unix", "from_unix", Some(&[1, 2])),
    ("from_unix_ms", "from_unix_ms", Some(&[1])),
    ("unix_seconds", "unix_seconds", Some(&[1])),
    ("unix_ms", "unix_ms", Some(&[1])),
    ("year", "year", Some(&[1])),
    ("month", "month", Some(&[1])),
    ("day", "day", Some(&[1])),
    ("hour", "hour", Some(&[1])),
    ("minute", "minute", Some(&[1])),
    ("second", "second", Some(&[1])),
    ("nanosecond", "nanosecond", Some(&[1])),
    ("weekday", "weekday", Some(&[1])),
    ("weekday_name", "weekday_name", Some(&[1])),
    ("month_name", "month_name", Some(&[1])),
    ("ordinal_day", "ordinal_day", Some(&[1])),
    ("iso_week", "iso_week", Some(&[1])),
    ("offset_minutes", "offset_minutes", Some(&[1])),
    ("days_in_month", "days_in_month", Some(&[1, 2])),
    ("is_leap_year", "is_leap_year", Some(&[1])),
    ("date_only", "date_only", Some(&[1])),
    ("with_offset", "with_offset", Some(&[2])),
    ("add_days", "add_days", Some(&[2])),
    ("add_months", "add_months", Some(&[2])),
    ("add_years", "add_years", Some(&[2])),
    ("add_duration", "add_duration", Some(&[2])),
    ("diff_days", "diff_days", Some(&[2])),
    ("diff_seconds", "diff_seconds", Some(&[2])),
    ("diff_duration", "diff_duration", Some(&[2])),
    ("start_of_day", "start_of_day", Some(&[1])),
    ("end_of_day", "end_of_day", Some(&[1])),
];

const JSON_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("read", "json.read", Some(&[1])),
    ("write", "json.write", Some(&[1, 2])),
];

const CSV_MODULE_ITEMS: &[ModuleItemSpec] = &[
    ("read", "csv.read", Some(&[1, 2])),
    ("write", "csv.write", Some(&[1])),
];

pub fn call(name: &str, args: &[Value]) -> NodiaResult<Option<Value>> {
    let result = match name {
        "upper" => text::unary_string(args, name, |s| s.to_uppercase())?,
        "lower" => text::unary_string(args, name, |s| s.to_lowercase())?,
        "capitalize" => text::unary_string(args, "capitalize", |s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })?,
        "trim" => text::unary_string(args, "trim", |s| s.trim().to_string())?,
        "replace" | "replace_all" => text::replace_text(args, name)?,
        "split" | "split_regex" => text::split_text(args, name)?,
        "join" => {
            expect_arity(&args, 2, "join")?;
            let values = expect_list(&args[0], "join", "first")?;
            Value::String(
                values
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join(&args[1].to_string()),
            )
        }
        "lines" => {
            expect_arity(&args, 1, "lines")?;
            Value::List(
                args[0]
                    .to_string()
                    .lines()
                    .map(|line| Value::String(line.to_string()))
                    .collect(),
            )
        }
        "unlines" => {
            expect_arity(&args, 1, "unlines")?;
            let values = expect_list(&args[0], "unlines", "first")?;
            Value::String(
                values
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        }
        "words" => {
            expect_arity(&args, 1, "words")?;
            Value::List(
                args[0]
                    .to_string()
                    .split_whitespace()
                    .map(|word| Value::String(word.to_string()))
                    .collect(),
            )
        }
        "test" => text::regex_test(args)?,
        "full_match" => text::regex_full_match(args)?,
        "find" => text::regex_find(args)?,
        "find_all" => text::regex_find_all(args)?,
        "contains" => {
            expect_arity(&args, 2, "contains")?;
            Value::Bool(match &args[0] {
                Value::String(value) => text::contains_text(value, &args[1])?,
                Value::List(values) => values.contains(&args[1]),
                Value::Map(values) => values.contains_key(&args[1].to_string()),
                other => {
                    return Err(NodiaError::runtime(format!(
                        "contains() does not accept {}",
                        other.type_name()
                    )));
                }
            })
        }
        "starts" => {
            expect_arity(&args, 2, name)?;
            Value::Bool(text::text_starts_with(&args[0].to_string(), &args[1])?)
        }
        "ends" => {
            expect_arity(&args, 2, name)?;
            Value::Bool(text::text_ends_with(&args[0].to_string(), &args[1])?)
        }
        "indent" => text::indent(args)?,
        "dedent" => {
            expect_arity(&args, 1, "dedent")?;
            Value::String(text::dedent(&args[0].to_string()))
        }
        "byte_len" => text::byte_len(args)?,
        "byte_offset" => text::byte_offset(args)?,
        "scalar_offset" => text::scalar_offset(args)?,
        "keys" => {
            expect_arity(&args, 1, "keys")?;
            let Value::Map(values) = &args[0] else {
                return Err(NodiaError::runtime(format!(
                    "keys() expects map, got {}",
                    args[0].type_name()
                )));
            };
            Value::List(values.keys().cloned().map(Value::String).collect())
        }
        "values" => {
            expect_arity(&args, 1, "values")?;
            let Value::Map(values) = &args[0] else {
                return Err(NodiaError::runtime(format!(
                    "values() expects map, got {}",
                    args[0].type_name()
                )));
            };
            Value::List(values.values().cloned().collect())
        }
        "entries" => {
            expect_arity(&args, 1, "entries")?;
            let Value::Map(values) = &args[0] else {
                return Err(NodiaError::runtime(format!(
                    "entries() expects map, got {}",
                    args[0].type_name()
                )));
            };
            Value::List(
                values
                    .iter()
                    .map(|(key, value)| {
                        let mut entry = BTreeMap::new();
                        entry.insert("key".to_string(), Value::String(key.clone()));
                        entry.insert("value".to_string(), value.clone());
                        Value::Map(entry)
                    })
                    .collect(),
            )
        }
        "get" => collections::get(args)?,
        "len" => {
            expect_arity(&args, 1, "len")?;
            let len = match &args[0] {
                Value::String(value) => value.chars().count(),
                Value::List(value) => value.len(),
                Value::Map(value) => value.len(),
                other => {
                    return Err(NodiaError::runtime(format!(
                        "len() does not accept {}",
                        other.type_name()
                    )));
                }
            };
            Value::Int(len as i64)
        }
        "int" => {
            expect_arity(&args, 1, "int")?;
            Value::Int(to_int(&args[0])?)
        }
        "float" => {
            expect_arity(&args, 1, "float")?;
            Value::Float(to_float(&args[0])?)
        }
        "string" => {
            expect_arity(&args, 1, "string")?;
            Value::String(args[0].to_string())
        }
        "format" => formatting::format(args)?,
        "pad_left" => formatting::pad_left(args)?,
        "pad_right" => formatting::pad_right(args)?,
        "fixed" => formatting::fixed(args)?,
        "basename" => pathing::basename(args)?,
        "dirname" => pathing::dirname(args)?,
        "exists" => pathing::exists(args)?,
        "is_file" => pathing::is_file(args)?,
        "is_dir" => pathing::is_dir(args)?,
        "list_dir" => pathing::list_dir(args)?,
        "glob" => pathing::glob(args)?,
        "now" => datetime::now(args)?,
        "today" => datetime::today(args)?,
        "date" => datetime::date(args)?,
        "datetime" => datetime::datetime(args)?,
        "duration" => datetime::duration(args)?,
        "parse_date" => datetime::parse_date(args)?,
        "parse_datetime" => datetime::parse_datetime(args)?,
        "parse_duration" => datetime::parse_duration(args)?,
        "isoformat" => datetime::isoformat(args)?,
        "strftime" => datetime::strftime(args)?,
        "from_unix" => datetime::from_unix(args)?,
        "from_unix_ms" => datetime::from_unix_ms(args)?,
        "unix_seconds" => datetime::unix_seconds(args)?,
        "unix_ms" => datetime::unix_ms(args)?,
        "year" => datetime::year(args)?,
        "month" => datetime::month(args)?,
        "day" => datetime::day(args)?,
        "hour" => datetime::hour(args)?,
        "minute" => datetime::minute(args)?,
        "second" => datetime::second(args)?,
        "nanosecond" => datetime::nanosecond(args)?,
        "weekday" => datetime::weekday(args)?,
        "weekday_name" => datetime::weekday_name(args)?,
        "month_name" => datetime::month_name(args)?,
        "ordinal_day" => datetime::ordinal_day(args)?,
        "iso_week" => datetime::iso_week(args)?,
        "offset_minutes" => datetime::offset_minutes(args)?,
        "days_in_month" => datetime::days_in_month_value(args)?,
        "is_leap_year" => datetime::is_leap_year_value(args)?,
        "date_only" => datetime::date_only(args)?,
        "with_offset" => datetime::with_offset(args)?,
        "add_days" => datetime::add_days(args)?,
        "add_months" => datetime::add_months(args)?,
        "add_years" => datetime::add_years(args)?,
        "add_duration" => datetime::add_duration(args)?,
        "diff_days" => datetime::diff_days(args)?,
        "diff_seconds" => datetime::diff_seconds(args)?,
        "diff_duration" => datetime::diff_duration(args)?,
        "start_of_day" => datetime::start_of_day(args)?,
        "end_of_day" => datetime::end_of_day(args)?,
        "json.read" => data::json_read(args)?,
        "json.write" => data::json_write(args)?,
        "csv.read" => data::csv_read(args)?,
        "csv.write" => data::csv_write(args)?,
        "bool" => {
            expect_arity(&args, 1, "bool")?;
            Value::Bool(args[0].truthy())
        }
        "range" => numeric::range(args)?,
        "abs" => numeric::abs(args)?,
        "floor" => numeric::rounded(args, "floor", f64::floor)?,
        "ceil" => numeric::rounded(args, "ceil", f64::ceil)?,
        "round" => numeric::rounded(args, "round", f64::round)?,
        "sqrt" => {
            expect_arity(&args, 1, "sqrt")?;
            Value::Float(to_float(&args[0])?.sqrt())
        }
        "pow" => {
            expect_arity(&args, 2, "pow")?;
            sequence::number_result(to_float(&args[0])?.powf(to_float(&args[1])?), &args)
        }
        "min" => {
            expect_arity(&args, 2, "min")?;
            let a = to_float(&args[0])?;
            let b = to_float(&args[1])?;
            sequence::number_result(a.min(b), &args)
        }
        "max" => {
            expect_arity(&args, 2, "max")?;
            let a = to_float(&args[0])?;
            let b = to_float(&args[1])?;
            sequence::number_result(a.max(b), &args)
        }
        "clamp" => {
            expect_arity(&args, 3, "clamp")?;
            let value = to_float(&args[0])?;
            let min = to_float(&args[1])?;
            let max = to_float(&args[2])?;
            if min > max {
                return Err(NodiaError::runtime(
                    "clamp() min cannot be greater than max",
                ));
            }
            sequence::number_result(value.clamp(min, max), &args)
        }
        "sum" => numeric::sum(args)?,
        "avg" => numeric::avg(args)?,
        "push" => {
            expect_arity(&args, 2, "push")?;
            let mut values = expect_list(&args[0], "push", "first")?.clone();
            values.push(args[1].clone());
            Value::List(values)
        }
        "pop" => {
            expect_arity(&args, 1, "pop")?;
            let mut values = expect_list(&args[0], "pop", "first")?.clone();
            values.pop();
            Value::List(values)
        }
        "first" => {
            expect_arity(&args, 1, "first")?;
            expect_list(&args[0], "first", "first")?
                .first()
                .cloned()
                .unwrap_or(Value::Null)
        }
        "last" => {
            expect_arity(&args, 1, "last")?;
            expect_list(&args[0], "last", "first")?
                .last()
                .cloned()
                .unwrap_or(Value::Null)
        }
        "slice" => sequence::slice(args)?,
        "reverse" => sequence::reverse(args)?,
        "sort" => sequence::sort(args)?,
        "unique" => sequence::unique(args)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

/// Returns the exported items for a standard-library module.
pub fn module_items(name: &str) -> Option<&'static [ModuleItemSpec]> {
    match name {
        "text" => Some(TEXT_MODULE_ITEMS),
        "numbers" => Some(NUMBERS_MODULE_ITEMS),
        "conversion" => Some(CONVERSION_MODULE_ITEMS),
        "collections" => Some(COLLECTIONS_MODULE_ITEMS),
        "format" => Some(FORMAT_MODULE_ITEMS),
        "re" => Some(REGEX_MODULE_ITEMS),
        "io" => Some(IO_MODULE_ITEMS),
        "system" => Some(SYSTEM_MODULE_ITEMS),
        "datetime" => Some(DATETIME_MODULE_ITEMS),
        "json" => Some(JSON_MODULE_ITEMS),
        "csv" => Some(CSV_MODULE_ITEMS),
        _ => None,
    }
}

pub(crate) fn expect_arity(args: &[Value], expected: usize, name: &str) -> NodiaResult<()> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(NodiaError::runtime(format!(
            "{name}() expects {expected} argument(s), got {}",
            args.len()
        )))
    }
}

pub(crate) fn expect_list<'a>(
    value: &'a Value,
    name: &str,
    position: &str,
) -> NodiaResult<&'a Vec<Value>> {
    let Value::List(values) = value else {
        return Err(NodiaError::runtime(format!(
            "{name}() expects list as {position} argument, got {}",
            value.type_name()
        )));
    };
    Ok(values)
}

pub(crate) fn to_int(value: &Value) -> NodiaResult<i64> {
    match value {
        Value::Int(value) => Ok(*value),
        Value::Float(value) => Ok(*value as i64),
        Value::String(value) => value
            .parse::<i64>()
            .map_err(|_| NodiaError::runtime(format!("cannot convert '{value}' to int"))),
        other => Err(NodiaError::runtime(format!(
            "cannot convert {} to int",
            other.type_name()
        ))),
    }
}

pub(crate) fn to_float(value: &Value) -> NodiaResult<f64> {
    match value {
        Value::Int(value) => Ok(*value as f64),
        Value::Float(value) => Ok(*value),
        Value::String(value) => value
            .parse::<f64>()
            .map_err(|_| NodiaError::runtime(format!("cannot convert '{value}' to float"))),
        other => Err(NodiaError::runtime(format!(
            "cannot convert {} to float",
            other.type_name()
        ))),
    }
}

pub(crate) fn compare_values(left: &Value, right: &Value) -> std::cmp::Ordering {
    sequence::compare_values(left, right)
}
