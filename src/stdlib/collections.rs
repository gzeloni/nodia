use super::to_int;
use crate::error::{DobraError, DobraResult};
use crate::value::Value;

pub fn get(args: &[Value]) -> DobraResult<Value> {
    if args.len() != 3 {
        return Err(DobraError::runtime(format!(
            "get() expects 3 argument(s), got {}",
            args.len()
        )));
    }

    let default = args[2].clone();
    match &args[0] {
        Value::Map(values) => Ok(values.get(&args[1].to_string()).cloned().unwrap_or(default)),
        Value::List(values) => {
            let index = normalize_index(values.len(), to_int(&args[1])?);
            Ok(index
                .and_then(|index| values.get(index).cloned())
                .unwrap_or(default))
        }
        Value::String(value) => {
            let index = to_int(&args[1])?;
            let index = if index < 0 {
                normalize_index(value.chars().count(), index)
            } else {
                Some(index as usize)
            };
            Ok(index
                .and_then(|index| value.chars().nth(index))
                .map(|ch| Value::String(ch.to_string()))
                .unwrap_or(default))
        }
        other => Err(DobraError::runtime(format!(
            "get() expects map, list or string as first argument, got {}",
            other.type_name()
        ))),
    }
}

fn normalize_index(len: usize, index: i64) -> Option<usize> {
    let len = len as i64;
    let index = if index < 0 { len + index } else { index };
    if index < 0 || index >= len {
        None
    } else {
        Some(index as usize)
    }
}
