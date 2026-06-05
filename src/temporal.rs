// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Date, datetime, duration, and formatting helpers used by the standard library.

use crate::error::{DobraError, DobraResult};
use std::cmp::min;
use std::time::{SystemTime, UNIX_EPOCH};

const SECONDS_PER_MINUTE: i64 = 60;
const MINUTES_PER_HOUR: i32 = 60;
const HOURS_PER_DAY: i64 = 24;
const SECONDS_PER_HOUR: i64 = SECONDS_PER_MINUTE * MINUTES_PER_HOUR as i64;
const SECONDS_PER_DAY: i64 = SECONDS_PER_HOUR * HOURS_PER_DAY;
const NANOS_PER_SECOND: i128 = 1_000_000_000;
const NANOS_PER_MILLISECOND: i128 = 1_000_000;
const NANOS_PER_MICROSECOND: i128 = 1_000;
const NANOS_PER_DAY: i128 = SECONDS_PER_DAY as i128 * NANOS_PER_SECOND;
const MAX_OFFSET_MINUTES: i32 = 23 * 60 + 59;

const WEEKDAY_SHORT: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const WEEKDAY_LONG: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const MONTH_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTH_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Calendar date stored as days since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateValue {
    days_since_epoch: i32,
}

/// Offset-aware datetime value with nanosecond precision.
#[derive(Debug, Clone, Copy)]
pub struct DateTimeValue {
    date: DateValue,
    seconds_of_day: u32,
    nanosecond: u32,
    offset_minutes: i32,
}

/// Signed duration stored in total nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DurationValue {
    total_nanoseconds: i128,
}

/// Expanded calendar components for a [`DateValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateParts {
    /// Four-digit or extended year value.
    pub year: i32,
    /// Month in the range `1..=12`.
    pub month: u8,
    /// Day in the range `1..=31`.
    pub day: u8,
}

/// Expanded datetime components for a [`DateTimeValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTimeParts {
    /// Four-digit or extended year value.
    pub year: i32,
    /// Month in the range `1..=12`.
    pub month: u8,
    /// Day in the range `1..=31`.
    pub day: u8,
    /// Hour in the range `0..=23`.
    pub hour: u8,
    /// Minute in the range `0..=59`.
    pub minute: u8,
    /// Second in the range `0..=59`.
    pub second: u8,
    /// Fractional second in nanoseconds.
    pub nanosecond: u32,
    /// Fixed offset from UTC in minutes.
    pub offset_minutes: i32,
}

impl DateValue {
    /// Creates a validated calendar date.
    pub fn new(year: i32, month: u8, day: u8) -> DobraResult<Self> {
        validate_month(month)?;
        let last_day = days_in_month(year, month);
        if day == 0 || day > last_day {
            return Err(DobraError::runtime(format!(
                "invalid date {year:04}-{month:02}-{day:02}"
            )));
        }
        let days = days_from_civil(year, month, day)?;
        Ok(Self {
            days_since_epoch: days,
        })
    }

    /// Reconstructs a date from its internal day count.
    pub fn from_days_since_epoch(days_since_epoch: i32) -> Self {
        Self { days_since_epoch }
    }

    /// Returns the current date at the given UTC offset in minutes.
    pub fn today(offset_minutes: i32) -> DobraResult<Self> {
        Ok(DateTimeValue::now(offset_minutes)?.date())
    }

    /// Parses an ISO 8601 calendar date such as `2026-06-04`.
    pub fn parse_iso(text: &str) -> DobraResult<Self> {
        let (year, month, day) = parse_date_fields(text)?;
        Self::new(year, month, day)
    }

    /// Expands the date into calendar parts.
    pub fn parts(self) -> DateParts {
        let (year, month, day) = civil_from_days(self.days_since_epoch as i64);
        DateParts { year, month, day }
    }

    /// Returns the internal day count since the Unix epoch.
    pub fn days_since_epoch(self) -> i32 {
        self.days_since_epoch
    }

    /// Returns the calendar year.
    pub fn year(self) -> i32 {
        self.parts().year
    }

    /// Returns the calendar month in the range `1..=12`.
    pub fn month(self) -> u8 {
        self.parts().month
    }

    /// Returns the day of month in the range `1..=31`.
    pub fn day(self) -> u8 {
        self.parts().day
    }

    /// Adds a whole number of days.
    pub fn add_days(self, days: i64) -> DobraResult<Self> {
        let next = self.days_since_epoch as i64 + days;
        Ok(Self::from_days_since_epoch(i32_from_i64(next)?))
    }

