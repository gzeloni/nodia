// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Core runtime construction and top-level statement execution.

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

impl Runtime {
    /// Creates a runtime with default options and no base directory.
    pub fn new(input: BTreeMap<String, Value>) -> Self {
        Self::with_options(input, None, RuntimeOptions::default())
    }

    /// Creates a runtime with a fixed base directory for module resolution.
    pub fn with_base_dir(input: BTreeMap<String, Value>, base_dir: Option<PathBuf>) -> Self {
        Self::with_options(input, base_dir, RuntimeOptions::default())
    }

    /// Creates a runtime with explicit execution options.
    pub fn with_options(
        input: BTreeMap<String, Value>,
        base_dir: Option<PathBuf>,
        options: RuntimeOptions,
    ) -> Self {
        Self::with_context(
            input,
            base_dir,
            Rc::new(RefCell::new(HashMap::new())),
            None,
            Rc::new(RefCell::new(IoRegistry::new())),
            options,
        )
    }

    pub(super) fn with_context(
        input: BTreeMap<String, Value>,
        base_dir: Option<PathBuf>,
        modules: ModuleCache,
        current_module: Option<ModuleRef>,
        io: IoState,
        options: RuntimeOptions,
    ) -> Self {
        let prng_state = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let mut root = HashMap::new();
        root.insert(
            "input".to_string(),
            binding_ref(Value::Map(input.clone()), false),
        );
        root.insert(
            "regex".to_string(),
            binding_ref(Self::regex_surface_value(), false),
        );
        Self {
            scopes: vec![root],
            output: String::new(),
            input,
            base_dir,
            modules,
            current_module,
            io,
            options,
            prng_state,
        }
    }

    fn regex_surface_value() -> Value {
        let mut exports = BTreeMap::new();
        for (field, target, arities) in stdlib::regex_surface_items() {
            let value = match arities {
                Some(_) => Value::BuiltinFunction((*target).to_string()),
                None => match *target {
                    "regex.any" => Value::String("any".to_string()),
                    "regex.full" => Value::String("full".to_string()),
                    "regex.first" => Value::String("first".to_string()),
                    "regex.all" => Value::String("all".to_string()),
                    other => panic!("unsupported regex surface binding '{other}'"),
                },
            };
            exports.insert((*field).to_string(), value);
        }
        Value::Map(exports)
    }

    /// Executes a parsed program and returns emitted output without the trailing newline.
    pub fn run(&mut self, program: &Program) -> NodiaResult<String> {
        for statement in &program.statements {
            let flow = match self.execute(statement) {
                Ok(flow) => flow,
                Err(err) => {
                    self.flush_output_channel()?;
                    self.io.borrow_mut().flush_all()?;
                    return Err(err.with_output(self.output.trim_end_matches('\n').to_string()));
                }
            };
            match flow {
                Flow::None => self.publish_statement(statement)?,
                Flow::Return(_) => return Err(NodiaError::runtime("return outside function")),
                Flow::Break => return Err(NodiaError::runtime("break outside loop")),
                Flow::Continue => return Err(NodiaError::runtime("continue outside loop")),
            }
        }
        self.flush_output_channel()?;
        self.io.borrow_mut().flush_all()?;
        Ok(self.output.trim_end_matches('\n').to_string())
    }

    pub(super) fn write_output_channel(&mut self, text: &str) -> NodiaResult<()> {
        self.output.push_str(text);
        if self.options.mirror_output {
            stdio::stdout()
                .write_all(text.as_bytes())
                .map_err(|err| NodiaError::io(format!("cannot write stdout: {err}")))?;
            self.flush_output_channel()?;
        }
        Ok(())
    }

    pub(super) fn flush_output_channel(&mut self) -> NodiaResult<()> {
        if self.options.mirror_output {
            stdio::stdout()
                .flush()
                .map_err(|err| NodiaError::io(format!("cannot flush stdout: {err}")))?;
        }
        Ok(())
    }

