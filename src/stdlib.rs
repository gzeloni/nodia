use crate::error::{DobraError, DobraResult};
use crate::regex::{self, RegexMatch, RuntimeRegex};
use crate::value::Value;
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub fn call(name: &str, args: Vec<Value>) -> DobraResult<Option<Value>> {
    let result = match name {
        "upper" | "uppercase" => unary_string(args, name, |s| s.to_uppercase())?,
        "lower" | "lowercase" => unary_string(args, name, |s| s.to_lowercase())?,
        "capitalize" => unary_string(args, "capitalize", |s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })?,
        "trim" => unary_string(args, "trim", |s| s.trim().to_string())?,
        "replace" => {
            expect_arity(&args, 3, "replace")?;
            Value::String(
                args[0]
                    .to_string()
                    .replace(&args[1].to_string(), &args[2].to_string()),
            )
        }
        "split" => {
            expect_arity(&args, 2, "split")?;
            Value::List(
                args[0]
                    .to_string()
                    .split(&args[1].to_string())
                    .map(|part| Value::String(part.to_string()))
                    .collect(),
            )
        }
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
        "test" => regex_test(args)?,
        "full_match" => regex_full_match(args)?,
        "find" => regex_find(args)?,
        "find_all" => regex_find_all(args)?,
        "contains" => {
            expect_arity(&args, 2, "contains")?;
            Value::Bool(match &args[0] {
                Value::String(value) => value.contains(&args[1].to_string()),
                Value::List(values) => values.contains(&args[1]),
                Value::Map(values) => values.contains_key(&args[1].to_string()),
                other => {
                    return Err(DobraError::runtime(format!(
                        "contains() does not accept {}",
                        other.type_name()
                    )));
                }
            })
        }
        "starts" | "starts_with" => {
            expect_arity(&args, 2, name)?;
            Value::Bool(args[0].to_string().starts_with(&args[1].to_string()))
        }
        "ends" | "ends_with" => {
            expect_arity(&args, 2, name)?;
            Value::Bool(args[0].to_string().ends_with(&args[1].to_string()))
        }
        "indent" => indent(args)?,
        "dedent" => {
            expect_arity(&args, 1, "dedent")?;
            Value::String(dedent(&args[0].to_string()))
        }
        "keys" => {
            expect_arity(&args, 1, "keys")?;
            let Value::Map(values) = &args[0] else {
                return Err(DobraError::runtime(format!(
                    "keys() expects map, got {}",
                    args[0].type_name()
                )));
            };
            Value::List(values.keys().cloned().map(Value::String).collect())
        }
        "values" => {
            expect_arity(&args, 1, "values")?;
            let Value::Map(values) = &args[0] else {
                return Err(DobraError::runtime(format!(
                    "values() expects map, got {}",
                    args[0].type_name()
                )));
            };
            Value::List(values.values().cloned().collect())
        }
        "len" => {
            expect_arity(&args, 1, "len")?;
            let len = match &args[0] {
                Value::String(value) => value.chars().count(),
                Value::List(value) => value.len(),
                Value::Map(value) => value.len(),
                other => {
                    return Err(DobraError::runtime(format!(
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
        "bool" => {
            expect_arity(&args, 1, "bool")?;
            Value::Bool(args[0].truthy())
        }
        "range" => range(args)?,
        "abs" => abs(args)?,
        "floor" => rounded(args, "floor", f64::floor)?,
        "ceil" => rounded(args, "ceil", f64::ceil)?,
        "round" => rounded(args, "round", f64::round)?,
        "sqrt" => {
            expect_arity(&args, 1, "sqrt")?;
            Value::Float(to_float(&args[0])?.sqrt())
        }
        "pow" => {
            expect_arity(&args, 2, "pow")?;
            number_result(to_float(&args[0])?.powf(to_float(&args[1])?), &args)
        }
        "min" => {
            expect_arity(&args, 2, "min")?;
            let a = to_float(&args[0])?;
            let b = to_float(&args[1])?;
            number_result(a.min(b), &args)
        }
        "max" => {
            expect_arity(&args, 2, "max")?;
            let a = to_float(&args[0])?;
            let b = to_float(&args[1])?;
            number_result(a.max(b), &args)
        }
        "clamp" => {
            expect_arity(&args, 3, "clamp")?;
            let value = to_float(&args[0])?;
            let min = to_float(&args[1])?;
            let max = to_float(&args[2])?;
            if min > max {
                return Err(DobraError::runtime(
                    "clamp() min cannot be greater than max",
                ));
            }
            number_result(value.clamp(min, max), &args)
        }
        "sum" => sum(args)?,
        "avg" => avg(args)?,
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
        "slice" => slice(args)?,
        "reverse" => reverse(args)?,
        "sort" => sort(args)?,
        "unique" => unique(args)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn unary_string(
    args: Vec<Value>,
    name: &str,
    f: impl FnOnce(String) -> String,
) -> DobraResult<Value> {
    expect_arity(&args, 1, name)?;
    Ok(Value::String(f(args[0].to_string())))
}

fn regex_test(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 2, "test")?;
    let pattern = expect_regex(&args[1], "test", "second")?;
    Ok(Value::Bool(pattern.is_match(&args[0].to_string())?))
}

fn regex_full_match(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 2, "full_match")?;
    let pattern = expect_regex(&args[1], "full_match", "second")?;
    Ok(Value::Bool(pattern.is_full_match(&args[0].to_string())?))
}

fn regex_find(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 2, "find")?;
    let text = args[0].to_string();
    let pattern = expect_regex(&args[1], "find", "second")?;
    Ok(pattern
        .find(&text)?
        .map(regex_match_value)
        .unwrap_or(Value::Null))
}

fn regex_find_all(args: Vec<Value>) -> DobraResult<Value> {
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

fn indent(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 2, "indent")?;
    let text = args[0].to_string();
    let prefix = match &args[1] {
        Value::Int(size) => " ".repeat((*size).max(0) as usize),
        other => other.to_string(),
    };
    Ok(Value::String(
        text.lines()
            .map(|line| format!("{prefix}{line}"))
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

fn dedent(text: &str) -> String {
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

fn range(args: Vec<Value>) -> DobraResult<Value> {
    if args.len() != 1 && args.len() != 2 {
        return Err(DobraError::runtime("range() expects 1 or 2 arguments"));
    }
    let (start, end) = if args.len() == 1 {
        (0, to_int(&args[0])?)
    } else {
        (to_int(&args[0])?, to_int(&args[1])?)
    };
    let values = if start <= end {
        (start..end).map(Value::Int).collect()
    } else {
        (end + 1..=start).rev().map(Value::Int).collect()
    };
    Ok(Value::List(values))
}

fn abs(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 1, "abs")?;
    match &args[0] {
        Value::Int(value) => value
            .checked_abs()
            .map(Value::Int)
            .ok_or_else(|| DobraError::runtime("abs() integer overflow")),
        Value::Float(value) => Ok(Value::Float(value.abs())),
        other => Err(DobraError::runtime(format!(
            "abs() expects number, got {}",
            other.type_name()
        ))),
    }
}

fn rounded(args: Vec<Value>, name: &str, op: impl FnOnce(f64) -> f64) -> DobraResult<Value> {
    expect_arity(&args, 1, name)?;
    Ok(Value::Int(op(to_float(&args[0])?) as i64))
}

fn sum(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 1, "sum")?;
    let values = expect_list(&args[0], "sum", "first")?;
    let mut total = 0.0;
    let mut all_ints = true;
    for value in values {
        if !matches!(value, Value::Int(_)) {
            all_ints = false;
        }
        total += to_float(value)?;
    }
    if all_ints {
        Ok(Value::Int(total as i64))
    } else {
        Ok(Value::Float(total))
    }
}

fn avg(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 1, "avg")?;
    let values = expect_list(&args[0], "avg", "first")?;
    if values.is_empty() {
        return Ok(Value::Null);
    }
    let mut total = 0.0;
    for value in values {
        total += to_float(value)?;
    }
    Ok(Value::Float(total / values.len() as f64))
}

fn slice(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 3, "slice")?;
    let start = to_int(&args[1])?;
    let end = to_int(&args[2])?;
    match &args[0] {
        Value::List(values) => {
            let (start, end) = normalize_bounds(values.len(), start, end);
            Ok(Value::List(values[start..end].to_vec()))
        }
        Value::String(value) => {
            let chars = value.chars().collect::<Vec<_>>();
            let (start, end) = normalize_bounds(chars.len(), start, end);
            Ok(Value::String(chars[start..end].iter().collect()))
        }
        other => Err(DobraError::runtime(format!(
            "slice() expects list or string, got {}",
            other.type_name()
        ))),
    }
}

fn reverse(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 1, "reverse")?;
    match &args[0] {
        Value::List(values) => {
            let mut values = values.clone();
            values.reverse();
            Ok(Value::List(values))
        }
        Value::String(value) => Ok(Value::String(value.chars().rev().collect())),
        other => Err(DobraError::runtime(format!(
            "reverse() expects list or string, got {}",
            other.type_name()
        ))),
    }
}

fn sort(args: Vec<Value>) -> DobraResult<Value> {
    expect_arity(&args, 1, "sort")?;
    let mut values = expect_list(&args[0], "sort", "first")?.clone();
    values.sort_by(compare_values);
    Ok(Value::List(values))
}

fn unique(args: Vec<Value>) -> DobraResult<Value> {
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

fn expect_arity(args: &[Value], expected: usize, name: &str) -> DobraResult<()> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(DobraError::runtime(format!(
            "{name}() expects {expected} argument(s), got {}",
            args.len()
        )))
    }
}

fn expect_list<'a>(value: &'a Value, name: &str, position: &str) -> DobraResult<&'a Vec<Value>> {
    let Value::List(values) = value else {
        return Err(DobraError::runtime(format!(
            "{name}() expects list as {position} argument, got {}",
            value.type_name()
        )));
    };
    Ok(values)
}

