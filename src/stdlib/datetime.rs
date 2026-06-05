// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Datetime-oriented standard-library functions backed by [`crate::temporal`].

use super::expect_arity;
use crate::error::{DobraError, DobraResult};
use crate::temporal::{
    days_in_month, is_leap_year, parse_offset_text, DateTimeValue, DateValue, DurationValue,
    ValueNumber,
};
use crate::value::Value;
use std::collections::BTreeMap;

const SECONDS_PER_DAY: i64 = 86_400;
const NANOS_PER_SECOND: i128 = 1_000_000_000;

pub fn now(args: &[Value]) -> DobraResult<Value> {
    if args.len() > 1 {
        return Err(DobraError::runtime(format!(
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

pub fn today(args: &[Value]) -> DobraResult<Value> {
    if args.len() > 1 {
        return Err(DobraError::runtime(format!(
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

pub fn date(args: &[Value]) -> DobraResult<Value> {
    let value = match args {
        [Value::Map(fields)] => build_date_from_map(fields, "date")?,
        [year, month, day] => DateValue::new(
            expect_i32(year, "date", "first")?,
            expect_u8(month, "date", "second")?,
            expect_u8(day, "date", "third")?,
        )?,
        _ => {
            return Err(DobraError::runtime(format!(
                "date() expects 1 or 3 argument(s), got {}",
                args.len()
            )));
        }
    };
    Ok(Value::Date(value))
}

pub fn datetime(args: &[Value]) -> DobraResult<Value> {
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
            return Err(DobraError::runtime(format!(
                "datetime() expects 1, 6 or 7 argument(s), got {}",
                args.len()
            )));
        }
    };
    Ok(Value::DateTime(value))
}

pub fn duration(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "duration")?;
    let Value::Map(fields) = &args[0] else {
        return Err(DobraError::runtime(format!(
            "duration() expects map as first argument, got {}",
            args[0].type_name()
        )));
    };

    if fields.is_empty() {
        return Err(DobraError::runtime(
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
            return Err(DobraError::runtime(format!(
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

pub fn parse_date(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "parse_date")?;
    Ok(Value::Date(DateValue::parse_iso(&expect_string(
        &args[0],
        "parse_date",
        "first",
    )?)?))
}

pub fn parse_datetime(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "parse_datetime")?;
    Ok(Value::DateTime(DateTimeValue::parse_iso(&expect_string(
        &args[0],
        "parse_datetime",
        "first",
    )?)?))
}

pub fn parse_duration(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "parse_duration")?;
    Ok(Value::Duration(DurationValue::parse_iso(&expect_string(
        &args[0],
        "parse_duration",
        "first",
    )?)?))
}

pub fn isoformat(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "isoformat")?;
    Ok(Value::String(match &args[0] {
        Value::Date(value) => value.isoformat(),
        Value::DateTime(value) => value.isoformat(),
        Value::Duration(value) => value.isoformat(),
        other => {
            return Err(DobraError::runtime(format!(
                "isoformat() expects date, datetime or duration, got {}",
                other.type_name()
            )));
        }
    }))
}

pub fn strftime(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "strftime")?;
    let pattern = expect_string(&args[1], "strftime", "second")?;
    let text = match &args[0] {
        Value::Date(value) => value.strftime(&pattern)?,
        Value::DateTime(value) => value.strftime(&pattern)?,
        other => {
            return Err(DobraError::runtime(format!(
                "strftime() expects date or datetime as first argument, got {}",
                other.type_name()
            )));
        }
    };
    Ok(Value::String(text))
}

pub fn from_unix(args: &[Value]) -> DobraResult<Value> {
    if args.len() != 1 && args.len() != 2 {
        return Err(DobraError::runtime(format!(
            "from_unix() expects 1 or 2 argument(s), got {}",
            args.len()
        )));
    }
    let offset = args
        .get(1)
        .map(|value| expect_offset(value, "from_unix", "second"))
        .transpose()?
        .unwrap_or(0);
    let (seconds, nanosecond) = expect_unix_seconds_value(&args[0], "from_unix", "first")?;
    Ok(Value::DateTime(DateTimeValue::from_unix_seconds(
        seconds, nanosecond, offset,
    )?))
}

pub fn from_unix_ms(args: &[Value]) -> DobraResult<Value> {
    if args.len() != 1 && args.len() != 2 {
        return Err(DobraError::runtime(format!(
            "from_unix_ms() expects 1 or 2 argument(s), got {}",
            args.len()
        )));
    }
    let offset = args
        .get(1)
        .map(|value| expect_offset(value, "from_unix_ms", "second"))
        .transpose()?
        .unwrap_or(0);
    Ok(Value::DateTime(DateTimeValue::from_unix_milliseconds(
        expect_unix_milliseconds_value(&args[0], "from_unix_ms", "first")?,
        offset,
    )?))
}

pub fn unix_seconds(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "unix_seconds")?;
    Ok(value_number(
        expect_datetime(&args[0], "unix_seconds", "first")?.unix_seconds(),
    ))
}

pub fn unix_ms(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "unix_ms")?;
    Ok(value_number(
        expect_datetime(&args[0], "unix_ms", "first")?.unix_milliseconds(),
    ))
}

pub fn year(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "year")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "year", "first")?.year() as i64,
    ))
}

pub fn month(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "month")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "month", "first")?.month() as i64,
    ))
}

