// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Datetime-oriented standard-library functions backed by [`crate::temporal`].

use super::expect_arity;
use super::result;
use crate::error::{NodiaError, NodiaResult};
use crate::temporal::{
    days_in_month, is_leap_year, parse_offset_text, DateTimeValue, DateValue, DurationValue,
    ValueNumber,
};
use crate::value::Value;
use std::collections::BTreeMap;

const SECONDS_PER_DAY: i64 = 86_400;
const NANOS_PER_SECOND: i128 = 1_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParseKind {
    Date,
    DateTime,
    Duration,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EpochUnit {
    Seconds,
    Milliseconds,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AddUnit {
    Days,
    Months,
    Years,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DiffUnit {
    Days,
    Seconds,
    Span,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoundSide {
    Start,
    End,
}

pub fn now(args: &[Value]) -> NodiaResult<Value> {
    if args.len() > 1 {
        return Err(NodiaError::runtime(format!(
            "now() expects 0 or 1 argument(s), got {}",
            args.len()
        )));
    }
    let offset = args
        .first()
        .map(|value| expect_offset(value, "now", "first"))
        .transpose()?
        .unwrap_or(0);
    Ok(Value::DateTime(DateTimeValue::now(offset)?))
}

pub fn today(args: &[Value]) -> NodiaResult<Value> {
    if args.len() > 1 {
        return Err(NodiaError::runtime(format!(
            "today() expects 0 or 1 argument(s), got {}",
            args.len()
        )));
    }
    let offset = args
        .first()
        .map(|value| expect_offset(value, "today", "first"))
        .transpose()?
        .unwrap_or(0);
    Ok(Value::Date(DateValue::today(offset)?))
}

pub fn date(args: &[Value]) -> NodiaResult<Value> {
    let value = match args {
        [Value::Map(fields)] => build_date_from_map(fields, "date")?,
        [year, month, day] => DateValue::new(
            expect_i32(year, "date", "first")?,
            expect_u8(month, "date", "second")?,
            expect_u8(day, "date", "third")?,
        )?,
        _ => {
            return Err(NodiaError::runtime(format!(
                "date() expects 1 or 3 argument(s), got {}",
                args.len()
            )));
        }
    };
    Ok(Value::Date(value))
}

pub fn datetime(args: &[Value]) -> NodiaResult<Value> {
    let value = match args {
        [Value::Map(fields)] => build_datetime_from_map(fields, "datetime")?,
        [year, month, day, hour, minute, second] => DateTimeValue::new(
            expect_i32(year, "datetime", "first")?,
            expect_u8(month, "datetime", "second")?,
            expect_u8(day, "datetime", "third")?,
            expect_u8(hour, "datetime", "fourth")?,
            expect_u8(minute, "datetime", "fifth")?,
            expect_u8(second, "datetime", "sixth")?,
            0,
            0,
        )?,
        [year, month, day, hour, minute, second, options] => {
            let (nanosecond, offset_minutes) = datetime_options(options, "datetime", "seventh")?;
            DateTimeValue::new(
                expect_i32(year, "datetime", "first")?,
                expect_u8(month, "datetime", "second")?,
                expect_u8(day, "datetime", "third")?,
                expect_u8(hour, "datetime", "fourth")?,
                expect_u8(minute, "datetime", "fifth")?,
                expect_u8(second, "datetime", "sixth")?,
                nanosecond,
                offset_minutes,
            )?
        }
        _ => {
            return Err(NodiaError::runtime(format!(
                "datetime() expects 1, 6 or 7 argument(s), got {}",
                args.len()
            )));
        }
    };
    Ok(Value::DateTime(value))
}

pub fn duration(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "duration")?;
    let Value::Map(fields) = &args[0] else {
        return Err(NodiaError::runtime(format!(
            "duration() expects map as first argument, got {}",
            args[0].type_name()
        )));
    };

    if fields.is_empty() {
        return Err(NodiaError::runtime(
            "duration() expects at least one duration component",
        ));
    }

    let weeks = optional_i64(fields, "weeks", "duration")?.unwrap_or(0);
    let days = optional_i64(fields, "days", "duration")?.unwrap_or(0);
    let hours = optional_i64(fields, "hours", "duration")?.unwrap_or(0);
    let minutes = optional_i64(fields, "minutes", "duration")?.unwrap_or(0);
    let milliseconds = optional_i64(fields, "milliseconds", "duration")?.unwrap_or(0);
    let microseconds = optional_i64(fields, "microseconds", "duration")?.unwrap_or(0);
    let nanoseconds = optional_i64(fields, "nanoseconds", "duration")?.unwrap_or(0);

    let (seconds, extra_nanos) = match fields.get("seconds") {
        Some(Value::Int(value)) => (*value, 0),
        Some(Value::Float(value)) => split_float_seconds(*value)?,
        Some(other) => {
            return Err(NodiaError::runtime(format!(
                "duration() expects int or float as option 'seconds', got {}",
                other.type_name()
            )));
        }
        None => (0, 0),
    };

    Ok(Value::Duration(DurationValue::from_parts(
        weeks,
        days,
        hours,
        minutes,
        seconds,
        milliseconds,
        microseconds,
        nanoseconds + extra_nanos,
    )?))
}

pub fn parse(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "parse")?;
    let text = expect_string(&args[0], "parse", "first")?;
    let kind = expect_parse_kind(&args[1], "parse", "second")?;
    let outcome = match kind {
        ParseKind::Date => DateValue::parse_iso(&text).map(Value::Date),
        ParseKind::DateTime => DateTimeValue::parse_iso(&text).map(Value::DateTime),
        ParseKind::Duration => DurationValue::parse_iso(&text).map(Value::Duration),
    };
    Ok(result::capture_outcome_in_context(
        "datetime.parse",
        outcome,
    ))
}

pub fn isoformat(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "isoformat")?;
    Ok(Value::String(match &args[0] {
        Value::Date(value) => value.isoformat(),
        Value::DateTime(value) => value.isoformat(),
        Value::Duration(value) => value.isoformat(),
        other => {
            return Err(NodiaError::runtime(format!(
                "isoformat() expects date, datetime or duration, got {}",
                other.type_name()
            )));
        }
    }))
}

pub fn strftime(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "strftime")?;
    let pattern = expect_string(&args[1], "strftime", "second")?;
    let text = match &args[0] {
        Value::Date(value) => value.strftime(&pattern)?,
        Value::DateTime(value) => value.strftime(&pattern)?,
        other => {
            return Err(NodiaError::runtime(format!(
                "strftime() expects date or datetime as first argument, got {}",
                other.type_name()
            )));
        }
    };
    Ok(Value::String(text))
}

