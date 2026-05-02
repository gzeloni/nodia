use crate::ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::error::{OrichError, OrichResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::stdlib;
use crate::value::{Function, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
struct Binding {
    value: Value,
    mutable: bool,
}

enum Flow {
    None,
    Return(Value),
    Break,
    Continue,
}

pub struct Runtime {
    scopes: Vec<HashMap<String, Binding>>,
    output: String,
}

impl Runtime {
    pub fn new(input: BTreeMap<String, Value>) -> Self {
        let mut root = HashMap::new();
        root.insert(
            "input".to_string(),
            Binding {
                value: Value::Map(input),
                mutable: false,
            },
        );
        Self {
            scopes: vec![root],
            output: String::new(),
        }
    }

    pub fn run(&mut self, program: &Program) -> OrichResult<String> {
        for statement in &program.statements {
            match self.execute(statement)? {
                Flow::None => {}
                Flow::Return(_) => return Err(OrichError::runtime("return outside function")),
                Flow::Break => return Err(OrichError::runtime("break outside loop")),
                Flow::Continue => return Err(OrichError::runtime("continue outside loop")),
            }
        }
        Ok(self.output.trim_end_matches('\n').to_string())
    }

    fn execute_block(&mut self, statements: &[Stmt]) -> OrichResult<Flow> {
        self.scopes.push(HashMap::new());
        for statement in statements {
            let flow = self.execute(statement)?;
            if !matches!(flow, Flow::None) {
                self.scopes.pop();
                return Ok(flow);
            }
        }
        self.scopes.pop();
        Ok(Flow::None)
    }

    fn execute(&mut self, statement: &Stmt) -> OrichResult<Flow> {
        match statement {
            Stmt::Let {
                name,
                value,
                mutable,
            } => {
                let value = self.eval(value)?;
                self.define(name, value, *mutable)?;
                Ok(Flow::None)
            }
            Stmt::Assign { name, value } => {
                let value = self.eval(value)?;
                self.assign(name, value)?;
                Ok(Flow::None)
            }
            Stmt::Fn { name, params, body } => {
                self.define(
                    name,
                    Value::Function(Function {
                        params: params.clone(),
                        body: body.clone(),
                    }),
                    false,
                )?;
                Ok(Flow::None)
            }
            Stmt::Return(value) => Ok(Flow::Return(match value {
                Some(expr) => self.eval(expr)?,
                None => Value::Null,
            })),
            Stmt::Emit(expr) => {
                let value = self.eval(expr)?;
                self.output.push_str(&value.to_string());
                self.output.push('\n');
                Ok(Flow::None)
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.eval(condition)?.truthy() {
                    self.execute_block(then_branch)
                } else {
                    self.execute_block(else_branch)
                }
            }
            Stmt::For {
                name,
                iterable,
                body,
            } => {
                let iterable = self.eval(iterable)?;
                let values = match iterable {
                    Value::List(values) => values,
                    Value::String(value) => value
                        .chars()
                        .map(|ch| Value::String(ch.to_string()))
                        .collect(),
                    Value::Map(value) => value.keys().cloned().map(Value::String).collect(),
                    other => {
                        return Err(OrichError::runtime(format!(
                            "cannot iterate over {}",
                            other.type_name()
                        )))
                    }
                };
                for value in values {
                    self.scopes.push(HashMap::new());
                    self.define(name, value, true)?;
                    let flow = self.execute_block(body)?;
                    self.scopes.pop();
                    match flow {
                        Flow::None | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                    }
                }
                Ok(Flow::None)
            }
            Stmt::While { condition, body } => {
                let mut iterations = 0usize;
                while self.eval(condition)?.truthy() {
                    iterations += 1;
                    if iterations > 100_000 {
                        return Err(OrichError::runtime("while loop exceeded 100000 iterations"));
                    }
                    match self.execute_block(body)? {
                        Flow::None | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                    }
                }
                Ok(Flow::None)
            }
            Stmt::Break => Ok(Flow::Break),
            Stmt::Continue => Ok(Flow::Continue),
            Stmt::Expr(expr) => {
                self.eval(expr)?;
                Ok(Flow::None)
            }
        }
    }

    fn eval(&mut self, expr: &Expr) -> OrichResult<Value> {
        match expr {
            Expr::Literal(Value::String(value)) => Ok(Value::String(self.interpolate(value)?)),
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Identifier(name) => self
                .get(name)
                .ok_or_else(|| OrichError::runtime(format!("undefined variable '{name}'"))),
            Expr::Unary { op, expr } => {
                let value = self.eval(expr)?;
                match op {
                    UnaryOp::Negate => match value {
                        Value::Int(value) => Ok(Value::Int(-value)),
                        Value::Float(value) => Ok(Value::Float(-value)),
                        other => Err(OrichError::runtime(format!(
                            "cannot negate {}",
                            other.type_name()
                        ))),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!value.truthy())),
                }
            }
            Expr::Binary { left, op, right } => self.eval_binary(left, *op, right),
            Expr::Call { callee, args } => self.call(callee, args),
            Expr::Get { object, field } => {
                let object = self.eval(object)?;
                match object {
                    Value::Map(map) => map
                        .get(field)
                        .cloned()
                        .ok_or_else(|| OrichError::runtime(format!("field '{field}' not found"))),
                    other => Err(OrichError::runtime(format!(
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
                .collect::<OrichResult<Vec<_>>>()
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

    fn eval_binary(&mut self, left: &Expr, op: BinaryOp, right: &Expr) -> OrichResult<Value> {
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
            BinaryOp::Divide => self.numeric(left, right, |a, b| a / b),
            BinaryOp::Modulo => self.numeric(left, right, |a, b| a % b),
            BinaryOp::Equal => Ok(Value::Bool(left == right)),
            BinaryOp::NotEqual => Ok(Value::Bool(left != right)),
            BinaryOp::Less => self.compare(left, right, |ord| ord.is_lt()),
            BinaryOp::LessEqual => self.compare(left, right, |ord| ord.is_lt() || ord.is_eq()),
            BinaryOp::Greater => self.compare(left, right, |ord| ord.is_gt()),
            BinaryOp::GreaterEqual => self.compare(left, right, |ord| ord.is_gt() || ord.is_eq()),
            BinaryOp::And | BinaryOp::Or => unreachable!(),
        }
    }

    fn call(&mut self, callee: &Expr, args: &[Expr]) -> OrichResult<Value> {
        let arg_values = args
            .iter()
            .map(|arg| self.eval(arg))
            .collect::<OrichResult<Vec<_>>>()?;

        if let Expr::Identifier(name) = callee {
            if let Some(result) = stdlib::call(name, arg_values.clone())? {
                return Ok(result);
            }
        }

        let callee = self.eval(callee)?;
        match callee {
            Value::Function(function) => {
                if function.params.len() != arg_values.len() {
                    return Err(OrichError::runtime(format!(
                        "function expects {} argument(s), got {}",
                        function.params.len(),
                        arg_values.len()
                    )));
                }
                self.scopes.push(HashMap::new());
                for (name, value) in function.params.iter().zip(arg_values) {
                    self.define(name, value, true)?;
                }
                let flow = self.execute_block(&function.body)?;
                self.scopes.pop();
                match flow {
                    Flow::Return(value) => Ok(value),
                    Flow::None => Ok(Value::Null),
                    Flow::Break => Err(OrichError::runtime("break inside function without loop")),
                    Flow::Continue => {
                        Err(OrichError::runtime("continue inside function without loop"))
                    }
                }
            }
            other => Err(OrichError::runtime(format!(
                "{} is not callable",
                other.type_name()
            ))),
        }
    }

    fn add(&self, left: Value, right: Value) -> OrichResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::String(a), b) => Ok(Value::String(a + &b.to_string())),
            (a, Value::String(b)) => Ok(Value::String(a.to_string() + &b)),
            (a, b) => Err(OrichError::runtime(format!(
                "cannot add {} and {}",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn numeric(
        &self,
        left: Value,
        right: Value,
        op: impl FnOnce(f64, f64) -> f64,
    ) -> OrichResult<Value> {
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

    fn compare(
        &self,
        left: Value,
        right: Value,
        f: impl FnOnce(std::cmp::Ordering) -> bool,
    ) -> OrichResult<Value> {
        let ordering = match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            _ => {
                let a = to_number(&left)?;
                let b = to_number(&right)?;
                a.partial_cmp(&b)
                    .ok_or_else(|| OrichError::runtime("cannot compare NaN"))?
            }
        };
        Ok(Value::Bool(f(ordering)))
    }

    fn index(&self, object: Value, index: Value) -> OrichResult<Value> {
        match object {
            Value::List(values) => {
                let index = match index {
                    Value::Int(value) => value,
                    other => {
                        return Err(OrichError::runtime(format!(
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
                    .ok_or_else(|| OrichError::runtime("list index out of bounds"))
            }
            Value::String(value) => {
                let index = match index {
                    Value::Int(value) => value,
                    other => {
                        return Err(OrichError::runtime(format!(
                            "string index must be int, got {}",
                            other.type_name()
                        )))
                    }
                };
                value
                    .chars()
                    .nth(index as usize)
                    .map(|ch| Value::String(ch.to_string()))
                    .ok_or_else(|| OrichError::runtime("string index out of bounds"))
            }
            Value::Map(values) => {
                let key = index.to_string();
                values
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| OrichError::runtime(format!("key '{key}' not found")))
            }
            other => Err(OrichError::runtime(format!(
                "cannot index {}",
                other.type_name()
            ))),
        }
    }

    fn interpolate(&mut self, raw: &str) -> OrichResult<String> {
        let mut output = String::new();
        let chars: Vec<char> = raw.chars().collect();
        let mut index = 0;
        while index < chars.len() {
            if chars[index] == '{' {
                if chars.get(index + 1) == Some(&'{') {
                    output.push('{');
                    index += 2;
                    continue;
                }
                let start = index + 1;
                let mut end = start;
                while end < chars.len() && chars[end] != '}' {
                    end += 1;
                }
                if end == chars.len() {
                    return Err(OrichError::runtime("unterminated interpolation"));
                }
                let expr_text: String = chars[start..end].iter().collect();
                let tokens = Lexer::new(&expr_text).tokenize()?;
                let expr = Parser::new(tokens).parse_expression_only()?;
                let value = self.eval(&expr)?;
                output.push_str(&value.to_string());
                index = end + 1;
            } else if chars[index] == '}' && chars.get(index + 1) == Some(&'}') {
                output.push('}');
                index += 2;
            } else {
                output.push(chars[index]);
                index += 1;
            }
        }
        Ok(output)
    }

    fn define(&mut self, name: &str, value: Value, mutable: bool) -> OrichResult<()> {
        let scope = self.scopes.last_mut().expect("runtime always has a scope");
        if scope.contains_key(name) {
            return Err(OrichError::runtime(format!(
                "'{name}' is already defined in this scope"
            )));
        }
        scope.insert(name.to_string(), Binding { value, mutable });
        Ok(())
    }

    fn assign(&mut self, name: &str, value: Value) -> OrichResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                if !binding.mutable {
                    return Err(OrichError::runtime(format!(
                        "cannot assign to const '{name}'"
                    )));
                }
                binding.value = value;
                return Ok(());
            }
        }
        Err(OrichError::runtime(format!("undefined variable '{name}'")))
    }

    fn get(&self, name: &str) -> Option<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|binding| binding.value.clone()))
    }
}

fn to_number(value: &Value) -> OrichResult<f64> {
    match value {
        Value::Int(value) => Ok(*value as f64),
        Value::Float(value) => Ok(*value),
        other => Err(OrichError::runtime(format!(
            "expected number, got {}",
            other.type_name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_source;

    #[test]
    fn emits_interpolated_input() {
        let mut input = BTreeMap::new();
        input.insert("name".to_string(), Value::String("Ana".to_string()));
        let output = run_source("let name = input.name\nemit \"Hello, {name}\"", input).unwrap();
        assert_eq!(output, "Hello, Ana");
    }
}