pub fn day(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "day")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "day", "first")?.day() as i64,
    ))
}

pub fn hour(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "hour")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "hour", "first")?.hour() as i64,
    ))
}

pub fn minute(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "minute")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "minute", "first")?.minute() as i64,
    ))
}

pub fn second(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "second")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "second", "first")?.second() as i64,
    ))
}

pub fn nanosecond(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "nanosecond")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "nanosecond", "first")?.nanosecond() as i64,
    ))
}

pub fn weekday(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "weekday")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "weekday", "first")?.weekday() as i64,
    ))
}

pub fn weekday_name(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "weekday_name")?;
    Ok(Value::String(
        expect_date_like(&args[0], "weekday_name", "first")?
            .weekday_long_name()
            .to_string(),
    ))
}

pub fn month_name(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "month_name")?;
    Ok(Value::String(
        expect_date_like(&args[0], "month_name", "first")?
            .month_long_name()
            .to_string(),
    ))
}

pub fn ordinal_day(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "ordinal_day")?;
    Ok(Value::Int(
        expect_date_like(&args[0], "ordinal_day", "first")?.ordinal_day() as i64,
    ))
}

pub fn iso_week(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "iso_week")?;
    let (year, week) = expect_date_like(&args[0], "iso_week", "first")?.iso_week();
    let mut out = BTreeMap::new();
    out.insert("year".to_string(), Value::Int(year as i64));
    out.insert("week".to_string(), Value::Int(week as i64));
    Ok(Value::Map(out))
}

pub fn offset_minutes(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "offset_minutes")?;
    Ok(Value::Int(
        expect_datetime(&args[0], "offset_minutes", "first")?.offset_minutes() as i64,
    ))
}

pub fn days_in_month_value(args: &[Value]) -> DobraResult<Value> {
    let result = match args {
        [Value::Date(value)] => value.days_in_month() as i64,
        [Value::DateTime(value)] => value.days_in_month() as i64,
        [value] => {
            return Err(DobraError::runtime(format!(
                "days_in_month() expects date/datetime or year/month, got {}",
                value.type_name()
            )));
        }
        [year, month] => days_in_month(
            expect_i32(year, "days_in_month", "first")?,
            expect_u8(month, "days_in_month", "second")?,
        ) as i64,
        _ => {
            return Err(DobraError::runtime(format!(
                "days_in_month() expects 1 or 2 argument(s), got {}",
                args.len()
            )));
        }
    };
    Ok(Value::Int(result))
}