    /// Adds calendar months while clamping the day to the destination month.
    pub fn add_months(self, months: i32) -> DobraResult<Self> {
        let parts = self.parts();
        let total = parts.year as i64 * 12 + (parts.month as i64 - 1) + months as i64;
        let year = total.div_euclid(12);
        let month = total.rem_euclid(12) as u8 + 1;
        let day = min(parts.day, days_in_month(i32_from_i64(year)?, month));
        Self::new(i32_from_i64(year)?, month, day)
    }

    /// Adds calendar years while preserving month and clamping the day when needed.
    pub fn add_years(self, years: i32) -> DobraResult<Self> {
        let parts = self.parts();
        let year = parts.year.checked_add(years).ok_or_else(|| {
            DobraError::runtime("date arithmetic overflowed supported year range")
        })?;
        let day = min(parts.day, days_in_month(year, parts.month));
        Self::new(year, parts.month, day)
    }

    /// Returns the ISO weekday number where Monday is `1`.
    pub fn weekday(self) -> u8 {
        ((self.days_since_epoch as i64 + 3).rem_euclid(7) + 1) as u8
    }

    /// Returns the abbreviated English weekday name.
    pub fn weekday_short_name(self) -> &'static str {
        WEEKDAY_SHORT[self.weekday() as usize - 1]
    }

    /// Returns the full English weekday name.
    pub fn weekday_long_name(self) -> &'static str {
        WEEKDAY_LONG[self.weekday() as usize - 1]
    }

    /// Returns the abbreviated English month name.
    pub fn month_short_name(self) -> &'static str {
        MONTH_SHORT[self.month() as usize - 1]
    }

    /// Returns the full English month name.
    pub fn month_long_name(self) -> &'static str {
        MONTH_LONG[self.month() as usize - 1]
    }

    /// Returns the ordinal day within the year.
    pub fn ordinal_day(self) -> u16 {
        let parts = self.parts();
        let mut ordinal = parts.day as u16;
        for month in 1..parts.month {
            ordinal += days_in_month(parts.year, month) as u16;
        }
        ordinal
    }

    /// Returns the ISO week-year and week number.
    pub fn iso_week(self) -> (i32, u8) {
        let weekday = self.weekday() as i64;
        let thursday = self
            .add_days(4 - weekday)
            .expect("weekday offset stays in range");
        let iso_year = thursday.year();
        let jan4 = DateValue::new(iso_year, 1, 4).expect("jan 4 is always valid");
        let week1_monday = jan4
            .add_days(1 - jan4.weekday() as i64)
            .expect("week arithmetic stays in range");
        let week =
            ((self.days_since_epoch as i64 - week1_monday.days_since_epoch as i64) / 7 + 1) as u8;
        (iso_year, week)
    }

    /// Returns the number of days in the current month.
    pub fn days_in_month(self) -> u8 {
        let parts = self.parts();
        days_in_month(parts.year, parts.month)
    }

    /// Reports whether the current year is a leap year.
    pub fn is_leap_year(self) -> bool {
        is_leap_year(self.year())
    }

    /// Formats the date as ISO 8601 text.
    pub fn isoformat(self) -> String {
        let parts = self.parts();
        format!(
            "{}-{:02}-{:02}",
            format_year(parts.year),
            parts.month,
            parts.day
        )
    }

    /// Formats the date using a restricted `strftime`-style pattern set.
    pub fn strftime(self, pattern: &str) -> DobraResult<String> {
        let parts = self.parts();
        format_strftime(
            pattern,
            parts,
            TimeParts {
                hour: 0,
                minute: 0,
                second: 0,
                nanosecond: 0,
                offset_minutes: None,
            },
            self.ordinal_day(),
            self.weekday(),
            self.iso_week(),
        )
    }
}

impl DateTimeValue {
    /// Creates a validated datetime with a fixed UTC offset in minutes.
    pub fn new(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
        offset_minutes: i32,
    ) -> DobraResult<Self> {
        validate_month(month)?;
        validate_time(hour, minute, second, nanosecond)?;
        validate_offset(offset_minutes)?;
        let date = DateValue::new(year, month, day)?;
        let seconds_of_day = hour as u32 * 3600 + minute as u32 * 60 + second as u32;
        Ok(Self {
            date,
            seconds_of_day,
            nanosecond,
            offset_minutes,
        })
    }