pub fn from_epoch(args: &[Value]) -> NodiaResult<Value> {
    if args.len() != 2 && args.len() != 3 {
        return Err(NodiaError::runtime(format!(
            "from_epoch() expects 2 or 3 argument(s), got {}",
            args.len()
        )));
    }
    let unit = expect_epoch_unit(&args[1], "from_epoch", "second")?;
    let offset = args
        .get(2)
        .map(|value| expect_offset(value, "from_epoch", "third"))
        .transpose()?
        .unwrap_or(0);
    Ok(Value::DateTime(match unit {
        EpochUnit::Seconds => {
            let (seconds, nanosecond) = expect_unix_seconds_value(&args[0], "from_epoch", "first")?;
            DateTimeValue::from_unix_seconds(seconds, nanosecond, offset)?
        }
        EpochUnit::Milliseconds => DateTimeValue::from_unix_milliseconds(
            expect_unix_ms(&args[0], "from_epoch", "first")?,
            offset,
        )?,
    }))
}

pub fn epoch(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "epoch")?;
    let unit = expect_epoch_unit(&args[1], "epoch", "second")?;
    let value = expect_datetime(&args[0], "epoch", "first")?;
    Ok(value_number(match unit {
        EpochUnit::Seconds => value.unix_seconds(),
        EpochUnit::Milliseconds => value.unix_milliseconds(),
    }))
}

pub fn year(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "year")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "year", "first")?.year() as i64,
    ))
}

pub fn month(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "month")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "month", "first")?.month() as i64,
    ))
}

pub fn day(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "day")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "day", "first")?.day() as i64,
    ))
}

pub fn hour(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "hour")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "hour", "first")?.hour() as i64,
    ))
}

pub fn minute(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "minute")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "minute", "first")?.minute() as i64,
    ))
}

pub fn second(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "second")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "second", "first")?.second() as i64,
    ))
}

pub fn nanosecond(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "nanosecond")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "nanosecond", "first")?.nanosecond() as i64,
    ))
}

pub fn weekday(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "weekday")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "weekday", "first")?.weekday() as i64,
    ))
}

pub fn weekday_name(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "weekday_name")?;
    Ok(Value::String(
        expect_date_like(&args[0], "weekday_name", "first")?
            .weekday_long_name()
            .to_string(),
    ))
}