fn to_int(value: &Value) -> DobraResult<i64> {
    match value {
        Value::Int(value) => Ok(*value),
        Value::Float(value) => Ok(*value as i64),
        Value::String(value) => value
            .parse::<i64>()
            .map_err(|_| DobraError::runtime(format!("cannot convert '{value}' to int"))),
        other => Err(DobraError::runtime(format!(
            "cannot convert {} to int",
            other.type_name()
        ))),
    }
}

fn to_float(value: &Value) -> DobraResult<f64> {
    match value {
        Value::Int(value) => Ok(*value as f64),
        Value::Float(value) => Ok(*value),
        Value::String(value) => value
            .parse::<f64>()
            .map_err(|_| DobraError::runtime(format!("cannot convert '{value}' to float"))),
        other => Err(DobraError::runtime(format!(
            "cannot convert {} to float",
            other.type_name()
        ))),
    }
}

fn number_result(value: f64, args: &[Value]) -> Value {
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

fn compare_values(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal),
        (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal),
        (Value::String(a), Value::String(b)) => a.cmp(b),
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
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
        Value::List(_) => 4,
        Value::Map(_) => 5,
        Value::Regex(_) => 6,
        Value::Stream(_) => 7,
        Value::UseBinding(_, _) => 8,
        Value::Function(_) => 9,
    }
}