    /// Returns the current datetime at the given UTC offset in minutes.
    pub fn now(offset_minutes: i32) -> DobraResult<Self> {
        validate_offset(offset_minutes)?;
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| DobraError::runtime(format!("system clock before unix epoch: {err}")))?;
        let total = duration.as_secs() as i128 * NANOS_PER_SECOND + duration.subsec_nanos() as i128;
        Self::from_unix_total_nanos(total, offset_minutes)
    }

    /// Builds a datetime from Unix seconds plus a nanosecond fraction.
    pub fn from_unix_seconds(
        unix_seconds: i64,
        nanosecond: u32,
        offset_minutes: i32,
    ) -> DobraResult<Self> {
        validate_time(0, 0, 0, nanosecond)?;
        validate_offset(offset_minutes)?;
        let total = unix_seconds as i128 * NANOS_PER_SECOND + nanosecond as i128;
        Self::from_unix_total_nanos(total, offset_minutes)
    }

    /// Builds a datetime from Unix milliseconds.
    pub fn from_unix_milliseconds(
        unix_milliseconds: i128,
        offset_minutes: i32,
    ) -> DobraResult<Self> {
        validate_offset(offset_minutes)?;
        Self::from_unix_total_nanos(unix_milliseconds * NANOS_PER_MILLISECOND, offset_minutes)
    }

    /// Parses an ISO 8601 datetime with an explicit offset.
    pub fn parse_iso(text: &str) -> DobraResult<Self> {
        let trimmed = text.trim();
        let split = trimmed.find(['T', 't', ' ']).ok_or_else(|| {
            DobraError::runtime("datetime text must separate date and time with 'T' or space")
        })?;
        let (date_text, rest) = trimmed.split_at(split);
        let time_text = &rest[1..];
        let (year, month, day) = parse_date_fields(date_text)?;
        let (hour, minute, second, nanosecond, offset_minutes) = parse_time_and_offset(time_text)?;
        Self::new(
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanosecond,
            offset_minutes,
        )
    }

    /// Returns the calendar date component.
    pub fn date(self) -> DateValue {
        self.date
    }

    /// Returns the calendar year.
    pub fn year(self) -> i32 {
        self.date.year()
    }

    /// Returns the calendar month in the range `1..=12`.
    pub fn month(self) -> u8 {
        self.date.month()
    }

    /// Returns the day of month in the range `1..=31`.
    pub fn day(self) -> u8 {
        self.date.day()
    }

    /// Returns the hour in the range `0..=23`.
    pub fn hour(self) -> u8 {
        (self.seconds_of_day / 3600) as u8
    }

    /// Returns the minute in the range `0..=59`.
    pub fn minute(self) -> u8 {
        ((self.seconds_of_day % 3600) / 60) as u8
    }

    /// Returns the second in the range `0..=59`.
    pub fn second(self) -> u8 {
        (self.seconds_of_day % 60) as u8
    }

    /// Returns the nanosecond fraction.
    pub fn nanosecond(self) -> u32 {
        self.nanosecond
    }

    /// Returns the fixed UTC offset in minutes.
    pub fn offset_minutes(self) -> i32 {
        self.offset_minutes
    }

    /// Returns the ISO weekday number where Monday is `1`.
    pub fn weekday(self) -> u8 {
        self.date.weekday()
    }

    /// Returns the abbreviated English weekday name.
    pub fn weekday_short_name(self) -> &'static str {
        self.date.weekday_short_name()
    }

    /// Returns the full English weekday name.
    pub fn weekday_long_name(self) -> &'static str {
        self.date.weekday_long_name()
    }

    /// Returns the abbreviated English month name.
    pub fn month_short_name(self) -> &'static str {
        self.date.month_short_name()
    }

    /// Returns the full English month name.
    pub fn month_long_name(self) -> &'static str {
        self.date.month_long_name()
    }

    /// Returns the ordinal day within the year.
    pub fn ordinal_day(self) -> u16 {
        self.date.ordinal_day()
    }

    /// Returns the ISO week-year and week number.
    pub fn iso_week(self) -> (i32, u8) {
        self.date.iso_week()
    }

    /// Returns the number of days in the current month.
    pub fn days_in_month(self) -> u8 {
        self.date.days_in_month()
    }

    /// Reports whether the current year is a leap year.
    pub fn is_leap_year(self) -> bool {
        self.date.is_leap_year()
    }

    /// Re-expresses the same absolute instant with a different offset.
    pub fn with_offset(self, offset_minutes: i32) -> DobraResult<Self> {
        Self::from_unix_total_nanos(self.to_unix_total_nanos(), offset_minutes)
    }

    /// Adds a whole number of days without changing the local time fields.
    pub fn add_days(self, days: i64) -> DobraResult<Self> {
        Ok(Self {
            date: self.date.add_days(days)?,
            ..self
        })
    }

    /// Adds calendar months while clamping the day when needed.
    pub fn add_months(self, months: i32) -> DobraResult<Self> {
        Ok(Self {
            date: self.date.add_months(months)?,
            ..self
        })
    }

    /// Adds calendar years while clamping the day when needed.
    pub fn add_years(self, years: i32) -> DobraResult<Self> {
        Ok(Self {
            date: self.date.add_years(years)?,
            ..self
        })
    }

    /// Adds an exact duration in nanoseconds.
    pub fn add_duration(self, duration: DurationValue) -> DobraResult<Self> {
        Self::from_unix_total_nanos(
            self.to_unix_total_nanos()
                .checked_add(duration.total_nanoseconds())
                .ok_or_else(|| DobraError::runtime("datetime arithmetic overflowed"))?,
            self.offset_minutes,
        )
    }

    /// Computes the signed duration between two absolute instants.
    pub fn diff(self, other: Self) -> DurationValue {
        DurationValue::from_total_nanoseconds(
            self.to_unix_total_nanos() - other.to_unix_total_nanos(),
        )
    }

    /// Returns the earliest representable time on the same local day.
    pub fn start_of_day(self) -> Self {
        Self {
            seconds_of_day: 0,
            nanosecond: 0,
            ..self
        }
    }

    /// Returns the latest representable time on the same local day.
    pub fn end_of_day(self) -> Self {
        Self {
            seconds_of_day: (SECONDS_PER_DAY - 1) as u32,
            nanosecond: 999_999_999,
            ..self
        }
    }

    /// Returns Unix time in seconds, preserving sub-second precision when needed.
    pub fn unix_seconds(self) -> ValueNumber {
        if self.nanosecond == 0 {
            ValueNumber::Int(self.to_unix_seconds())
        } else {
            ValueNumber::Float(
                self.to_unix_seconds() as f64 + self.nanosecond as f64 / NANOS_PER_SECOND as f64,
            )
        }
    }

    /// Returns Unix time in milliseconds, preserving fractional precision when needed.
    pub fn unix_milliseconds(self) -> ValueNumber {
        let total = self.to_unix_total_nanos() / NANOS_PER_MILLISECOND;
        if self.nanosecond % 1_000_000 == 0 {
            ValueNumber::Int(i64_from_i128(total).unwrap_or(i64::MAX))
        } else {
            ValueNumber::Float(self.to_unix_total_nanos() as f64 / NANOS_PER_MILLISECOND as f64)
        }
    }

    /// Formats the datetime as ISO 8601 text.
    pub fn isoformat(self) -> String {
        let date = self.date.isoformat();
        let mut out = format!(
            "{date}T{:02}:{:02}:{:02}",
            self.hour(),
            self.minute(),
            self.second()
        );
        if self.nanosecond != 0 {
            out.push('.');
            out.push_str(&trimmed_fraction(self.nanosecond));
        }
        out.push_str(&format_offset(self.offset_minutes));
        out
    }

    /// Formats the datetime using a restricted `strftime`-style pattern set.
    pub fn strftime(self, pattern: &str) -> DobraResult<String> {
        format_strftime(
            pattern,
            self.date.parts(),
            TimeParts {
                hour: self.hour(),
                minute: self.minute(),
                second: self.second(),
                nanosecond: self.nanosecond,
                offset_minutes: Some(self.offset_minutes),
            },
            self.ordinal_day(),
            self.weekday(),
            self.iso_week(),
        )
    }

    fn to_unix_total_nanos(self) -> i128 {
        let local_seconds = self.date.days_since_epoch as i128 * SECONDS_PER_DAY as i128
            + self.seconds_of_day as i128;
        let utc_seconds = local_seconds - self.offset_minutes as i128 * SECONDS_PER_MINUTE as i128;
        utc_seconds * NANOS_PER_SECOND + self.nanosecond as i128
    }

    fn to_unix_seconds(self) -> i64 {
        (self.to_unix_total_nanos() / NANOS_PER_SECOND) as i64
    }

    fn from_unix_total_nanos(total_nanoseconds: i128, offset_minutes: i32) -> DobraResult<Self> {
        validate_offset(offset_minutes)?;
        let shifted = total_nanoseconds
            + offset_minutes as i128 * SECONDS_PER_MINUTE as i128 * NANOS_PER_SECOND;
        let days = shifted.div_euclid(NANOS_PER_DAY);
        let nanos_of_day = shifted.rem_euclid(NANOS_PER_DAY);
        let seconds_of_day = (nanos_of_day / NANOS_PER_SECOND) as u32;
        let nanosecond = (nanos_of_day % NANOS_PER_SECOND) as u32;
        Ok(Self {
            date: DateValue::from_days_since_epoch(i32_from_i128(days)?),
            seconds_of_day,
            nanosecond,
            offset_minutes,
        })
    }
}