pub fn month_name(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "month_name")?;
    Ok(Value::String(
        expect_date_like(&args[0], "month_name", "first")?
            .month_long_name()
            .to_string(),
    ))
}

pub fn ordinal_day(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "ordinal_day")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "ordinal_day", "first")?.ordinal_day() as i64,
    ))
}

pub fn iso_week(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "iso_week")?;
    let (year, week) = expect_date_like(&args[0], "iso_week", "first")?.iso_week();
    let mut out = BTreeMap::new();
    out.insert("year".to_string(), Value::Int(year as i64));
    out.insert("week".to_string(), Value::Int(week as i64));
    Ok(Value::Map(out))
}

pub fn offset_minutes(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "offset_minutes")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "offset_minutes", "first")?.offset_minutes() as i64,
    ))
}

pub fn days_in_month_value(args: &[Value]) -> NodiaResult<Value> {
    let result = match args {
        [Value::Date(value)] => value.days_in_month() as i64,
        [Value::DateTime(value)] => value.days_in_month() as i64,
        [value] => {
            return Err(NodiaError::runtime(format!(
                "days_in_month() expects date/datetime or year/month, got {}",
                value.type_name()
            )));
        }
        [year, month] => days_in_month(
            expect_i32(year, "days_in_month", "first")?,
            expect_u8(month, "days_in_month", "second")?,
        ) as i64,
        _ => {
            return Err(NodiaError::runtime(format!(
                "days_in_month() expects 1 or 2 argument(s), got {}",
                args.len()
            )));
        }
    };
    Ok(Value::Int(result))
}

pub fn is_leap_year_value(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "is_leap_year")?;
    let result = match &args[0] {
        Value::Int(year) => is_leap_year(i32::try_from(*year).map_err(|_| {
            NodiaError::runtime("is_leap_year() expects int within i32 range as first argument")
        })?),
        Value::Date(value) => value.is_leap_year(),
        Value::DateTime(value) => value.is_leap_year(),
        other => {
            return Err(NodiaError::runtime(format!(
                "is_leap_year() expects int, date or datetime, got {}",
                other.type_name()
            )));
        }
    };
    Ok(Value::Bool(result))
}

pub fn date_only(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 1, "date_only")?;
    Ok(Value::Date(expect_date_like(
        &args[0],
        "date_only",
        "first",
    )?))
}

pub fn with_offset(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "with_offset")?;
    Ok(Value::DateTime(
        expect_datetime(&args[0], "with_offset", "first")?.with_offset(expect_offset(
            &args[1],
            "with_offset",
            "second",
        )?)?,
    ))
}

pub fn add(args: &[Value]) -> NodiaResult<Value> {
    match args {
        [value, delta] => add_duration_value(value, delta, "add"),
        [value, amount, unit] => {
            let unit = expect_add_unit(unit, "add", "third")?;
            match unit {
                AddUnit::Days => add_days_value(value, expect_i64(amount, "add", "second")?, "add"),
                AddUnit::Months => {
                    add_months_value(value, expect_i32(amount, "add", "second")?, "add")
                }
                AddUnit::Years => {
                    add_years_value(value, expect_i32(amount, "add", "second")?, "add")
                }
            }
        }
        _ => Err(NodiaError::runtime(format!(
            "add() expects 2 or 3 argument(s), got {}",
            args.len()
        ))),
    }
}

pub fn diff(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 3, "diff")?;
    let unit = expect_diff_unit(&args[2], "diff", "third")?;
    match unit {
        DiffUnit::Days => {
            let left = expect_date_like(&args[0], "diff", "first")?;
            let right = expect_date_like(&args[1], "diff", "second")?;
            Ok(Value::Int(
                (left.days_since_epoch() - right.days_since_epoch()) as i64,
            ))
        }
        DiffUnit::Seconds => Ok(value_number(diff_seconds_number(
            &args[0], &args[1], "diff",
        )?)),
        DiffUnit::Span => Ok(Value::Duration(diff_span_value(
            &args[0], &args[1], "diff",
        )?)),
    }
}

pub fn bound(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(args, 2, "bound")?;
    let side = expect_bound_side(&args[1], "bound", "second")?;
    match side {
        BoundSide::Start => bound_start_value(&args[0], "bound"),
        BoundSide::End => bound_end_value(&args[0], "bound"),
    }
}