pub fn is_leap_year_value(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "is_leap_year")?;
    let result = match &args[0] {
        Value::Int(year) => is_leap_year(i32::try_from(*year).map_err(|_| {
            DobraError::runtime("is_leap_year() expects int within i32 range as first argument")
        })?),
        Value::Date(value) => value.is_leap_year(),
        Value::DateTime(value) => value.is_leap_year(),
        other => {
            return Err(DobraError::runtime(format!(
                "is_leap_year() expects int, date or datetime, got {}",
                other.type_name()
            )));
        }
    };
    Ok(Value::Bool(result))
}

pub fn date_only(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "date_only")?;
    Ok(Value::Date(expect_date_like(
        &args[0],
        "date_only",
        "first",
    )?))
}

pub fn with_offset(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "with_offset")?;
    Ok(Value::DateTime(
        expect_datetime(&args[0], "with_offset", "first")?.with_offset(expect_offset(
            &args[1],
            "with_offset",
            "second",
        )?)?,
    ))
}

pub fn add_days(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "add_days")?;
    let amount = expect_i64(&args[1], "add_days", "second")?;
    match &args[0] {
        Value::Date(value) => Ok(Value::Date(value.add_days(amount)?)),
        Value::DateTime(value) => Ok(Value::DateTime(value.add_days(amount)?)),
        other => Err(DobraError::runtime(format!(
            "add_days() expects date or datetime as first argument, got {}",
            other.type_name()
        ))),
    }
}

pub fn add_months(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "add_months")?;
    let amount = expect_i32(&args[1], "add_months", "second")?;
    match &args[0] {
        Value::Date(value) => Ok(Value::Date(value.add_months(amount)?)),
        Value::DateTime(value) => Ok(Value::DateTime(value.add_months(amount)?)),
        other => Err(DobraError::runtime(format!(
            "add_months() expects date or datetime as first argument, got {}",
            other.type_name()
        ))),
    }
}

pub fn add_years(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "add_years")?;
    let amount = expect_i32(&args[1], "add_years", "second")?;
    match &args[0] {
        Value::Date(value) => Ok(Value::Date(value.add_years(amount)?)),
        Value::DateTime(value) => Ok(Value::DateTime(value.add_years(amount)?)),
        other => Err(DobraError::runtime(format!(
            "add_years() expects date or datetime as first argument, got {}",
            other.type_name()
        ))),
    }
}

pub fn add_duration(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "add_duration")?;
    let duration = expect_duration(&args[1], "add_duration", "second")?;
    match &args[0] {
        Value::DateTime(value) => Ok(Value::DateTime(value.add_duration(duration)?)),
        Value::Duration(value) => Ok(Value::Duration(DurationValue::from_total_nanoseconds(
            value.total_nanoseconds() + duration.total_nanoseconds(),
        ))),
        other => Err(DobraError::runtime(format!(
            "add_duration() expects datetime or duration as first argument, got {}",
            other.type_name()
        ))),
    }
}

pub fn diff_days(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "diff_days")?;
    let left = expect_date_like(&args[0], "diff_days", "first")?;
    let right = expect_date_like(&args[1], "diff_days", "second")?;
    Ok(Value::Int(
        (left.days_since_epoch() - right.days_since_epoch()) as i64,
    ))
}

pub fn diff_seconds(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "diff_seconds")?;
    let seconds = match (&args[0], &args[1]) {
        (Value::Date(left), Value::Date(right)) => {
            duration_seconds_number(DurationValue::from_total_nanoseconds(
                (left.days_since_epoch() as i128 - right.days_since_epoch() as i128)
                    * SECONDS_PER_DAY as i128
                    * NANOS_PER_SECOND,
            ))
        }
        (Value::DateTime(left), Value::DateTime(right)) => {
            duration_seconds_number(left.diff(*right))
        }
        (Value::Duration(left), Value::Duration(right)) => {
            duration_seconds_number(DurationValue::from_total_nanoseconds(
                left.total_nanoseconds() - right.total_nanoseconds(),
            ))
        }
        (left, right) => {
            return Err(DobraError::runtime(format!(
                "diff_seconds() expects date/date, datetime/datetime or duration/duration, got {}/{}",
                left.type_name(),
                right.type_name()
            )));
        }
    };
    Ok(value_number(seconds))
}

