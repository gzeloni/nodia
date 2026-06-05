// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Runtime helpers for argument validation, conversions, and assignment.

use super::*;

impl Runtime {
    pub(super) fn expect_arity(
        &self,
        args: &[Value],
        expected: usize,
        name: &str,
    ) -> NodiaResult<()> {
        if args.len() == expected {
            Ok(())
        } else {
            Err(NodiaError::runtime(format!(
                "{name}() expects {expected} argument(s), got {}",
                args.len()
            )))
        }
    }

    pub(super) fn expect_string(
        &self,
        value: &Value,
        name: &str,
        position: &str,
    ) -> NodiaResult<String> {
        match value {
            Value::String(value) => Ok(value.clone()),
            other => Err(NodiaError::runtime(format!(
                "{name}() expects string as {position} argument, got {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn expect_stream(
        &self,
        value: &Value,
        name: &str,
        position: &str,
    ) -> NodiaResult<StreamId> {
        match value {
            Value::Stream(stream) => Ok(*stream),
            other => Err(NodiaError::runtime(format!(
                "{name}() expects stream as {position} argument, got {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn expect_callable(
        &self,
        value: &Value,
        name: &str,
        position: &str,
    ) -> NodiaResult<Value> {
        match value {
            Value::Function(_) | Value::BuiltinFunction(_) => Ok(value.clone()),
            other => Err(NodiaError::runtime(format!(
                "{name}() expects function as {position} argument, got {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn expect_list_value<'a>(
        &self,
        value: &'a Value,
        name: &str,
        position: &str,
    ) -> NodiaResult<&'a Vec<Value>> {
        match value {
            Value::List(values) => Ok(values),
            other => Err(NodiaError::runtime(format!(
                "{name}() expects list as {position} argument, got {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn expect_non_negative_size(
        &self,
        value: &Value,
        name: &str,
        position: &str,
    ) -> NodiaResult<usize> {
        match value {
            Value::Int(value) if *value >= 0 => Ok(*value as usize),
            Value::Int(_) => Err(NodiaError::runtime(format!(
                "{name}() expects non-negative size as {position} argument"
            ))),
            other => Err(NodiaError::runtime(format!(
                "{name}() expects int as {position} argument, got {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn expect_exit_code(&self, value: &Value) -> NodiaResult<i32> {
        match value {
            Value::Int(value) if (0..=255).contains(value) => Ok(*value as i32),
            Value::Int(_) => Err(NodiaError::runtime(
                "exit() expects an int status between 0 and 255",
            )),
            other => Err(NodiaError::runtime(format!(
                "exit() expects int as first argument, got {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn add(&self, left: Value, right: Value) -> NodiaResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::String(a), b) => Ok(Value::String(a + &b.to_string())),
            (a, Value::String(b)) => Ok(Value::String(a.to_string() + &b)),
            (a, b) => Err(NodiaError::runtime(format!(
                "cannot add {} and {}",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    pub(super) fn divide(&self, left: Value, right: Value) -> NodiaResult<Value> {
        Ok(Value::Float(to_number(&left)? / to_number(&right)?))
    }

    pub(super) fn numeric(
        &self,
        left: Value,
        right: Value,
        op: impl FnOnce(f64, f64) -> f64,
    ) -> NodiaResult<Value> {
        let left_float = to_number(&left)?;
        let right_float = to_number(&right)?;
        let result = op(left_float, right_float);
        if matches!(left, Value::Int(_)) && matches!(right, Value::Int(_)) && result.fract() == 0.0
        {
            Ok(Value::Int(result as i64))
        } else {
            Ok(Value::Float(result))
        }
    }

    pub(super) fn compare(
        &self,
        left: Value,
        right: Value,
        f: impl FnOnce(std::cmp::Ordering) -> bool,
    ) -> NodiaResult<Value> {
        let ordering = match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Date(a), Value::Date(b)) => a.cmp(b),
            (Value::DateTime(a), Value::DateTime(b)) => a.cmp(b),
            (Value::Duration(a), Value::Duration(b)) => a.cmp(b),
            _ => {
                let a = to_number(&left)?;
                let b = to_number(&right)?;
                a.partial_cmp(&b)
                    .ok_or_else(|| NodiaError::runtime("cannot compare NaN"))?
            }
        };
        Ok(Value::Bool(f(ordering)))
    }

    pub(super) fn index(&self, object: Value, index: Value) -> NodiaResult<Value> {
        match object {
            Value::List(values) => {
                let index = match index {
                    Value::Int(value) => value,
                    other => {
                        return Err(NodiaError::runtime(format!(
                            "list index must be int, got {}",
                            other.type_name()
                        )))
                    }
                };
                let normalized = if index < 0 {
                    values.len() as i64 + index
                } else {
                    index
                };
                values
                    .get(normalized as usize)
                    .cloned()
                    .ok_or_else(|| NodiaError::runtime("list index out of bounds"))
            }
            Value::String(value) => {
                let index = match index {
                    Value::Int(value) => value,
                    other => {
                        return Err(NodiaError::runtime(format!(
                            "string index must be int, got {}",
                            other.type_name()
                        )))
                    }
                };
                value
                    .chars()
                    .nth(index as usize)
                    .map(|ch| Value::String(ch.to_string()))
                    .ok_or_else(|| NodiaError::runtime("string index out of bounds"))
            }
            Value::Map(values) => {
                let key = index.to_string();
                let value = values
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| NodiaError::runtime(format!("key '{key}' not found")))?;
                self.resolve_value(value)
            }
            other => Err(NodiaError::runtime(format!(
                "cannot index {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn interpolate(&mut self, raw: &str) -> NodiaResult<String> {
        let mut output = String::with_capacity(raw.len());
        let bytes = raw.as_bytes();
        let mut index = 0;
        while index < raw.len() {
            if bytes[index] == b'{' {
                if bytes.get(index + 1) == Some(&b'{') {
                    output.push('{');
                    index += 2;
                    continue;
                }
                let start = index + 1;
                let Some(offset) = raw[start..].find('}') else {
                    return Err(NodiaError::runtime("unterminated interpolation"));
                };
                let end = start + offset;
                let tokens = Lexer::new(&raw[start..end]).tokenize()?;
                let expr = Parser::new(tokens).parse_expression_only()?;
                let value = self.eval(&expr)?;
                output.push_str(&value.to_string());
                index = end + 1;
            } else if bytes[index] == b'}' && bytes.get(index + 1) == Some(&b'}') {
                output.push('}');
                index += 2;
            } else {
                let next = raw[index..]
                    .chars()
                    .next()
                    .map(|ch| index + ch.len_utf8())
                    .expect("index always points to a char boundary");
                output.push_str(&raw[index..next]);
                index = next;
            }
        }
        Ok(output)
    }

    pub(super) fn define(&mut self, name: &str, value: Value, mutable: bool) -> NodiaResult<()> {
        let scope = self.scopes.last_mut().expect("runtime always has a scope");
        if scope.contains_key(name) {
            return Err(NodiaError::runtime(format!(
                "'{name}' is already defined in this scope"
            )));
        }
        scope.insert(name.to_string(), binding_ref(value, mutable));
        Ok(())
    }

    pub(super) fn assign(&mut self, name: &str, value: Value) -> NodiaResult<()> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                let mut binding = binding.borrow_mut();
                if let Value::UseBinding(module, export_name) = binding.value.clone() {
                    return assign_use_binding(module, &export_name, value);
                }
                if !binding.mutable {
                    return Err(NodiaError::runtime(format!(
                        "cannot assign to val '{name}'"
                    )));
                }
                binding.value = value;
                return Ok(());
            }
        }
        Err(NodiaError::runtime(format!("undefined variable '{name}'")))
    }

    pub(super) fn assign_target(&mut self, target: &AssignTarget, value: Value) -> NodiaResult<()> {
        let (root, steps) = self.resolve_target(target)?;
        let current = self
            .get(&root)
            .ok_or_else(|| NodiaError::runtime(format!("undefined variable '{root}'")))?;
        if let Some(updated) = self.update_value_path(current, &steps, value)? {
            self.assign(&root, updated)?;
        }
        Ok(())
    }

    pub(super) fn resolve_target(
        &mut self,
        target: &AssignTarget,
    ) -> NodiaResult<(String, Vec<TargetStep>)> {
        match target {
            AssignTarget::Identifier(name) => Ok((name.clone(), Vec::new())),
            AssignTarget::Get { object, field } => {
                let (root, mut steps) = self.resolve_target(object)?;
                steps.push(TargetStep::Field(field.clone()));
                Ok((root, steps))
            }
            AssignTarget::Index { object, index } => {
                let (root, mut steps) = self.resolve_target(object)?;
                steps.push(TargetStep::Index(self.eval(index)?));
                Ok((root, steps))
            }
        }
    }

    pub(super) fn update_value_path(
        &mut self,
        current: Value,
        steps: &[TargetStep],
        new_value: Value,
    ) -> NodiaResult<Option<Value>> {
        if let Value::UseBinding(module, name) = current.clone() {
            if steps.is_empty() {
                assign_use_binding(module, &name, new_value)?;
                return Ok(None);
            }
            let resolved = self.resolve_value(current)?;
            if let Some(updated) = self.update_value_path(resolved, steps, new_value)? {
                assign_use_binding(module, &name, updated)?;
            }
            return Ok(None);
        }

        if steps.is_empty() {
            return Ok(Some(new_value));
        }

        let (step, rest) = (&steps[0], &steps[1..]);
        match (current, step) {
            (Value::Map(mut map), TargetStep::Field(field)) => {
                if rest.is_empty() {
                    map.insert(field.clone(), new_value);
                    return Ok(Some(Value::Map(map)));
                }
                let child = map
                    .get(field)
                    .cloned()
                    .ok_or_else(|| NodiaError::runtime(format!("field '{field}' not found")))?;
                match self.update_value_path(child, rest, new_value)? {
                    Some(updated) => {
                        map.insert(field.clone(), updated);
                        Ok(Some(Value::Map(map)))
                    }
                    None => Ok(None),
                }
            }
            (Value::Map(mut map), TargetStep::Index(index)) => {
                let key = index.to_string();
                if rest.is_empty() {
                    map.insert(key, new_value);
                    return Ok(Some(Value::Map(map)));
                }
                let child = map
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| NodiaError::runtime(format!("key '{key}' not found")))?;
                match self.update_value_path(child, rest, new_value)? {
                    Some(updated) => {
                        map.insert(key, updated);
                        Ok(Some(Value::Map(map)))
                    }
                    None => Ok(None),
                }
            }
            (Value::List(mut values), TargetStep::Index(index)) => {
                let index = self.normalize_list_index(values.len(), index)?;
                if rest.is_empty() {
                    values[index] = new_value;
                    return Ok(Some(Value::List(values)));
                }
                let child = values[index].clone();
                match self.update_value_path(child, rest, new_value)? {
                    Some(updated) => {
                        values[index] = updated;
                        Ok(Some(Value::List(values)))
                    }
                    None => Ok(None),
                }
            }
            (Value::String(_), TargetStep::Index(_)) => {
                Err(NodiaError::runtime("cannot assign through string index"))
            }
            (other, TargetStep::Field(_)) => Err(NodiaError::runtime(format!(
                "cannot assign field on {}",
                other.type_name()
            ))),
            (other, TargetStep::Index(_)) => Err(NodiaError::runtime(format!(
                "cannot index {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn normalize_list_index(&self, len: usize, index: &Value) -> NodiaResult<usize> {
        let index = match index {
            Value::Int(value) => *value,
            other => {
                return Err(NodiaError::runtime(format!(
                    "list index must be int, got {}",
                    other.type_name()
                )))
            }
        };
        let normalized = if index < 0 { len as i64 + index } else { index };
        if normalized < 0 || normalized as usize >= len {
            return Err(NodiaError::runtime("list index out of bounds"));
        }
        Ok(normalized as usize)
    }

    pub(super) fn iterable_values(
        &self,
        binding: &ForBinding,
        iterable: Value,
    ) -> NodiaResult<Vec<Vec<(String, Value)>>> {
        match binding {
            ForBinding::Single(name) => Ok(self
                .iterable_single_values(iterable)?
                .into_iter()
                .map(|value| vec![(name.clone(), value)])
                .collect()),
            ForBinding::Pair { key, value } => self.iterable_pair_values(iterable, key, value),
        }
    }

    pub(super) fn iterable_single_values(&self, iterable: Value) -> NodiaResult<Vec<Value>> {
        match iterable {
            Value::List(values) => Ok(values),
            Value::String(value) => Ok(value
                .chars()
                .map(|ch| Value::String(ch.to_string()))
                .collect()),
            Value::Map(value) => Ok(value.keys().cloned().map(Value::String).collect()),
            other => Err(NodiaError::runtime(format!(
                "cannot iterate over {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn iterable_pair_values(
        &self,
        iterable: Value,
        key_name: &str,
        value_name: &str,
    ) -> NodiaResult<Vec<Vec<(String, Value)>>> {
        match iterable {
            Value::Map(values) => Ok(values
                .into_iter()
                .map(|(key, value)| {
                    vec![
                        (key_name.to_string(), Value::String(key)),
                        (value_name.to_string(), value),
                    ]
                })
                .collect()),
            Value::List(values) => values
                .into_iter()
                .map(|value| {
                    let (key, value) = self.destructure_pair(value)?;
                    Ok(vec![
                        (key_name.to_string(), key),
                        (value_name.to_string(), value),
                    ])
                })
                .collect(),
            other => Err(NodiaError::runtime(format!(
                "cannot destructure iteration over {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn destructure_pair(&self, value: Value) -> NodiaResult<(Value, Value)> {
        match value {
            Value::List(values) => match values.as_slice() {
                [key, value] => Ok((key.clone(), value.clone())),
                _ => Err(NodiaError::runtime(
                    "pair iteration expects list items with exactly 2 values",
                )),
            },
            Value::Map(values) => {
                let key = values.get("key").cloned();
                let value = values.get("value").cloned();
                match (key, value) {
                    (Some(key), Some(value)) => Ok((key, value)),
                    _ => Err(NodiaError::runtime(
                        "pair iteration expects map items with 'key' and 'value'",
                    )),
                }
            }
            other => Err(NodiaError::runtime(format!(
                "cannot destructure pair from {}",
                other.type_name()
            ))),
        }
    }

    pub(super) fn get(&self, name: &str) -> Option<Value> {
        self.scopes.iter().rev().find_map(|scope| {
            scope
                .get(name)
                .map(|binding| binding.borrow().value.clone())
        })
    }

    pub(super) fn builtin_value(&self, name: &str) -> Option<Value> {
        match name {
            "args" => Some(Value::List(
                self.options
                    .args
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            )),
            "stdin" => Some(Value::Stream(StreamId::Stdin)),
            "stdout" => Some(Value::Stream(StreamId::Stdout)),
            "stderr" => Some(Value::Stream(StreamId::Stderr)),
            _ => stdlib::global_builtin_item(name).and_then(|(_, export_name, arities)| {
                arities.map(|_| Value::BuiltinFunction(export_name.to_string()))
            }),
        }
    }

    pub(super) fn root_get(&self, name: &str) -> Option<Value> {
        self.scopes.first().and_then(|scope| {
            scope
                .get(name)
                .map(|binding| binding.borrow().value.clone())
        })
    }
}