fn add_days_value(value: &Value, amount: i64, name: &str) -> NodiaResult<Value> {
    match value {
        Value::Date(value) => Ok(Value::Date(value.add_days(amount)?)),
        Value::DateTime(value) => Ok(Value::DateTime(value.add_days(amount)?)),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects date or datetime as first argument, got {}",
            other.type_name()
        ))),
    }
}

fn add_months_value(value: &Value, amount: i32, name: &str) -> NodiaResult<Value> {
    match value {
        Value::Date(value) => Ok(Value::Date(value.add_months(amount)?)),
        Value::DateTime(value) => Ok(Value::DateTime(value.add_months(amount)?)),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects date or datetime as first argument, got {}",
            other.type_name()
        ))),
    }
}

fn add_years_value(value: &Value, amount: i32, name: &str) -> NodiaResult<Value> {
    match value {
        Value::Date(value) => Ok(Value::Date(value.add_years(amount)?)),
        Value::DateTime(value) => Ok(Value::DateTime(value.add_years(amount)?)),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects date or datetime as first argument, got {}",
            other.type_name()
        ))),
    }
}

fn add_duration_value(value: &Value, delta: &Value, name: &str) -> NodiaResult<Value> {
    let duration = expect_duration(delta, name, "second")?;
    match value {
        Value::DateTime(value) => Ok(Value::DateTime(value.add_duration(duration)?)),
        Value::Duration(value) => Ok(Value::Duration(DurationValue::from_total_nanoseconds(
            value.total_nanoseconds() + duration.total_nanoseconds(),
        ))),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects datetime or duration as first argument, got {}",
            other.type_name()
        ))),
    }
}

fn diff_seconds_number(left: &Value, right: &Value, name: &str) -> NodiaResult<ValueNumber> {
    match (left, right) {
        (Value::Date(left), Value::Date(right)) => Ok(duration_seconds_number(
            DurationValue::from_total_nanoseconds(
                (left.days_since_epoch() as i128 - right.days_since_epoch() as i128)
                    * SECONDS_PER_DAY as i128
                    * NANOS_PER_SECOND,
            ),
        )),
        (Value::DateTime(left), Value::DateTime(right)) => {
            Ok(duration_seconds_number(left.diff(*right)))
        }
        (Value::Duration(left), Value::Duration(right)) => Ok(duration_seconds_number(
            DurationValue::from_total_nanoseconds(
                left.total_nanoseconds() - right.total_nanoseconds(),
            ),
        )),
        (left, right) => Err(NodiaError::runtime(format!(
            "{name}() expects date/date, datetime/datetime or duration/duration for seconds, got {}/{}",
            left.type_name(),
            right.type_name()
        ))),
    }
}

fn diff_span_value(left: &Value, right: &Value, name: &str) -> NodiaResult<DurationValue> {
    match (left, right) {
        (Value::Date(left), Value::Date(right)) => Ok(DurationValue::from_total_nanoseconds(
            (left.days_since_epoch() as i128 - right.days_since_epoch() as i128)
                * SECONDS_PER_DAY as i128
                * NANOS_PER_SECOND,
        )),
        (Value::DateTime(left), Value::DateTime(right)) => Ok(left.diff(*right)),
        (Value::Duration(left), Value::Duration(right)) => {
            Ok(DurationValue::from_total_nanoseconds(
                left.total_nanoseconds() - right.total_nanoseconds(),
            ))
        }
        (left, right) => Err(NodiaError::runtime(format!(
            "{name}() expects matching date, datetime or duration values for span, got {}/{}",
            left.type_name(),
            right.type_name()
        ))),
    }
}

fn bound_start_value(value: &Value, name: &str) -> NodiaResult<Value> {
    match value {
        Value::Date(value) => Ok(Value::DateTime(DateTimeValue::new(
            value.year(),
            value.month(),
            value.day(),
            0,
            0,
            0,
            0,
            0,
        )?)),
        Value::DateTime(value) => Ok(Value::DateTime(value.start_of_day())),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects date or datetime as first argument, got {}",
            other.type_name()
        ))),
    }
}

fn bound_end_value(value: &Value, name: &str) -> NodiaResult<Value> {
    match value {
        Value::Date(value) => Ok(Value::DateTime(DateTimeValue::new(
            value.year(),
            value.month(),
            value.day(),
            23,
            59,
            59,
            999_999_999,
            0,
        )?)),
        Value::DateTime(value) => Ok(Value::DateTime(value.end_of_day())),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects date or datetime as first argument, got {}",
            other.type_name()
        ))),
    }
}