impl PartialEq for DateTimeValue {
    fn eq(&self, other: &Self) -> bool {
        self.to_unix_total_nanos() == other.to_unix_total_nanos()
    }
}

impl Eq for DateTimeValue {}

impl PartialOrd for DateTimeValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTimeValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_unix_total_nanos().cmp(&other.to_unix_total_nanos())
    }
}

impl DurationValue {
    /// Creates a duration directly from total nanoseconds.
    pub fn from_total_nanoseconds(total_nanoseconds: i128) -> Self {
        Self { total_nanoseconds }
    }

    /// Creates a duration by summing calendar-free duration components.
    pub fn from_parts(
        weeks: i64,
        days: i64,
        hours: i64,
        minutes: i64,
        seconds: i64,
        milliseconds: i64,
        microseconds: i64,
        nanoseconds: i64,
    ) -> DobraResult<Self> {
        let total = checked_sum_i128(&[
            weeks as i128 * 7 * NANOS_PER_DAY,
            days as i128 * NANOS_PER_DAY,
            hours as i128 * SECONDS_PER_HOUR as i128 * NANOS_PER_SECOND,
            minutes as i128 * SECONDS_PER_MINUTE as i128 * NANOS_PER_SECOND,
            seconds as i128 * NANOS_PER_SECOND,
            milliseconds as i128 * NANOS_PER_MILLISECOND,
            microseconds as i128 * NANOS_PER_MICROSECOND,
            nanoseconds as i128,
        ])?;
        Ok(Self::from_total_nanoseconds(total))
    }