    pub(super) fn execute_block(&mut self, statements: &[Stmt]) -> NodiaResult<Flow> {
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

    pub(super) fn execute(&mut self, statement: &Stmt) -> NodiaResult<Flow> {
        match statement {
            Stmt::Comment(_) => Ok(Flow::None),
            Stmt::Use {
                target,
                alias,
                pick,
                hide,
            } => {
                self.execute_use(target, alias.as_deref(), pick, hide)?;
                Ok(Flow::None)
            }
            Stmt::Bind {
                name,
                value,
                mutable,
            } => {
                let value = self.eval(value)?;
                self.define(name, value, *mutable)?;
                Ok(Flow::None)
            }
            Stmt::Assign { target, value } => {
                let value = self.eval(value)?;
                self.assign_target(target, value)?;
                Ok(Flow::None)
            }
            Stmt::Func { name, params, body } => {
                let func = self.function_value(name, params, body);
                self.define(name, func, false)?;
                Ok(Flow::None)
            }
            Stmt::Return(value) => Ok(Flow::Return(match value {
                Some(expr) => self.eval(expr)?,
                None => Value::Null,
            })),
            Stmt::Throw(expr) => {
                let value = self.eval(expr)?;
                Err(self.thrown_error(value))
            }
            Stmt::Emit(expr) => {
                let value = self.eval(expr)?;
                self.write_output_channel(&value.to_string())?;
                self.write_output_channel("\n")?;
                Ok(Flow::None)
            }
            Stmt::Try {
                try_branch,
                catch_name,
                catch_branch,
            } => match self.execute_block(try_branch) {
                Ok(flow) => Ok(flow),
                Err(err) if err.exit_status.is_some() => Err(err),
                Err(err) => {
                    let caught = self.caught_error_value(err);
                    self.scopes.push(HashMap::new());
                    let result = (|| {
                        self.define(catch_name, caught, false)?;
                        self.execute_block(catch_branch)
                    })();
                    self.scopes.pop();
                    result
                }
            },
            Stmt::Match {
                value,
                arms,
                default,
            } => {
                let value = self.eval(value)?;
                self.execute_match(&value, arms, default.as_deref())
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
                binding,
                iterable,
                body,
            } => {
                let iterable = self.eval(iterable)?;
                if let Value::Lazy(lazy) = iterable {
                    return self.execute_lazy_for(binding, &lazy, body);
                }
                let pairs = self.iterable_values(binding, iterable)?;
                for values in pairs {
                    self.scopes.push(HashMap::new());
                    for (name, value) in values {
                        self.define(&name, value, true)?;
                    }
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
                        return Err(NodiaError::runtime("while loop exceeded 100000 iterations"));
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
            Stmt::Namespace { name, body } => {
                let ns = self.execute_namespace(name, body)?;
                self.define(name, Value::Map(ns), false)?;
                Ok(Flow::None)
            }
            Stmt::Struct { name, fields } => {
                let ctor = self.struct_constructor(name, fields);
                self.define(name, ctor, false)?;
                Ok(Flow::None)
            }
            Stmt::Enum { name, variants } => {
                let ns = self.enum_namespace(name, variants);
                self.define(name, Value::Map(ns), false)?;
                Ok(Flow::None)
            }
            Stmt::TypeAlias { name: _, target: _ } => Ok(Flow::None),
        }
    }

    fn execute_namespace(
        &mut self,
        _name: &str,
        body: &[Stmt],
    ) -> NodiaResult<BTreeMap<String, Value>> {
        self.scopes.push(HashMap::new());
        for statement in body {
            match self.execute(statement)? {
                Flow::None => {}
                _flow => {
                    self.scopes.pop();
                    return Err(NodiaError::runtime(
                        "unexpected control flow inside namespace".to_string(),
                    ));
                }
            }
        }
        let scope = self.scopes.pop().expect("namespace scope");
        let mut exports = BTreeMap::new();
        for (name, binding) in scope {
            exports.insert(name, binding.borrow().value.clone());
        }
        Ok(exports)
    }

    fn struct_constructor(&mut self, _name: &str, fields: &[StructField]) -> Value {
        let mut ns = BTreeMap::new();
        for field in fields {
            let default = match &field.default {
                Some(expr) => self.eval(expr).unwrap_or(Value::Null),
                None => Value::Null,
            };
            ns.insert(field.name.clone(), default);
        }
        Value::Map(ns)
    }

    fn enum_namespace(&self, _name: &str, variants: &[String]) -> BTreeMap<String, Value> {
        let mut exports = BTreeMap::new();
        for variant in variants {
            let mut map = BTreeMap::new();
            map.insert("kind".to_string(), Value::String(variant.clone()));
            exports.insert(variant.clone(), Value::Map(map));
        }
        exports
    }

    fn thrown_error(&self, value: Value) -> NodiaError {
        match value {
            Value::Map(fields) => RecoverableErrorValue::from_map(&fields)
                .map(|error| error.to_error())
                .unwrap_or_else(|| NodiaError::runtime(Value::Map(fields).to_string())),
            other => NodiaError::runtime(other.to_string()),
        }
    }

    fn caught_error_value(&self, error: NodiaError) -> Value {
        Value::Map(RecoverableErrorValue::from_error(error).to_map())
    }

    fn execute_match(
        &mut self,
        value: &Value,
        arms: &[MatchArm],
        default: Option<&[Stmt]>,
    ) -> NodiaResult<Flow> {
        for arm in arms {
            let mut bindings = Vec::new();
            if self.match_pattern(&arm.pattern, value, &mut bindings) {
                return self.execute_match_arm(bindings, &arm.body);
            }
        }
        match default {
            Some(body) => self.execute_block(body),
            None => Ok(Flow::None),
        }
    }

    fn execute_match_arm(
        &mut self,
        bindings: Vec<(String, Value)>,
        body: &[Stmt],
    ) -> NodiaResult<Flow> {
        self.scopes.push(HashMap::new());
        let result = (|| {
            for (name, value) in bindings {
                self.define(&name, value, false)?;
            }
            self.execute_block(body)
        })();
        self.scopes.pop();
        result
    }

    fn match_pattern(
        &self,
        pattern: &MatchPattern,
        value: &Value,
        bindings: &mut Vec<(String, Value)>,
    ) -> bool {
        let start = bindings.len();
        let matched = match pattern {
            MatchPattern::Wildcard => true,
            MatchPattern::Capture(name) => {
                bindings.push((name.clone(), value.clone()));
                true
            }
            MatchPattern::Literal(expected) => value == expected,
            MatchPattern::List(items) => match value {
                Value::List(values) if values.len() == items.len() => items
                    .iter()
                    .zip(values)
                    .all(|(pattern, value)| self.match_pattern(pattern, value, bindings)),
                _ => false,
            },
            MatchPattern::Map(entries) => match value {
                Value::Map(values) => entries.iter().all(|(key, pattern)| {
                    values
                        .get(key)
                        .is_some_and(|value| self.match_pattern(pattern, value, bindings))
                }),
                _ => false,
            },
        };
        if !matched {
            bindings.truncate(start);
        }
        matched
    }
}