pub fn diff_duration(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 2, "diff_duration")?;
    let value = match (&args[0], &args[1]) {
        (Value::Date(left), Value::Date(right)) => DurationValue::from_total_nanoseconds(
            (left.days_since_epoch() as i128 - right.days_since_epoch() as i128)
                * SECONDS_PER_DAY as i128
                * NANOS_PER_SECOND,
        ),
        (Value::DateTime(left), Value::DateTime(right)) => left.diff(*right),
        (Value::Duration(left), Value::Duration(right)) => DurationValue::from_total_nanoseconds(
            left.total_nanoseconds() - right.total_nanoseconds(),
        ),
        (left, right) => {
            return Err(DobraError::runtime(format!(
                "diff_duration() expects matching date, datetime or duration values, got {}/{}",
                left.type_name(),
                right.type_name()
            )));
        }
    };
    Ok(Value::Duration(value))
}

pub fn start_of_day(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "start_of_day")?;
    match &args[0] {
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
        other => Err(DobraError::runtime(format!(
            "start_of_day() expects date or datetime, got {}",
            other.type_name()
        ))),
    }
}

pub fn end_of_day(args: &[Value]) -> DobraResult<Value> {
    expect_arity(args, 1, "end_of_day")?;
    match &args[0] {
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
        other => Err(DobraError::runtime(format!(
            "end_of_day() expects date or datetime, got {}",
            other.type_name()
        ))),
    }
}

fn build_date_from_map(fields: &BTreeMap<String, Value>, name: &str) -> DobraResult<DateValue> {
    DateValue::new(
        required_i32(fields, "year", name)?,
        required_u8(fields, "month", name)?,
        required_u8(fields, "day", name)?,
    )
}