    /// Parses an ISO 8601 duration such as `PT1H30M`.
    pub fn parse_iso(text: &str) -> DobraResult<Self> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(DobraError::runtime("duration text cannot be empty"));
        }
        let (negative, body) = if let Some(rest) = trimmed.strip_prefix('-') {
            (true, rest)
        } else if let Some(rest) = trimmed.strip_prefix('+') {
            (false, rest)
        } else {
            (false, trimmed)
        };
        if !body.starts_with('P') {
            return Err(DobraError::runtime("duration text must start with 'P'"));
        }

        let mut index = 1usize;
        let chars: Vec<char> = body.chars().collect();
        let mut in_time = false;
        let mut seen_any = false;
        let mut total = 0i128;

        while index < chars.len() {
            if chars[index] == 'T' {
                in_time = true;
                index += 1;
                continue;
            }

            let start = index;
            while index < chars.len() && (chars[index].is_ascii_digit() || chars[index] == '.') {
                index += 1;
            }
            if start == index || index == chars.len() {
                return Err(DobraError::runtime("invalid duration component"));
            }
            let number = chars[start..index].iter().collect::<String>();
            let designator = chars[index];
            index += 1;
            seen_any = true;

            total = total
                .checked_add(match designator {
                    'W' if !in_time => {
                        parse_int_component(&number, "weeks")? as i128 * 7 * NANOS_PER_DAY
                    }
                    'D' if !in_time => {
                        parse_int_component(&number, "days")? as i128 * NANOS_PER_DAY
                    }
                    'H' if in_time => {
                        parse_int_component(&number, "hours")? as i128
                            * SECONDS_PER_HOUR as i128
                            * NANOS_PER_SECOND
                    }
                    'M' if in_time => {
                        parse_int_component(&number, "minutes")? as i128
                            * SECONDS_PER_MINUTE as i128
                            * NANOS_PER_SECOND
                    }
                    'S' if in_time => parse_second_fraction(&number)?,
                    'Y' | 'M' => {
                        return Err(DobraError::runtime(
                            "duration text does not support years or calendar months",
                        ))
                    }
                    _ => return Err(DobraError::runtime("invalid duration designator")),
                })
                .ok_or_else(|| DobraError::runtime("duration overflowed supported range"))?;
        }

        if !seen_any {
            return Err(DobraError::runtime(
                "duration text must include at least one component",
            ));
        }

        Ok(Self::from_total_nanoseconds(if negative {
            -total
        } else {
            total
        }))
    }

    /// Returns the total duration in nanoseconds.
    pub fn total_nanoseconds(self) -> i128 {
        self.total_nanoseconds
    }

    /// Returns the total duration in seconds as a floating-point value.
    pub fn total_seconds(self) -> f64 {
        self.total_nanoseconds as f64 / NANOS_PER_SECOND as f64
    }

    /// Returns the total duration in milliseconds as a floating-point value.
    pub fn total_milliseconds(self) -> f64 {
        self.total_nanoseconds as f64 / NANOS_PER_MILLISECOND as f64
    }

    /// Returns the whole-day component using Euclidean division.
    pub fn whole_days(self) -> i64 {
        (self.total_nanoseconds.div_euclid(NANOS_PER_DAY)) as i64
    }

    /// Formats the duration as ISO 8601 text.
    pub fn isoformat(self) -> String {
        if self.total_nanoseconds == 0 {
            return "PT0S".to_string();
        }

        let negative = self.total_nanoseconds < 0;
        let mut remaining = self.total_nanoseconds.abs();
        let days = remaining / NANOS_PER_DAY;
        remaining %= NANOS_PER_DAY;
        let hours = remaining / (SECONDS_PER_HOUR as i128 * NANOS_PER_SECOND);
        remaining %= SECONDS_PER_HOUR as i128 * NANOS_PER_SECOND;
        let minutes = remaining / (SECONDS_PER_MINUTE as i128 * NANOS_PER_SECOND);
        remaining %= SECONDS_PER_MINUTE as i128 * NANOS_PER_SECOND;
        let seconds = remaining / NANOS_PER_SECOND;
        let nanos = (remaining % NANOS_PER_SECOND) as u32;

        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out.push('P');
        if days != 0 {
            out.push_str(&format!("{days}D"));
        }
        if hours != 0 || minutes != 0 || seconds != 0 || nanos != 0 {
            out.push('T');
            if hours != 0 {
                out.push_str(&format!("{hours}H"));
            }
            if minutes != 0 {
                out.push_str(&format!("{minutes}M"));
            }
            if nanos == 0 {
                if seconds != 0 {
                    out.push_str(&format!("{seconds}S"));
                }
            } else {
                out.push_str(&format!("{seconds}.{}S", trimmed_fraction(nanos)));
            }
        }
        out
    }
}

