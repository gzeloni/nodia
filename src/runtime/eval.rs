// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Expression evaluation for runtime values.

use super::*;

impl Runtime {
    pub(super) fn eval(&mut self, expr: &Expr) -> NodiaResult<Value> {
        match expr {
            Expr::String { value, interpolate } => {
                if *interpolate {
                    Ok(Value::String(self.interpolate(value)?))
                } else {
                    Ok(Value::String(value.clone()))
                }
            }
            Expr::Lambda { params, body } => Ok(self.lambda_value(params, body)),
            Expr::Literal(value) => self.resolve_value(value.clone()),
            Expr::Regex(pattern) => regex::compile(pattern).map(Value::Regex),
            Expr::Identifier(name) => {
                let value = self
                    .get(name)
                    .ok_or_else(|| NodiaError::runtime(format!("undefined variable '{name}'")))?;
                self.resolve_value(value)
            }
            Expr::Unary { op, expr } => {
                let value = self.eval(expr)?;
                match op {
                    UnaryOp::Negate => match value {
                        Value::Int(value) => Ok(Value::Int(-value)),
                        Value::Float(value) => Ok(Value::Float(-value)),
                        other => Err(NodiaError::runtime(format!(
                            "cannot negate {}",
                            other.type_name()
                        ))),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!value.truthy())),
                    UnaryOp::BitNot => self.bit_not(value),
                }
            }
            Expr::Binary { left, op, right } => self.eval_binary(left, *op, right),
            Expr::Call { callee, args } => self.call(callee, args),
            Expr::Get { object, field } => {
                let object = self.eval(object)?;
                match object {
                    Value::Map(map) => {
                        let value = map.get(field).cloned().ok_or_else(|| {
                            NodiaError::runtime(format!("field '{field}' not found"))
                        })?;
                        self.resolve_value(value)
                    }
                    other => Err(NodiaError::runtime(format!(
                        "cannot access field on {}",
                        other.type_name()
                    ))),
                }
            }
            Expr::Index { object, index } => {
                let object = self.eval(object)?;
                let index = self.eval(index)?;
                self.index(object, index)
            }
            Expr::List(values) => values
                .iter()
                .map(|expr| self.eval(expr))
                .collect::<NodiaResult<Vec<_>>>()
                .map(Value::List),
            Expr::Map(pairs) => {
                let mut map = BTreeMap::new();
                for (key, value) in pairs {
                    map.insert(key.clone(), self.eval(value)?);
                }
                Ok(Value::Map(map))
            }
        }
    }

    pub(super) fn resolve_value(&self, value: Value) -> NodiaResult<Value> {
        match value {
            Value::UseBinding(module, name) => {
                module.borrow().exports.get(&name).cloned().ok_or_else(|| {
                    NodiaError::runtime(format!("used binding '{name}' is not initialized yet"))
                })
            }
            other => Ok(other),
        }
    }

    pub(super) fn eval_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
    ) -> NodiaResult<Value> {
        if op == BinaryOp::And {
            let left = self.eval(left)?;
            return if left.truthy() {
                self.eval(right)
            } else {
                Ok(Value::Bool(false))
            };
        }
        if op == BinaryOp::Or {
            let left = self.eval(left)?;
            return if left.truthy() {
                Ok(Value::Bool(true))
            } else {
                self.eval(right)
            };
        }

        let left = self.eval(left)?;
        let right = self.eval(right)?;
        match op {
            BinaryOp::Add => self.add(left, right),
            BinaryOp::Subtract => self.numeric(left, right, |a, b| a - b),
            BinaryOp::Multiply => self.numeric(left, right, |a, b| a * b),
            BinaryOp::Divide => self.divide(left, right),
            BinaryOp::Modulo => self.numeric(left, right, |a, b| a % b),
            BinaryOp::Equal => Ok(Value::Bool(left == right)),
            BinaryOp::NotEqual => Ok(Value::Bool(left != right)),
            BinaryOp::Less => self.compare(left, right, |ord| ord.is_lt()),
            BinaryOp::LessEqual => self.compare(left, right, |ord| ord.is_lt() || ord.is_eq()),
            BinaryOp::Greater => self.compare(left, right, |ord| ord.is_gt()),
            BinaryOp::GreaterEqual => self.compare(left, right, |ord| ord.is_gt() || ord.is_eq()),
            BinaryOp::BitOr => self.bitwise_binary(left, right, "bitwise or", |a, b| a | b),
            BinaryOp::BitXor => self.bitwise_binary(left, right, "bitwise xor", |a, b| a ^ b),
            BinaryOp::BitAnd => self.bitwise_binary(left, right, "bitwise and", |a, b| a & b),
            BinaryOp::ShiftLeft => self.shift_binary(left, right, "left shift", i64::checked_shl),
            BinaryOp::ShiftRight => self.shift_binary(left, right, "right shift", i64::checked_shr),
            BinaryOp::And | BinaryOp::Or => unreachable!(),
        }
    }

    pub(super) fn call(&mut self, callee: &Expr, args: &[Expr]) -> NodiaResult<Value> {
        let arg_values = args
            .iter()
            .map(|arg| self.eval(arg))
            .collect::<NodiaResult<Vec<_>>>()?;

        if let Expr::Identifier(name) = callee {
            if let Some(value) = self.get(name) {
                return self.call_value(self.resolve_value(value)?, arg_values);
            }
        }

        let callee = self.eval(callee)?;
        self.call_value(callee, arg_values)
    }

    pub(super) fn call_value(
        &mut self,
        callee: Value,
        arg_values: Vec<Value>,
    ) -> NodiaResult<Value> {
        match callee {
            Value::BuiltinFunction(name) => self
                .call_builtin_name(&name, &arg_values)?
                .ok_or_else(|| NodiaError::runtime(format!("builtin '{name}' is not callable"))),
            Value::Function(function) => self.invoke_function(&function, arg_values),
            other => Err(NodiaError::runtime(format!(
                "{} is not callable",
                other.type_name()
            ))),
        }
    }

    pub(super) fn call_builtin_name(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> NodiaResult<Option<Value>> {
        if let Some(result) = self.call_io_builtin(name, args)? {
            return Ok(Some(result));
        }
        if let Some(result) = self.call_scan_builtin(name, args)? {
            return Ok(Some(result));
        }
        if let Some(result) = self.call_runtime_builtin(name, args)? {
            return Ok(Some(result));
        }
        stdlib::call(name, args)
    }

    pub(super) fn invoke_function(
        &mut self,
        function: &Function,
        arg_values: Vec<Value>,
    ) -> NodiaResult<Value> {
        let arg_count = arg_values.len();
        self.invoke_function_args(function, arg_count, arg_values)
    }

    pub(super) fn invoke_callable1(&mut self, callee: Value, value: Value) -> NodiaResult<Value> {
        self.call_value(callee, vec![value])
    }

    pub(super) fn invoke_callable2(
        &mut self,
        callee: Value,
        left: Value,
        right: Value,
    ) -> NodiaResult<Value> {
        self.call_value(callee, vec![left, right])
    }

    fn invoke_function_args<I>(
        &mut self,
        function: &Function,
        arg_count: usize,
        arg_values: I,
    ) -> NodiaResult<Value>
    where
        I: IntoIterator<Item = Value>,
    {
        let total_params = function.params.len();
        let required_count = function.defaults.iter().filter(|d| d.is_none()).count();

        if arg_count < required_count || arg_count > total_params {
            return Err(NodiaError::runtime(format!(
                "function expects {}-{} argument(s), got {}",
                required_count, total_params, arg_count
            )));
        }

        let has_captures = !function.captures.is_empty();
        if has_captures {
            self.scopes.push(binding_scope(&function.captures));
        }
        self.scopes.push(HashMap::with_capacity(total_params));

        let mut args: Vec<Value> = arg_values.into_iter().collect();
        // Fill missing args from defaults
        while args.len() < total_params {
            let idx = args.len();
            args.push(
                function
                    .defaults
                    .get(idx)
                    .and_then(|d| d.clone())
                    .unwrap_or(Value::Null),
            );
        }

        for (name, value) in function.params.iter().zip(args) {
            self.define(name, value, true)?;
        }
        let flow = self.execute_block(&function.body)?;
        self.scopes.pop();
        if has_captures {
            self.scopes.pop();
        }
        match flow {
            Flow::Return(value) => Ok(value),
            Flow::None => Ok(Value::Null),
            Flow::Break => Err(NodiaError::runtime("break inside function without loop")),
            Flow::Continue => Err(NodiaError::runtime("continue inside function without loop")),
        }
    }
}
