use super::*;

pub(super) fn range(args: &[Value]) -> DobraResult<Value> {
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

pub(super) fn abs(args: &[Value]) -> DobraResult<Value> {
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

pub(super) fn rounded(
    args: &[Value],
    name: &str,
    op: impl FnOnce(f64) -> f64,
) -> DobraResult<Value> {
    expect_arity(&args, 1, name)?;
    Ok(Value::Int(op(to_float(&args[0])?) as i64))
}

pub(super) fn sum(args: &[Value]) -> DobraResult<Value> {
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

pub(super) fn avg(args: &[Value]) -> DobraResult<Value> {
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