fn build_datetime_from_map(
    fields: &BTreeMap<String, Value>,
    name: &str,
) -> DobraResult<DateTimeValue> {
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

fn datetime_options(value: &Value, name: &str, position: &str) -> DobraResult<(u32, i32)> {
    match value {
        Value::Map(fields) => Ok((
            optional_u32(fields, "nanosecond", name)?.unwrap_or(0),
            optional_offset(fields, "offset", name)?.unwrap_or(0),
        )),
        Value::Int(_) | Value::String(_) => Ok((0, expect_offset(value, name, position)?)),
        other => Err(DobraError::runtime(format!(
            "{name}() expects offset or map as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_date_like(value: &Value, name: &str, position: &str) -> DobraResult<DateValue> {
    match value {
        Value::Date(value) => Ok(*value),
        Value::DateTime(value) => Ok(value.date()),
        other => Err(DobraError::runtime(format!(
            "{name}() expects date or datetime as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_datetime(value: &Value, name: &str, position: &str) -> DobraResult<DateTimeValue> {
    match value {
        Value::DateTime(value) => Ok(*value),
        other => Err(DobraError::runtime(format!(
            "{name}() expects datetime as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_duration(value: &Value, name: &str, position: &str) -> DobraResult<DurationValue> {
    match value {
        Value::Duration(value) => Ok(*value),
        other => Err(DobraError::runtime(format!(
            "{name}() expects duration as {position} argument, got {}",
            other.type_name()
        ))),
    }
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

fn expect_i64(value: &Value, name: &str, position: &str) -> DobraResult<i64> {
    match value {
        Value::Int(value) => Ok(*value),
        other => Err(DobraError::runtime(format!(
            "{name}() expects int as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_i32(value: &Value, name: &str, position: &str) -> DobraResult<i32> {
    i32::try_from(expect_i64(value, name, position)?).map_err(|_| {
        DobraError::runtime(format!(
            "{name}() int in {position} argument is outside i32 range"
        ))
    })
}

fn expect_u8(value: &Value, name: &str, position: &str) -> DobraResult<u8> {
    u8::try_from(expect_i64(value, name, position)?).map_err(|_| {
        DobraError::runtime(format!(
            "{name}() int in {position} argument is outside u8 range"
        ))
    })
}

fn expect_u32(value: &Value, name: &str, position: &str) -> DobraResult<u32> {
    u32::try_from(expect_i64(value, name, position)?).map_err(|_| {
        DobraError::runtime(format!(
            "{name}() int in {position} argument is outside u32 range"
        ))
    })
}

fn expect_offset(value: &Value, name: &str, position: &str) -> DobraResult<i32> {
    match value {
        Value::Int(value) => i32::try_from(*value).map_err(|_| {
            DobraError::runtime(format!(
                "{name}() expects offset within i32 range as {position} argument"
            ))
        }),
        Value::String(value) => parse_offset_text(value),
        other => Err(DobraError::runtime(format!(
            "{name}() expects int or string offset as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_unix_seconds_value(value: &Value, name: &str, position: &str) -> DobraResult<(i64, u32)> {
    match value {
        Value::Int(value) => Ok((*value, 0)),
        Value::Float(value) => float_to_seconds_and_nanos(*value, name, position),
        other => Err(DobraError::runtime(format!(
            "{name}() expects int or float as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn expect_unix_milliseconds_value(value: &Value, name: &str, position: &str) -> DobraResult<i128> {
    match value {
        Value::Int(value) => Ok(*value as i128),
        Value::Float(value) => {
            if !value.is_finite() {
                return Err(DobraError::runtime(format!(
                    "{name}() expects finite number as {position} argument"
                )));
            }
            Ok(value.round() as i128)
        }
        other => Err(DobraError::runtime(format!(
            "{name}() expects int or float as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn required_i32(fields: &BTreeMap<String, Value>, key: &str, name: &str) -> DobraResult<i32> {
    let value = fields
        .get(key)
        .ok_or_else(|| DobraError::runtime(format!("{name}() missing required option '{key}'")))?;
    expect_i32(value, name, &format!("option '{key}'"))
}

fn required_u8(fields: &BTreeMap<String, Value>, key: &str, name: &str) -> DobraResult<u8> {
    let value = fields
        .get(key)
        .ok_or_else(|| DobraError::runtime(format!("{name}() missing required option '{key}'")))?;
    expect_u8(value, name, &format!("option '{key}'"))
}

fn optional_u32(
    fields: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> DobraResult<Option<u32>> {
    fields
        .get(key)
        .map(|value| expect_u32(value, name, &format!("option '{key}'")).map(Some))
        .unwrap_or(Ok(None))
}

fn optional_i64(
    fields: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> DobraResult<Option<i64>> {
    fields
        .get(key)
        .map(|value| expect_i64(value, name, &format!("option '{key}'")).map(Some))
        .unwrap_or(Ok(None))
}

fn optional_offset(
    fields: &BTreeMap<String, Value>,
    key: &str,
    name: &str,
) -> DobraResult<Option<i32>> {
    fields
        .get(key)
        .map(|value| expect_offset(value, name, &format!("option '{key}'")).map(Some))
        .unwrap_or(Ok(None))
}

fn split_float_seconds(value: f64) -> DobraResult<(i64, i64)> {
    if !value.is_finite() {
        return Err(DobraError::runtime("duration() seconds must be finite"));
    }
    let whole = value.trunc() as i64;
    let nanos = ((value.fract().abs()) * NANOS_PER_SECOND as f64).round() as i64;
    let signed_nanos = if value < 0.0 { -nanos } else { nanos };
    Ok((whole, signed_nanos))
}

fn float_to_seconds_and_nanos(value: f64, name: &str, position: &str) -> DobraResult<(i64, u32)> {
    if !value.is_finite() {
        return Err(DobraError::runtime(format!(
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