/// Numeric return type used when temporal conversions may be integral or fractional.
#[derive(Debug, Clone, Copy)]
pub enum ValueNumber {
    /// Integer-valued result.
    Int(i64),
    /// Floating-point result.
    Float(f64),
}

#[derive(Debug, Clone, Copy)]
struct TimeParts {
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
    offset_minutes: Option<i32>,
}

/// Parses a UTC offset written as `Z`, `+HH`, `+HHMM`, or `+HH:MM`.
pub fn parse_offset_text(text: &str) -> DobraResult<i32> {
    if text == "Z" || text == "z" {
        return Ok(0);
    }
    let bytes = text.as_bytes();
    let Some(sign) = bytes.first().copied() else {
        return Err(DobraError::runtime("offset text cannot be empty"));
    };
    let sign = match sign {
        b'+' => 1,
        b'-' => -1,
        _ => {
            return Err(DobraError::runtime(
                "offset text must start with '+', '-', or 'Z'",
            ))
        }
    };
    let digits = &text[1..];
    let (hours, minutes) = if let Some((hours, minutes)) = digits.split_once(':') {
        (hours, minutes)
    } else if digits.len() == 2 {
        (digits, "00")
    } else if digits.len() == 4 {
        (&digits[..2], &digits[2..])
    } else {
        return Err(DobraError::runtime(
            "offset text must use +HH, +HHMM, or +HH:MM",
        ));
    };
    let hours = parse_u8(hours, "offset hour")?;
    let minutes = parse_u8(minutes, "offset minute")?;
    let total = hours as i32 * 60 + minutes as i32;
    validate_offset(total * sign)?;
    Ok(total * sign)
}

