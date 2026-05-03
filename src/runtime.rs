use crate::ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::error::{OrichError, OrichResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::stdlib;
use crate::value::{Function, Module, ModuleRef, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

type ModuleCache = Rc<RefCell<HashMap<PathBuf, ModuleRef>>>;

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
    input: BTreeMap<String, Value>,
    base_dir: Option<PathBuf>,
    modules: ModuleCache,
    current_module: Option<ModuleRef>,
}

impl Runtime {
    pub fn new(input: BTreeMap<String, Value>) -> Self {
        Self::with_base_dir(input, None)
    }

    pub fn with_base_dir(input: BTreeMap<String, Value>, base_dir: Option<PathBuf>) -> Self {
        Self::with_context(input, base_dir, Rc::new(RefCell::new(HashMap::new())), None)
    }

    fn with_context(
        input: BTreeMap<String, Value>,
        base_dir: Option<PathBuf>,
        modules: ModuleCache,
        current_module: Option<ModuleRef>,
    ) -> Self {
        let mut root = HashMap::new();
        root.insert(
            "input".to_string(),
            Binding {
                value: Value::Map(input.clone()),
                mutable: false,
            },
        );
        Self {
            scopes: vec![root],
            output: String::new(),
            input,
            base_dir,
            modules,
            current_module,
        }
    }

    pub fn run(&mut self, program: &Program) -> OrichResult<String> {
        for statement in &program.statements {
            match self.execute(statement)? {
                Flow::None => self.publish_statement(statement)?,
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
            Stmt::Comment(_) => Ok(Flow::None),
            Stmt::Import {
                path,
                alias,
                show,
                hide,
            } => {
                self.execute_import(path, alias.as_deref(), show, hide)?;
                Ok(Flow::None)
            }
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
                        captures: BTreeMap::new(),
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

    fn execute_import(
        &mut self,
        path: &str,
        alias: Option<&str>,
        show: &[String],
        hide: &[String],
    ) -> OrichResult<()> {
        let resolved = self.resolve_import(path)?;
        let module = self.load_module(&resolved)?;
        let names = self.selected_import_names(&module, show, hide)?;

        if let Some(alias) = alias {
            let mut namespace = BTreeMap::new();
            for name in names {
                namespace.insert(name.clone(), Value::ImportBinding(module.clone(), name));
            }
            self.define(alias, Value::Map(namespace), false)
        } else {
            for name in names {
                self.define(
                    &name,
                    Value::ImportBinding(module.clone(), name.clone()),
                    false,
                )?;
            }
            Ok(())
        }
    }

    fn load_module(&mut self, resolved: &Path) -> OrichResult<ModuleRef> {
        if let Some(module) = self.modules.borrow().get(resolved).cloned() {
            return Ok(module);
        }

        let source = fs::read_to_string(resolved).map_err(|err| {
            OrichError::io(format!(
                "cannot read import '{}': {err}",
                resolved.display()
            ))
        })?;
        let tokens = Lexer::new(&source)
            .tokenize()
            .map_err(|err| err.with_file(resolved.display().to_string()))?;
        let program = Parser::new(tokens)
            .parse_program()
            .map_err(|err| err.with_file(resolved.display().to_string()))?;
        let declared = declared_bindings(&program);
        let module = Rc::new(RefCell::new(Module {
            path: resolved.to_path_buf(),
            declared: declared.keys().cloned().collect(),
            exports: BTreeMap::new(),
            mutability: declared,
            loaded: false,
        }));
        self.modules
            .borrow_mut()
            .insert(resolved.to_path_buf(), module.clone());

        let mut runtime = Runtime::with_context(
            self.input.clone(),
            resolved.parent().map(Path::to_path_buf),
            self.modules.clone(),
            Some(module.clone()),
        );
        runtime
            .run(&program)
            .map_err(|err| err.with_file(resolved.display().to_string()))?;
        let exports = runtime.export_declared_bindings();
        let mut module_mut = module.borrow_mut();
        module_mut.exports = exports;
        module_mut.loaded = true;
        drop(module_mut);
        Ok(module)
    }

    fn selected_import_names(
        &self,
        module: &ModuleRef,
        show: &[String],
        hide: &[String],
    ) -> OrichResult<Vec<String>> {
        let module = module.borrow();
        let all = if module.declared.is_empty() {
            module.exports.keys().cloned().collect::<Vec<_>>()
        } else {
            module.declared.clone()
        };

        let mut names = if show.is_empty() {
            all.clone()
        } else {
            for name in show {
                if !all.contains(name) {
                    return Err(OrichError::runtime(format!(
                        "import '{}' does not export '{name}'",
                        module.path.display()
                    )));
                }
            }
            show.to_vec()
        };

        names.retain(|name| !hide.contains(name));
        Ok(names)
    }

    fn resolve_import(&self, path: &str) -> OrichResult<PathBuf> {
        let raw = Path::new(path);
        let joined = if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            self.base_dir
                .as_deref()
                .unwrap_or_else(|| Path::new("."))
                .join(raw)
        };

        let candidates = if joined.extension().is_some() {
            vec![joined]
        } else {
            vec![
                joined.with_extension("och"),
                joined.join("index.och"),
                joined,
            ]
        };

        for candidate in candidates {
            if candidate.exists() {
                return candidate.canonicalize().map_err(|err| {
                    OrichError::io(format!(
                        "cannot resolve import '{}': {err}",
                        candidate.display()
                    ))
                });
            }
        }

        Err(OrichError::io(format!("cannot resolve import '{path}'")))
    }

    fn publish_statement(&mut self, statement: &Stmt) -> OrichResult<()> {
        let Some(name) = statement_export_name(statement) else {
            return Ok(());
        };
        let Some(module) = &self.current_module else {
            return Ok(());
        };
        if !module
            .borrow()
            .declared
            .iter()
            .any(|declared| declared == name)
        {
            return Ok(());
        }
        let Some(value) = self.root_get(name) else {
            return Ok(());
        };
        let value = self.prepare_export(value);
        module.borrow_mut().exports.insert(name.to_string(), value);
        Ok(())
    }

    fn export_declared_bindings(&self) -> BTreeMap<String, Value> {
        let Some(module) = &self.current_module else {
            return BTreeMap::new();
        };
        let declared = module.borrow().declared.clone();
        let mut exports = BTreeMap::new();
        for name in declared {
            if let Some(value) = self.root_get(&name) {
                exports.insert(name, self.prepare_export(value));
            }
        }
        exports
    }

    fn prepare_export(&self, value: Value) -> Value {
        match value {
            Value::Function(mut function) => {
                function.captures = self.capture_bindings();
                Value::Function(function)
            }
            other => other,
        }
    }

    fn capture_bindings(&self) -> BTreeMap<String, Value> {
        self.scopes
            .first()
            .into_iter()
            .flat_map(|scope| scope.iter())
            .filter(|(name, _)| name.as_str() != "input")
            .map(|(name, binding)| (name.clone(), binding.value.clone()))
            .collect()
    }

    fn eval(&mut self, expr: &Expr) -> OrichResult<Value> {
        match expr {
            Expr::Literal(Value::String(value)) => Ok(Value::String(self.interpolate(value)?)),
            Expr::Literal(value) => self.resolve_value(value.clone()),
            Expr::Identifier(name) => {
                let value = self
                    .get(name)
                    .ok_or_else(|| OrichError::runtime(format!("undefined variable '{name}'")))?;
                self.resolve_value(value)
            }
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
                    Value::Map(map) => {
                        let value = map.get(field).cloned().ok_or_else(|| {
                            OrichError::runtime(format!("field '{field}' not found"))
                        })?;
                        self.resolve_value(value)
                    }
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

    fn resolve_value(&self, value: Value) -> OrichResult<Value> {
        match value {
            Value::ImportBinding(module, name) => {
                module.borrow().exports.get(&name).cloned().ok_or_else(|| {
                    OrichError::runtime(format!("imported binding '{name}' is not initialized yet"))
                })
            }
            other => Ok(other),
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
                let has_captures = !function.captures.is_empty();
                if has_captures {
                    self.scopes.push(binding_scope(&function.captures, false));
                }
                self.scopes.push(HashMap::new());
                for (name, value) in function.params.iter().zip(arg_values) {
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
                let value = values
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| OrichError::runtime(format!("key '{key}' not found")))?;
                self.resolve_value(value)
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
                if let Value::ImportBinding(module, export_name) = binding.value.clone() {
                    return assign_import_binding(module, &export_name, value);
                }
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

    fn root_get(&self, name: &str) -> Option<Value> {
        self.scopes
            .first()
            .and_then(|scope| scope.get(name).map(|binding| binding.value.clone()))
    }
}

fn declared_bindings(program: &Program) -> BTreeMap<String, bool> {
    program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Stmt::Let { name, mutable, .. } => Some((name.clone(), *mutable)),
            Stmt::Fn { name, .. } => Some((name.clone(), false)),
            _ => None,
        })
        .collect()
}

fn statement_export_name(statement: &Stmt) -> Option<&str> {
    match statement {
        Stmt::Let { name, .. } | Stmt::Fn { name, .. } | Stmt::Assign { name, .. } => Some(name),
        _ => None,
    }
}

fn assign_import_binding(module: ModuleRef, name: &str, value: Value) -> OrichResult<()> {
    let mut module = module.borrow_mut();
    let mutable = module.mutability.get(name).copied().unwrap_or(false);
    if !mutable {
        return Err(OrichError::runtime(format!(
            "cannot assign to const '{name}'"
        )));
    }
    if !module.exports.contains_key(name) {
        return Err(OrichError::runtime(format!(
            "imported binding '{name}' is not initialized yet"
        )));
    }
    module.exports.insert(name.to_string(), value);
    Ok(())
}

fn binding_scope(values: &BTreeMap<String, Value>, mutable: bool) -> HashMap<String, Binding> {
    values
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                Binding {
                    value: value.clone(),
                    mutable,
                },
            )
        })
        .collect()
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
    use std::fs;

    #[test]
    fn emits_interpolated_input() {
        let mut input = BTreeMap::new();
        input.insert("name".to_string(), Value::String("Ana".to_string()));
        let output = run_source("let name = input.name\nemit \"Hello, {name}\"", input).unwrap();
        assert_eq!(output, "Hello, Ana");
    }

    #[test]
    fn imported_functions_keep_module_bindings() {
        let dir = std::env::temp_dir().join(format!("orich-import-capture-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.och");
        let main = dir.join("main.och");
        fs::write(
            &lib,
            "const prefix = \"Hi\"\nfn greet(name) {\n  return \"{prefix}, {name}\"\n}\n",
        )
        .unwrap();
        fs::write(&main, "import './lib' as lib\nemit lib.greet(\"Ana\")\n").unwrap();

        let output = crate::run_file(&main, BTreeMap::new()).unwrap();
        assert_eq!(output, "Hi, Ana");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn direct_import_of_let_can_be_assigned() {
        let dir = std::env::temp_dir().join(format!("orich-import-let-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bar = dir.join("bar.och");
        let main = dir.join("main.och");
        fs::write(&bar, "let n = 0\n").unwrap();
        fs::write(
            &main,
            "import './bar' show n\nwhile n < 3 {\n  emit n\n  n = n + 1\n}\n",
        )
        .unwrap();

        let output = crate::run_file(&main, BTreeMap::new()).unwrap();
        assert_eq!(output, "0\n1\n2");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn direct_import_of_const_cannot_be_assigned() {
        let dir = std::env::temp_dir().join(format!("orich-import-const-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bar = dir.join("bar.och");
        let main = dir.join("main.och");
        fs::write(&bar, "const n = 0\n").unwrap();
        fs::write(&main, "import './bar' show n\nn = n + 1\n").unwrap();

        let err = crate::run_file(&main, BTreeMap::new()).unwrap_err();
        assert!(err.to_string().contains("cannot assign to const 'n'"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn circular_imports_are_cached_and_resolved_lazily() {
        let dir =
            std::env::temp_dir().join(format!("orich-circular-import-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.och");
        let b = dir.join("b.och");
        let main = dir.join("main.och");
        fs::write(
            &a,
            "import './b' as b\nconst name = \"A\"\nfn pair() {\n  return \"{name}/{b.name}\"\n}\n",
        )
        .unwrap();
        fs::write(
            &b,
            "import './a' as a\nconst name = \"B\"\nfn pair() {\n  return \"{name}/{a.name}\"\n}\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import './a' as a\nimport './b' as b\nemit a.pair()\nemit b.pair()\n",
        )
        .unwrap();

        let output = crate::run_file(&main, BTreeMap::new()).unwrap();
        assert_eq!(output, "A/B\nB/A");
        let _ = fs::remove_dir_all(dir);
    }
}
