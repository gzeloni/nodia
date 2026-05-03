use crate::error::{OrichError, OrichResult};
use crate::value::Value;

pub fn call(name: &str, args: Vec<Value>) -> OrichResult<Option<Value>> {
    let result = match name {
        "uppercase" => unary_string(args, "uppercase", |s| s.to_uppercase())?,
        "lowercase" => unary_string(args, "lowercase", |s| s.to_lowercase())?,
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
            let Value::List(values) = &args[0] else {
                return Err(OrichError::runtime(format!(
                    "join() expects list as first argument, got {}",
                    args[0].type_name()
                )));
            };
            Value::String(
                values
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join(&args[1].to_string()),
            )
        }
        "contains" => {
            expect_arity(&args, 2, "contains")?;
            Value::Bool(match &args[0] {
                Value::String(value) => value.contains(&args[1].to_string()),
                Value::List(values) => values.contains(&args[1]),
                Value::Map(values) => values.contains_key(&args[1].to_string()),
                other => {
                    return Err(OrichError::runtime(format!(
                        "contains() does not accept {}",
                        other.type_name()
                    )));
                }
            })
        }
        "starts_with" => {
            expect_arity(&args, 2, "starts_with")?;
            Value::Bool(args[0].to_string().starts_with(&args[1].to_string()))
        }
        "ends_with" => {
            expect_arity(&args, 2, "ends_with")?;
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
                return Err(OrichError::runtime(format!(
                    "keys() expects map, got {}",
                    args[0].type_name()
                )));
            };
            Value::List(values.keys().cloned().map(Value::String).collect())
        }
        "values" => {
            expect_arity(&args, 1, "values")?;
            let Value::Map(values) = &args[0] else {
                return Err(OrichError::runtime(format!(
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
                    return Err(OrichError::runtime(format!(
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
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn unary_string(
    args: Vec<Value>,
    name: &str,
    f: impl FnOnce(String) -> String,
) -> OrichResult<Value> {
    expect_arity(&args, 1, name)?;
    Ok(Value::String(f(args[0].to_string())))
}

fn indent(args: Vec<Value>) -> OrichResult<Value> {
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

fn range(args: Vec<Value>) -> OrichResult<Value> {
    if args.len() != 1 && args.len() != 2 {
        return Err(OrichError::runtime("range() expects 1 or 2 arguments"));
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

fn expect_arity(args: &[Value], expected: usize, name: &str) -> OrichResult<()> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(OrichError::runtime(format!(
            "{name}() expects {expected} argument(s), got {}",
            args.len()
        )))
    }
}

fn to_int(value: &Value) -> OrichResult<i64> {
    match value {
        Value::Int(value) => Ok(*value),
        Value::Float(value) => Ok(*value as i64),
        Value::String(value) => value
            .parse::<i64>()
            .map_err(|_| OrichError::runtime(format!("cannot convert '{value}' to int"))),
        other => Err(OrichError::runtime(format!(
            "cannot convert {} to int",
            other.type_name()
        ))),
    }
}

fn to_float(value: &Value) -> OrichResult<f64> {
    match value {
        Value::Int(value) => Ok(*value as f64),
        Value::Float(value) => Ok(*value),
        Value::String(value) => value
            .parse::<f64>()
            .map_err(|_| OrichError::runtime(format!("cannot convert '{value}' to float"))),
        other => Err(OrichError::runtime(format!(
            "cannot convert {} to float",
            other.type_name()
        ))),
    }
}