/// Reports whether a Gregorian year is a leap year.
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Returns the number of days in a Gregorian month.
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn format_strftime(
    pattern: &str,
    date: DateParts,
    time: TimeParts,
    ordinal_day: u16,
    weekday: u8,
    iso_week: (i32, u8),
) -> DobraResult<String> {
    let mut out = String::with_capacity(pattern.len() + 16);
    let chars: Vec<char> = pattern.chars().collect();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] != '%' {
            out.push(chars[index]);
            index += 1;
            continue;
        }
        index += 1;
        if index >= chars.len() {
            return Err(DobraError::runtime("strftime() pattern ends with '%'"));
        }
        let directive = chars[index];
        index += 1;
        match directive {
            '%' => out.push('%'),
            'Y' => out.push_str(&format_year(date.year)),
            'm' => out.push_str(&format!("{:02}", date.month)),
            'd' => out.push_str(&format!("{:02}", date.day)),
            'H' => out.push_str(&format!("{:02}", time.hour)),
            'M' => out.push_str(&format!("{:02}", time.minute)),
            'S' => out.push_str(&format!("{:02}", time.second)),
            'f' => out.push_str(&format!("{:06}", time.nanosecond / 1_000)),
            'N' => out.push_str(&format!("{:09}", time.nanosecond)),
            'a' => out.push_str(WEEKDAY_SHORT[weekday as usize - 1]),
            'A' => out.push_str(WEEKDAY_LONG[weekday as usize - 1]),
            'b' => out.push_str(MONTH_SHORT[date.month as usize - 1]),
            'B' => out.push_str(MONTH_LONG[date.month as usize - 1]),
            'j' => out.push_str(&format!("{ordinal_day:03}")),
            'u' => out.push_str(&weekday.to_string()),
            'V' => out.push_str(&format!("{:02}", iso_week.1)),
            'F' => out.push_str(&format!(
                "{}-{:02}-{:02}",
                format_year(date.year),
                date.month,
                date.day
            )),
            'T' => out.push_str(&format!(
                "{:02}:{:02}:{:02}",
                time.hour, time.minute, time.second
            )),
            'z' => {
                if let Some(offset) = time.offset_minutes {
                    out.push_str(&compact_offset(offset));
                }
            }
            ':' => {
                if chars.get(index) == Some(&'z') {
                    index += 1;
                    if let Some(offset) = time.offset_minutes {
                        out.push_str(&format_offset(offset));
                    }
                } else {
                    return Err(DobraError::runtime("strftime() does not support '%:' here"));
                }
            }
            'Z' => {
                if let Some(offset) = time.offset_minutes {
                    if offset == 0 {
                        out.push_str("UTC");
                    } else {
                        out.push_str("UTC");
                        out.push_str(&format_offset(offset));
                    }
                }
            }
            other => {
                return Err(DobraError::runtime(format!(
                    "strftime() does not support '%{other}'"
                )))
            }
        }
    }
    Ok(out)
}

fn parse_date_fields(text: &str) -> DobraResult<(i32, u8, u8)> {
    let mut parts = text.split('-');
    let year = parts
        .next()
        .ok_or_else(|| DobraError::runtime("date text must be YYYY-MM-DD"))?;
    let month = parts
        .next()
        .ok_or_else(|| DobraError::runtime("date text must be YYYY-MM-DD"))?;
    let day = parts
        .next()
        .ok_or_else(|| DobraError::runtime("date text must be YYYY-MM-DD"))?;
    if parts.next().is_some() {
        return Err(DobraError::runtime("date text must be YYYY-MM-DD"));
    }
    Ok((
        year.parse::<i32>()
            .map_err(|_| DobraError::runtime("invalid date year"))?,
        parse_u8(month, "date month")?,
        parse_u8(day, "date day")?,
    ))
}

fn parse_time_and_offset(text: &str) -> DobraResult<(u8, u8, u8, u32, i32)> {
    let (time_text, offset_text) = split_time_offset(text)?;
    let mut parts = time_text.split(':');
    let hour = parse_u8(
        parts
            .next()
            .ok_or_else(|| DobraError::runtime("time text must start with hour"))?,
        "time hour",
    )?;
    let minute = parse_u8(
        parts
            .next()
            .ok_or_else(|| DobraError::runtime("time text must include minute"))?,
        "time minute",
    )?;
    let second_text = parts.next().unwrap_or("00");
    if parts.next().is_some() {
        return Err(DobraError::runtime("time text has too many ':' separators"));
    }
    let (second, nanosecond) = if let Some((whole, fraction)) = second_text.split_once('.') {
        (
            parse_u8(whole, "time second")?,
            parse_fractional_nanoseconds(fraction)?,
        )
    } else {
        (parse_u8(second_text, "time second")?, 0)
    };
    let offset = offset_text.map(parse_offset_text).transpose()?.unwrap_or(0);
    Ok((hour, minute, second, nanosecond, offset))
}

fn split_time_offset(text: &str) -> DobraResult<(&str, Option<&str>)> {
    if let Some(stripped) = text.strip_suffix('Z').or_else(|| text.strip_suffix('z')) {
        return Ok((stripped, Some("Z")));
    }
    for (index, ch) in text.char_indices().skip(1) {
        if ch == '+' || ch == '-' {
            return Ok((&text[..index], Some(&text[index..])));
        }
    }
    Ok((text, None))
}

fn validate_month(month: u8) -> DobraResult<()> {
    if (1..=12).contains(&month) {
        Ok(())
    } else {
        Err(DobraError::runtime("month must be between 1 and 12"))
    }
}