fn build_date_from_map(fields: &BTreeMap<String, Value>, name: &str) -> NodiaResult<DateValue> {
    DateValue::new(
        required_i32(fields, "year", name)?,
        required_u8(fields, "month", name)?,
        required_u8(fields, "day", name)?,
    )
}

fn build_datetime_from_map(
    fields: &BTreeMap<String, Value>,
    name: &str,
) -> NodiaResult<DateTimeValue> {
    let nanosecond = optional_u32(fields, "nanosecond", name)?.unwrap_or(0);
    let offset_minutes = optional_offset(fields, "offset", name)?.unwrap_or(0);
    DateTimeValue::new(
        required_i32(fields, "year", name)?,
        required_u8(fields, "month", name)?,
        required_u8(fields, "day", name)?,
        required_u8(fields, "hour", name)?,
        required_u8(fields, "minute", name)?,
        required_u8(fields, "second", name)?,
        nanosecond,
        offset_minutes,
    )
}

fn datetime_options(value: &Value, name: &str, position: &str) -> NodiaResult<(u32, i32)> {
    match value {
        Value::Map(fields) => Ok((
            optional_u32(fields, "nanosecond", name)?.unwrap_or(0),
            optional_offset(fields, "offset", name)?.unwrap_or(0),
        )),
        Value::Int(_) | Value::String(_) => Ok((0, expect_offset(value, name, position)?)),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects offset or map as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_date_like(value: &Value, name: &str, position: &str) -> NodiaResult<DateValue> {
    match value {
        Value::Date(value) => Ok(*value),
        Value::DateTime(value) => Ok(value.date()),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects date or datetime as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_datetime(value: &Value, name: &str, position: &str) -> NodiaResult<DateTimeValue> {
    match value {
        Value::DateTime(value) => Ok(*value),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects datetime as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_duration(value: &Value, name: &str, position: &str) -> NodiaResult<DurationValue> {
    match value {
        Value::Duration(value) => Ok(*value),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects duration as {position} argument, got {}",
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

fn expect_named_string<'a>(
    value: &'a Value,
    name: &str,
    position: &str,
    kind: &str,
) -> NodiaResult<&'a str> {
    match value {
        Value::String(value) => Ok(value),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects {kind} as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_parse_kind(value: &Value, name: &str, position: &str) -> NodiaResult<ParseKind> {
    match expect_named_string(value, name, position, "parse kind")? {
        "date" => Ok(ParseKind::Date),
        "datetime" => Ok(ParseKind::DateTime),
        "duration" => Ok(ParseKind::Duration),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects as_date, as_datetime, or as_duration as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_epoch_unit(value: &Value, name: &str, position: &str) -> NodiaResult<EpochUnit> {
    match expect_named_string(value, name, position, "epoch unit")? {
        "seconds" => Ok(EpochUnit::Seconds),
        "milliseconds" => Ok(EpochUnit::Milliseconds),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects seconds or milliseconds as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_add_unit(value: &Value, name: &str, position: &str) -> NodiaResult<AddUnit> {
    match expect_named_string(value, name, position, "add unit")? {
        "days" => Ok(AddUnit::Days),
        "months" => Ok(AddUnit::Months),
        "years" => Ok(AddUnit::Years),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects days, months, or years as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_diff_unit(value: &Value, name: &str, position: &str) -> NodiaResult<DiffUnit> {
    match expect_named_string(value, name, position, "diff unit")? {
        "days" => Ok(DiffUnit::Days),
        "seconds" => Ok(DiffUnit::Seconds),
        "span" => Ok(DiffUnit::Span),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects days, seconds, or span as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_bound_side(value: &Value, name: &str, position: &str) -> NodiaResult<BoundSide> {
    match expect_named_string(value, name, position, "boundary side")? {
        "start" => Ok(BoundSide::Start),
        "end" => Ok(BoundSide::End),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects start or end as {position} argument, got '{other}'"
        ))),
    }
}

fn expect_i64(value: &Value, name: &str, position: &str) -> NodiaResult<i64> {
    match value {
        Value::Int(value) => Ok(*value),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects int as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_i32(value: &Value, name: &str, position: &str) -> NodiaResult<i32> {
    i32::try_from(expect_i64(value, name, position)?).map_err(|_| {
        NodiaError::runtime(format!(
            "{name}() int in {position} argument is outside i32 range"
        ))
    })
}

fn expect_u8(value: &Value, name: &str, position: &str) -> NodiaResult<u8> {
    u8::try_from(expect_i64(value, name, position)?).map_err(|_| {
        NodiaError::runtime(format!(
            "{name}() int in {position} argument is outside u8 range"
        ))
    })
}

fn expect_u32(value: &Value, name: &str, position: &str) -> NodiaResult<u32> {
    u32::try_from(expect_i64(value, name, position)?).map_err(|_| {
        NodiaError::runtime(format!(
            "{name}() int in {position} argument is outside u32 range"
        ))
    })
}

fn expect_offset(value: &Value, name: &str, position: &str) -> NodiaResult<i32> {
    match value {
        Value::Int(value) => i32::try_from(*value).map_err(|_| {
            NodiaError::runtime(format!(
                "{name}() expects offset within i32 range as {position} argument"
            ))
        }),
        Value::String(value) => parse_offset_text(value),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects int or string offset as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_unix_seconds_value(value: &Value, name: &str, position: &str) -> NodiaResult<(i64, u32)> {
    match value {
        Value::Int(value) => Ok((*value, 0)),
        Value::Float(value) => float_to_seconds_and_nanos(*value, name, position),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects int or float as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_unix_ms(value: &Value, name: &str, position: &str) -> NodiaResult<i128> {
    match value {
        Value::Int(value) => Ok(*value as i128),
        Value::Float(value) => {
            if !value.is_finite() {
                return Err(NodiaError::runtime(format!(
                    "{name}() expects finite number as {position} argument"
                )));
            }
            Ok(value.round() as i128)
        }
        other => Err(NodiaError::runtime(format!(
            "{name}() expects int or float as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn required_i32(fields: &BTreeMap<String, Value>, key: &str, name: &str) -> NodiaResult<i32> {
    let value = fields
        .get(key)
        .ok_or_else(|| NodiaError::runtime(format!("{name}() missing required option '{key}'")))?;
    expect_i32(value, name, &format!("option '{key}'"))
}

fn required_u8(fields: &BTreeMap<String, Value>, key: &str, name: &str) -> NodiaResult<u8> {
    let value = fields
        .get(key)
        .ok_or_else(|| NodiaError::runtime(format!("{name}() missing required option '{key}'")))?;
    expect_u8(value, name, &format!("option '{key}'"))
}

fn optional_u32(
    fields: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> NodiaResult<Option<u32>> {
    fields
        .get(key)
        .map(|value| expect_u32(value, name, &format!("option '{key}'")).map(Some))
        .unwrap_or(Ok(None))
}

fn optional_i64(
    fields: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> NodiaResult<Option<i64>> {
    fields
        .get(key)
        .map(|value| expect_i64(value, name, &format!("option '{key}'")).map(Some))
        .unwrap_or(Ok(None))
}

fn optional_offset(
    fields: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> NodiaResult<Option<i32>> {
    fields
        .get(key)
        .map(|value| expect_offset(value, name, &format!("option '{key}'")).map(Some))
        .unwrap_or(Ok(None))
}

fn split_float_seconds(value: f64) -> NodiaResult<(i64, i64)> {
    if !value.is_finite() {
        return Err(NodiaError::runtime("duration() seconds must be finite"));
    }
    let whole = value.trunc() as i64;
    let nanos = ((value.fract().abs()) * NANOS_PER_SECOND as f64).round() as i64;
    let signed_nanos = if value < 0.0 { -nanos } else { nanos };
    Ok((whole, signed_nanos))
}

fn float_to_seconds_and_nanos(value: f64, name: &str, position: &str) -> NodiaResult<(i64, u32)> {
    if !value.is_finite() {
        return Err(NodiaError::runtime(format!(
            "{name}() expects finite number as {position} argument"
        )));
    }
    let whole = value.floor();
    let fractional = value - whole;
    let mut seconds = whole as i64;
    let mut nanosecond = (fractional * NANOS_PER_SECOND as f64).round() as u32;
    if nanosecond == 1_000_000_000 {
        seconds += 1;
        nanosecond = 0;
    }
    Ok((seconds, nanosecond))
}

fn value_number(number: ValueNumber) -> Value {
    match number {
        ValueNumber::Int(value) => Value::Int(value),
        ValueNumber::Float(value) => Value::Float(value),
    }
}

fn duration_seconds_number(duration: DurationValue) -> ValueNumber {
    let nanos = duration.total_nanoseconds();
    if nanos % NANOS_PER_SECOND == 0 {
        ValueNumber::Int((nanos / NANOS_PER_SECOND) as i64)
    } else {
        ValueNumber::Float(duration.total_seconds())
    }
}
