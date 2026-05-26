use crate::ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::error::{DobraError, DobraResult};
use crate::io::{self as fsio, IoRegistry};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::regex;
use crate::stdlib;
use crate::value::{Function, Module, ModuleRef, StreamId, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self as stdio, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

type ModuleCache = Rc<RefCell<HashMap<PathBuf, ModuleRef>>>;
type IoState = Rc<RefCell<IoRegistry>>;

#[derive(Debug, Clone, Default)]
pub struct RuntimeOptions {
    pub allow_write: bool,
}

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
    io: IoState,
    options: RuntimeOptions,
}

impl Runtime {
    pub fn new(input: BTreeMap<String, Value>) -> Self {
        Self::with_options(input, None, RuntimeOptions::default())
    }

    pub fn with_base_dir(input: BTreeMap<String, Value>, base_dir: Option<PathBuf>) -> Self {
        Self::with_options(input, base_dir, RuntimeOptions::default())
    }

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

    fn with_context(
        input: BTreeMap<String, Value>,
        base_dir: Option<PathBuf>,
        modules: ModuleCache,
        current_module: Option<ModuleRef>,
        io: IoState,
        options: RuntimeOptions,
    ) -> Self {
        let mut root = HashMap::new();
        root.insert(
            "input".to_string(),
            Binding {
                value: Value::Map(input.clone()),
                mutable: false,
            },
        );
        root.insert(
            "stdin".to_string(),
            Binding {
                value: Value::Stream(StreamId::Stdin),
                mutable: false,
            },
        );
        root.insert(
            "stdout".to_string(),
            Binding {
                value: Value::Stream(StreamId::Stdout),
                mutable: false,
            },
        );
        root.insert(
            "stderr".to_string(),
            Binding {
                value: Value::Stream(StreamId::Stderr),
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
            io,
            options,
        }
    }

    pub fn run(&mut self, program: &Program) -> DobraResult<String> {
        for statement in &program.statements {
            match self.execute(statement)? {
                Flow::None => self.publish_statement(statement)?,
                Flow::Return(_) => return Err(DobraError::runtime("return outside function")),
                Flow::Break => return Err(DobraError::runtime("break outside loop")),
                Flow::Continue => return Err(DobraError::runtime("continue outside loop")),
            }
        }
        self.io.borrow_mut().flush_all()?;
        Ok(self.output.trim_end_matches('\n').to_string())
    }

    fn execute_block(&mut self, statements: &[Stmt]) -> DobraResult<Flow> {
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

    fn execute(&mut self, statement: &Stmt) -> DobraResult<Flow> {
        match statement {
            Stmt::Comment(_) => Ok(Flow::None),
            Stmt::Use {
                path,
                alias,
                pick,
                hide,
            } => {
                self.execute_use(path, alias.as_deref(), pick, hide)?;
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
            Stmt::Assign { name, value } => {
                let value = self.eval(value)?;
                self.assign(name, value)?;
                Ok(Flow::None)
            }
            Stmt::Func { name, params, body } => {
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
                        return Err(DobraError::runtime(format!(
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
                        return Err(DobraError::runtime("while loop exceeded 100000 iterations"));
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

    fn execute_use(
        &mut self,
        path: &str,
        alias: Option<&str>,
        pick: &[String],
        hide: &[String],
    ) -> DobraResult<()> {
        let resolved = self.resolve_use(path)?;
        let module = self.load_module(&resolved)?;
        let names = self.selected_use_names(&module, pick, hide)?;

        if let Some(alias) = alias {
            let mut namespace = BTreeMap::new();
            for name in names {
                namespace.insert(name.clone(), Value::UseBinding(module.clone(), name));
            }
            self.define(alias, Value::Map(namespace), false)
        } else {
            for name in names {
                self.define(
                    &name,
                    Value::UseBinding(module.clone(), name.clone()),
                    false,
                )?;
            }
            Ok(())
        }
    }

    fn load_module(&mut self, resolved: &Path) -> DobraResult<ModuleRef> {
        if let Some(module) = self.modules.borrow().get(resolved).cloned() {
            return Ok(module);
        }

        let source = fs::read_to_string(resolved).map_err(|err| {
            DobraError::io(format!("cannot read use '{}': {err}", resolved.display()))
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
            self.io.clone(),
            self.options.clone(),
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

    fn selected_use_names(
        &self,
        module: &ModuleRef,
        pick: &[String],
        hide: &[String],
    ) -> DobraResult<Vec<String>> {
        let module = module.borrow();
        let all = if module.declared.is_empty() {
            module.exports.keys().cloned().collect::<Vec<_>>()
        } else {
            module.declared.clone()
        };

        let mut names = if pick.is_empty() {
            all.clone()
        } else {
            for name in pick {
                if !all.contains(name) {
                    return Err(DobraError::runtime(format!(
                        "use '{}' does not expose '{name}'",
                        module.path.display()
                    )));
                }
            }
            pick.to_vec()
        };

        names.retain(|name| !hide.contains(name));
        Ok(names)
    }

    fn resolve_use(&self, path: &str) -> DobraResult<PathBuf> {
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
                joined.with_extension("nod"),
                joined.join("index.nod"),
                joined,
            ]
        };

        for candidate in candidates {
            if candidate.exists() {
                return candidate.canonicalize().map_err(|err| {
                    DobraError::io(format!(
                        "cannot resolve use '{}': {err}",
                        candidate.display()
                    ))
                });
            }
        }

        Err(DobraError::io(format!("cannot resolve use '{path}'")))
    }

    fn publish_statement(&mut self, statement: &Stmt) -> DobraResult<()> {
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

    fn eval(&mut self, expr: &Expr) -> DobraResult<Value> {
        match expr {
            Expr::Literal(Value::String(value)) => Ok(Value::String(self.interpolate(value)?)),
            Expr::Literal(value) => self.resolve_value(value.clone()),
            Expr::Regex(pattern) => regex::compile(pattern).map(Value::Regex),
            Expr::Identifier(name) => {
                let value = self
                    .get(name)
                    .ok_or_else(|| DobraError::runtime(format!("undefined variable '{name}'")))?;
                self.resolve_value(value)
            }
            Expr::Unary { op, expr } => {
                let value = self.eval(expr)?;
                match op {
                    UnaryOp::Negate => match value {
                        Value::Int(value) => Ok(Value::Int(-value)),
                        Value::Float(value) => Ok(Value::Float(-value)),
                        other => Err(DobraError::runtime(format!(
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
                            DobraError::runtime(format!("field '{field}' not found"))
                        })?;
                        self.resolve_value(value)
                    }
                    other => Err(DobraError::runtime(format!(
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
                .collect::<DobraResult<Vec<_>>>()
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

    fn resolve_value(&self, value: Value) -> DobraResult<Value> {
        match value {
            Value::UseBinding(module, name) => {
                module.borrow().exports.get(&name).cloned().ok_or_else(|| {
                    DobraError::runtime(format!("used binding '{name}' is not initialized yet"))
                })
            }
            other => Ok(other),
        }
    }

    fn eval_binary(&mut self, left: &Expr, op: BinaryOp, right: &Expr) -> DobraResult<Value> {
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

    fn call(&mut self, callee: &Expr, args: &[Expr]) -> DobraResult<Value> {
        let arg_values = args
            .iter()
            .map(|arg| self.eval(arg))
            .collect::<DobraResult<Vec<_>>>()?;

        if let Expr::Identifier(name) = callee {
            if let Some(result) = self.call_io_builtin(name, &arg_values)? {
                return Ok(result);
            }
            if let Some(result) = stdlib::call(name, arg_values.clone())? {
                return Ok(result);
            }
        }

        let callee = self.eval(callee)?;
        match callee {
            Value::Function(function) => {
                if function.params.len() != arg_values.len() {
                    return Err(DobraError::runtime(format!(
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
                    Flow::Break => Err(DobraError::runtime("break inside function without loop")),
                    Flow::Continue => {
                        Err(DobraError::runtime("continue inside function without loop"))
                    }
                }
            }
            other => Err(DobraError::runtime(format!(
                "{} is not callable",
                other.type_name()
            ))),
        }
    }

    fn call_io_builtin(&mut self, name: &str, args: &[Value]) -> DobraResult<Option<Value>> {
        let result = match name {
            "open" => {
                self.expect_arity(args, 2, "open")?;
                let path = self.expect_string(&args[0], "open", "first")?;
                let mode = self.expect_string(&args[1], "open", "second")?;
                Value::Stream(
                    self.io
                        .borrow_mut()
                        .open(&path, &mode, self.options.allow_write)?,
                )
            }
            "close" => {
                self.expect_arity(args, 1, "close")?;
                self.close_stream(self.expect_stream(&args[0], "close", "first")?)?;
                Value::Null
            }
            "flush" => {
                self.expect_arity(args, 1, "flush")?;
                self.flush_stream(self.expect_stream(&args[0], "flush", "first")?)?;
                Value::Null
            }
            "eof" => {
                self.expect_arity(args, 1, "eof")?;
                Value::Bool(self.eof_stream(self.expect_stream(&args[0], "eof", "first")?)?)
            }
            "read" => self.read_builtin(args)?,
            "readln" => {
                self.expect_arity(args, 1, "readln")?;
                match self.read_line_stream(self.expect_stream(&args[0], "readln", "first")?)? {
                    Some(line) => Value::String(line),
                    None => Value::Null,
                }
            }
            "write" => self.write_builtin(args, false)?,
            "writeln" => self.write_builtin(args, true)?,
            "append" => {
                self.expect_arity(args, 2, "append")?;
                let path = self.expect_string(&args[0], "append", "first")?;
                fsio::append_path(&path, &args[1].to_string(), self.options.allow_write)?;
                Value::Null
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }

    fn read_builtin(&mut self, args: &[Value]) -> DobraResult<Value> {
        if args.len() == 1 {
            return match &args[0] {
                Value::String(path) => fsio::read_path(path).map(Value::String),
                Value::Stream(stream) => self.read_stream(*stream).map(Value::String),
                other => Err(DobraError::runtime(format!(
                    "read() expects path or stream, got {}",
                    other.type_name()
                ))),
            };
        }
        if args.len() == 2 {
            let stream = self.expect_stream(&args[0], "read", "first")?;
            let size = self.expect_non_negative_size(&args[1], "read", "second")?;
            return self.read_chunk_stream(stream, size).map(Value::String);
        }
        Err(DobraError::runtime(format!(
            "read() expects 1 or 2 argument(s), got {}",
            args.len()
        )))
    }

    fn write_builtin(&mut self, args: &[Value], line: bool) -> DobraResult<Value> {
        self.expect_arity(args, 2, if line { "writeln" } else { "write" })?;
        let mut text = args[1].to_string();
        if line {
            text.push('\n');
        }
        match &args[0] {
            Value::String(path) if !line => {
                fsio::write_path(path, &text, self.options.allow_write)?;
            }
            Value::String(_) => {
                return Err(DobraError::runtime(
                    "writeln() expects stream as first argument",
                ));
            }
            Value::Stream(stream) => self.write_stream(*stream, &text)?,
            other => {
                return Err(DobraError::runtime(format!(
                    "{}() expects path or stream, got {}",
                    if line { "writeln" } else { "write" },
                    other.type_name()
                )));
            }
        }
        Ok(Value::Null)
    }

    fn read_stream(&mut self, stream: StreamId) -> DobraResult<String> {
        match stream {
            StreamId::Stdin => {
                let mut input = String::new();
                stdio::stdin()
                    .lock()
                    .read_to_string(&mut input)
                    .map_err(|err| DobraError::io(format!("cannot read stdin: {err}")))?;
                Ok(input)
            }
            StreamId::Stdout => Err(DobraError::runtime("cannot read from stdout")),
            StreamId::Stderr => Err(DobraError::runtime("cannot read from stderr")),
            StreamId::File(_) => self.io.borrow_mut().read_all(stream),
        }
    }

    fn read_chunk_stream(&mut self, stream: StreamId, size: usize) -> DobraResult<String> {
        match stream {
            StreamId::Stdin => {
                let mut buffer = vec![0; size];
                let read = stdio::stdin()
                    .lock()
                    .read(&mut buffer)
                    .map_err(|err| DobraError::io(format!("cannot read stdin: {err}")))?;
                buffer.truncate(read);
                Ok(String::from_utf8_lossy(&buffer).to_string())
            }
            StreamId::Stdout => Err(DobraError::runtime("cannot read from stdout")),
            StreamId::Stderr => Err(DobraError::runtime("cannot read from stderr")),
            StreamId::File(_) => self.io.borrow_mut().read_chunk(stream, size),
        }
    }

    fn read_line_stream(&mut self, stream: StreamId) -> DobraResult<Option<String>> {
        match stream {
            StreamId::Stdin => {
                let mut line = String::new();
                let read = stdio::stdin()
                    .lock()
                    .read_line(&mut line)
                    .map_err(|err| DobraError::io(format!("cannot read stdin: {err}")))?;
                if read == 0 {
                    return Ok(None);
                }
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                Ok(Some(line))
            }
            StreamId::Stdout => Err(DobraError::runtime("cannot read from stdout")),
            StreamId::Stderr => Err(DobraError::runtime("cannot read from stderr")),
            StreamId::File(_) => self.io.borrow_mut().read_line(stream),
        }
    }

    fn write_stream(&mut self, stream: StreamId, text: &str) -> DobraResult<()> {
        match stream {
            StreamId::Stdin => Err(DobraError::runtime("cannot write to stdin")),
            StreamId::Stdout => {
                self.output.push_str(text);
                Ok(())
            }
            StreamId::Stderr => stdio::stderr()
                .write_all(text.as_bytes())
                .map_err(|err| DobraError::io(format!("cannot write stderr: {err}"))),
            StreamId::File(_) => self.io.borrow_mut().write(stream, text),
        }
    }

    fn flush_stream(&mut self, stream: StreamId) -> DobraResult<()> {
        match stream {
            StreamId::Stdin => Ok(()),
            StreamId::Stdout => Ok(()),
            StreamId::Stderr => stdio::stderr()
                .flush()
                .map_err(|err| DobraError::io(format!("cannot flush stderr: {err}"))),
            StreamId::File(_) => self.io.borrow_mut().flush(stream),
        }
    }

    fn close_stream(&mut self, stream: StreamId) -> DobraResult<()> {
        match stream {
            StreamId::Stdin | StreamId::Stdout => Ok(()),
            StreamId::Stderr => self.flush_stream(stream),
            StreamId::File(_) => self.io.borrow_mut().close(stream),
        }
    }

    fn eof_stream(&mut self, stream: StreamId) -> DobraResult<bool> {
        match stream {
            StreamId::Stdin => Ok(false),
            StreamId::Stdout | StreamId::Stderr => {
                Err(DobraError::runtime("eof() expects readable stream"))
            }
            StreamId::File(_) => self.io.borrow_mut().eof(stream),
        }
    }

    fn expect_arity(&self, args: &[Value], expected: usize, name: &str) -> DobraResult<()> {
        if args.len() == expected {
            Ok(())
        } else {
            Err(DobraError::runtime(format!(
                "{name}() expects {expected} argument(s), got {}",
                args.len()
            )))
        }
    }

    fn expect_string(&self, value: &Value, name: &str, position: &str) -> DobraResult<String> {
        match value {
            Value::String(value) => Ok(value.clone()),
            other => Err(DobraError::runtime(format!(
                "{name}() expects string as {position} argument, got {}",
                other.type_name()
            ))),
        }
    }

    fn expect_stream(&self, value: &Value, name: &str, position: &str) -> DobraResult<StreamId> {
        match value {
            Value::Stream(stream) => Ok(*stream),
            other => Err(DobraError::runtime(format!(
                "{name}() expects stream as {position} argument, got {}",
                other.type_name()
            ))),
        }
    }

    fn expect_non_negative_size(
        &self,
        value: &Value,
        name: &str,
        position: &str,
    ) -> DobraResult<usize> {
        match value {
            Value::Int(value) if *value >= 0 => Ok(*value as usize),
            Value::Int(_) => Err(DobraError::runtime(format!(
                "{name}() expects non-negative size as {position} argument"
            ))),
            other => Err(DobraError::runtime(format!(
                "{name}() expects int as {position} argument, got {}",
                other.type_name()
            ))),
        }
    }

    fn add(&self, left: Value, right: Value) -> DobraResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::String(a), b) => Ok(Value::String(a + &b.to_string())),
            (a, Value::String(b)) => Ok(Value::String(a.to_string() + &b)),
            (a, b) => Err(DobraError::runtime(format!(
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
    ) -> DobraResult<Value> {
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
    ) -> DobraResult<Value> {
        let ordering = match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            _ => {
                let a = to_number(&left)?;
                let b = to_number(&right)?;
                a.partial_cmp(&b)
                    .ok_or_else(|| DobraError::runtime("cannot compare NaN"))?
            }
        };
        Ok(Value::Bool(f(ordering)))
    }

    fn index(&self, object: Value, index: Value) -> DobraResult<Value> {
        match object {
            Value::List(values) => {
                let index = match index {
                    Value::Int(value) => value,
                    other => {
                        return Err(DobraError::runtime(format!(
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
                    .ok_or_else(|| DobraError::runtime("list index out of bounds"))
            }
            Value::String(value) => {
                let index = match index {
                    Value::Int(value) => value,
                    other => {
                        return Err(DobraError::runtime(format!(
                            "string index must be int, got {}",
                            other.type_name()
                        )))
                    }
                };
                value
                    .chars()
                    .nth(index as usize)
                    .map(|ch| Value::String(ch.to_string()))
                    .ok_or_else(|| DobraError::runtime("string index out of bounds"))
            }
            Value::Map(values) => {
                let key = index.to_string();
                let value = values
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| DobraError::runtime(format!("key '{key}' not found")))?;
                self.resolve_value(value)
            }
            other => Err(DobraError::runtime(format!(
                "cannot index {}",
                other.type_name()
            ))),
        }
    }

    fn interpolate(&mut self, raw: &str) -> DobraResult<String> {
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
                    return Err(DobraError::runtime("unterminated interpolation"));
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

    fn define(&mut self, name: &str, value: Value, mutable: bool) -> DobraResult<()> {
        let scope = self.scopes.last_mut().expect("runtime always has a scope");
        if scope.contains_key(name) {
            return Err(DobraError::runtime(format!(
                "'{name}' is already defined in this scope"
            )));
        }
        scope.insert(name.to_string(), Binding { value, mutable });
        Ok(())
    }

    fn assign(&mut self, name: &str, value: Value) -> DobraResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                if let Value::UseBinding(module, export_name) = binding.value.clone() {
                    return assign_use_binding(module, &export_name, value);
                }
                if !binding.mutable {
                    return Err(DobraError::runtime(format!(
                        "cannot assign to val '{name}'"
                    )));
                }
                binding.value = value;
                return Ok(());
            }
        }
        Err(DobraError::runtime(format!("undefined variable '{name}'")))
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
            Stmt::Bind { name, mutable, .. } => Some((name.clone(), *mutable)),
            Stmt::Func { name, .. } => Some((name.clone(), false)),
            _ => None,
        })
        .collect()
}

fn statement_export_name(statement: &Stmt) -> Option<&str> {
    match statement {
        Stmt::Bind { name, .. } | Stmt::Func { name, .. } | Stmt::Assign { name, .. } => Some(name),
        _ => None,
    }
}

fn assign_use_binding(module: ModuleRef, name: &str, value: Value) -> DobraResult<()> {
    let mut module = module.borrow_mut();
    let mutable = module.mutability.get(name).copied().unwrap_or(false);
    if !mutable {
        return Err(DobraError::runtime(format!(
            "cannot assign to val '{name}'"
        )));
    }
    if !module.exports.contains_key(name) {
        return Err(DobraError::runtime(format!(
            "used binding '{name}' is not initialized yet"
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

fn to_number(value: &Value) -> DobraResult<f64> {
    match value {
        Value::Int(value) => Ok(*value as f64),
        Value::Float(value) => Ok(*value),
        other => Err(DobraError::runtime(format!(
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
        let output = run_source("val name = input.name\nemit \"Hello, {name}\"", input).unwrap();
        assert_eq!(output, "Hello, Ana");
    }

    #[test]
    fn used_functions_keep_module_bindings() {
        let dir = std::env::temp_dir().join(format!("nodia-use-capture-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.nod");
        let main = dir.join("main.nod");
        fs::write(
            &lib,
            "val prefix = \"Hi\"\nfunc greet(name) {\n  return \"{prefix}, {name}\"\n}\n",
        )
        .unwrap();
        fs::write(&main, "use './lib' as lib\nemit lib.greet(\"Ana\")\n").unwrap();

        let output = crate::run_file(&main, BTreeMap::new()).unwrap();
        assert_eq!(output, "Hi, Ana");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn direct_use_of_var_can_be_assigned() {
        let dir = std::env::temp_dir().join(format!("nodia-use-var-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bar = dir.join("bar.nod");
        let main = dir.join("main.nod");
        fs::write(&bar, "var n = 0\n").unwrap();
        fs::write(
            &main,
            "use './bar' pick n\nwhile n < 3 {\n  emit n\n  n = n + 1\n}\n",
        )
        .unwrap();

        let output = crate::run_file(&main, BTreeMap::new()).unwrap();
        assert_eq!(output, "0\n1\n2");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn direct_use_of_val_cannot_be_assigned() {
        let dir = std::env::temp_dir().join(format!("nodia-use-val-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let bar = dir.join("bar.nod");
        let main = dir.join("main.nod");
        fs::write(&bar, "val n = 0\n").unwrap();
        fs::write(&main, "use './bar' pick n\nn = n + 1\n").unwrap();

        let err = crate::run_file(&main, BTreeMap::new()).unwrap_err();
        assert!(err.to_string().contains("cannot assign to val 'n'"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn circular_uses_are_cached_and_resolved_lazily() {
        let dir = std::env::temp_dir().join(format!("nodia-circular-use-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.nod");
        let b = dir.join("b.nod");
        let main = dir.join("main.nod");
        fs::write(
            &a,
            "use './b' as b\nval name = \"A\"\nfunc pair() {\n  return \"{name}/{b.name}\"\n}\n",
        )
        .unwrap();
        fs::write(
            &b,
            "use './a' as a\nval name = \"B\"\nfunc pair() {\n  return \"{name}/{a.name}\"\n}\n",
        )
        .unwrap();
        fs::write(
            &main,
            "use './a' as a\nuse './b' as b\nemit a.pair()\nemit b.pair()\n",
        )
        .unwrap();

        let output = crate::run_file(&main, BTreeMap::new()).unwrap();
        assert_eq!(output, "A/B\nB/A");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn file_streams_read_and_write_lines() {
        let dir = std::env::temp_dir().join(format!("nodia-io-streams-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.txt");
        let output = dir.join("output.txt");
        fs::write(&input, "ana\nbruno\n").unwrap();

        let source = format!(
            r#"val src = open("{}", "read")
val out = open("{}", "write")

var line = readln(src)
while line != null {{
  writeln(out, upper(line))
  line = readln(src)
}}

close(src)
close(out)
emit read("{}")
"#,
            input.display(),
            output.display(),
            output.display()
        );
        let output_text = crate::run_source_with_options(
            &source,
            BTreeMap::new(),
            RuntimeOptions { allow_write: true },
        )
        .unwrap();

        assert_eq!(output_text, "ANA\nBRUNO");
        assert_eq!(fs::read_to_string(&output).unwrap(), "ANA\nBRUNO\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn file_writes_require_permission() {
        let dir = std::env::temp_dir().join(format!("nodia-io-denied-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let output = dir.join("output.txt");
        let source = format!("write(\"{}\", \"blocked\")", output.display());

        let err = crate::run_source_with_options(
            &source,
            BTreeMap::new(),
            RuntimeOptions { allow_write: false },
        )
        .unwrap_err();

        assert_eq!(err.code, "E3001");
        assert!(!output.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn regex_expression_renders_to_classic_regex_text() {
        let source = r#"emit regex(case_insensitive, multiline) {
  start
  named year {
    exactly 4 digit
  }
  "-"
  one_or_more char_set {
    range "a" to "z"
    digit
  }
  followed_by {
    ".log"
  }
  end
}"#;

        let output = crate::run_source(source, BTreeMap::new()).unwrap();
        assert_eq!(output, r"(?im)^(?<year>\d{4})-[a-z0-9]+(?=\.log)$");
    }

    #[test]
    fn explicit_regex_forms_render_correctly() {
        let source = r#"emit regex {
  with_flags(case_insensitive) {
    literal("abc")
  }
  one_or_more any_codepoint
  char_set {
    char(".")
    digit
  }
}"#;

        let output = crate::run_source(source, BTreeMap::new()).unwrap();
        assert_eq!(output, r"(?i:abc)[\s\S]+[.0-9]");
    }

    #[test]
    fn regex_builtins_execute_against_regex_values() {
        let source = r#"val pat = regex(case_insensitive) {
  named scheme {
    either {
      branch {
        "http"
      }
      branch {
        "https"
      }
    }
  }
  "://"
  named host {
    one_or_more {
      char_set {
        letter
        digit
        "."
        "-"
      }
    }
  }
}

val first = find("go to https://example.com now", pat)
emit test("go to https://example.com now", pat)
emit full_match("https://example.com", pat)
emit first.text
emit first.named.scheme
emit first.named.host
emit first.start
emit first.end
emit len(find_all("http://a https://b", pat))
"#;

        let output = crate::run_source(source, BTreeMap::new()).unwrap();
        assert_eq!(
            output,
            "true\ntrue\nhttps://example.com\nhttps\nexample.com\n6\n25\n2"
        );
    }

    #[test]
    fn regex_find_reports_char_offsets() {
        let source = r#"val hit = find("é ana", regex {
  named word {
    one_or_more letter
  }
})

emit hit.start
emit hit.end
"#;

        let output = crate::run_source(source, BTreeMap::new()).unwrap();
        assert_eq!(output, "2\n5");
    }

    #[test]
    fn regex_builtins_accept_string_patterns() {
        let source = r#"emit test("abc-42", "^[a-z]+-\\d+$")
emit full_match("abc-42", "^[a-z]+-\\d+$")
"#;

        let output = crate::run_source(source, BTreeMap::new()).unwrap();
        assert_eq!(output, "true\ntrue");
    }
}