fn validate_time(hour: u8, minute: u8, second: u8, nanosecond: u32) -> DobraResult<()> {
    if hour > 23 {
        return Err(DobraError::runtime("hour must be between 0 and 23"));
    }
    if minute > 59 {
        return Err(DobraError::runtime("minute must be between 0 and 59"));
    }
    if second > 59 {
        return Err(DobraError::runtime("second must be between 0 and 59"));
    }
    if nanosecond >= 1_000_000_000 {
        return Err(DobraError::runtime(
            "nanosecond must be between 0 and 999999999",
        ));
    }
    Ok(())
}

fn validate_offset(offset_minutes: i32) -> DobraResult<()> {
    if offset_minutes.abs() <= MAX_OFFSET_MINUTES {
        Ok(())
    } else {
        Err(DobraError::runtime(
            "offset must be between -23:59 and +23:59",
        ))
    }
}

fn parse_u8(text: &str, label: &str) -> DobraResult<u8> {
    text.parse::<u8>()
        .map_err(|_| DobraError::runtime(format!("invalid {label}")))
}

fn parse_fractional_nanoseconds(text: &str) -> DobraResult<u32> {
    if text.is_empty() || text.len() > 9 || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(DobraError::runtime(
            "fractional seconds must use 1 to 9 digits",
        ));
    }
    let mut digits = text.to_string();
    while digits.len() < 9 {
        digits.push('0');
    }
    digits
        .parse::<u32>()
        .map_err(|_| DobraError::runtime("invalid fractional seconds"))
}

fn parse_int_component(text: &str, label: &str) -> DobraResult<i64> {
    if text.contains('.') {
        return Err(DobraError::runtime(format!(
            "{label} duration component cannot be fractional"
        )));
    }
    text.parse::<i64>()
        .map_err(|_| DobraError::runtime(format!("invalid {label} duration component")))
}

fn parse_second_fraction(text: &str) -> DobraResult<i128> {
    if let Some((whole, fraction)) = text.split_once('.') {
        let seconds = whole
            .parse::<i64>()
            .map_err(|_| DobraError::runtime("invalid seconds duration component"))?;
        Ok(seconds as i128 * NANOS_PER_SECOND + parse_fractional_nanoseconds(fraction)? as i128)
    } else {
        Ok(text
            .parse::<i64>()
            .map_err(|_| DobraError::runtime("invalid seconds duration component"))?
            as i128
            * NANOS_PER_SECOND)
    }
}

fn checked_sum_i128(values: &[i128]) -> DobraResult<i128> {
    let mut total = 0i128;
    for value in values {
        total = total
            .checked_add(*value)
            .ok_or_else(|| DobraError::runtime("duration overflowed supported range"))?;
    }
    Ok(total)
}

fn trimmed_fraction(nanosecond: u32) -> String {
    let mut text = format!("{nanosecond:09}");
    while text.ends_with('0') {
        text.pop();
    }
    text
}

fn compact_offset(offset_minutes: i32) -> String {
    if offset_minutes == 0 {
        return "+0000".to_string();
    }
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let total = offset_minutes.abs();
    format!("{sign}{:02}{:02}", total / 60, total % 60)
}

fn format_offset(offset_minutes: i32) -> String {
    if offset_minutes == 0 {
        return "Z".to_string();
    }
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let total = offset_minutes.abs();
    format!("{sign}{:02}:{:02}", total / 60, total % 60)
}

fn format_year(year: i32) -> String {
    if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else if year < 0 {
        format!("-{:04}", year.abs())
    } else {
        format!("+{:05}", year)
    }
}

fn i32_from_i64(value: i64) -> DobraResult<i32> {
    i32::try_from(value)
        .map_err(|_| DobraError::runtime("date arithmetic overflowed supported year range"))
}

fn i32_from_i128(value: i128) -> DobraResult<i32> {
    i32::try_from(value)
        .map_err(|_| DobraError::runtime("datetime arithmetic overflowed supported year range"))
}

fn i64_from_i128(value: i128) -> DobraResult<i64> {
    i64::try_from(value).map_err(|_| DobraError::runtime("value overflowed i64 range"))
}

fn days_from_civil(year: i32, month: u8, day: u8) -> DobraResult<i32> {
    let mut year = year as i64;
    let month = month as i64;
    let day = day as i64;
    year -= if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i32_from_i64(era * 146097 + doe - 719468)
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u8, u8) {
    let z = days_since_epoch + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    (year as i32, month as u8, day as u8)
}
