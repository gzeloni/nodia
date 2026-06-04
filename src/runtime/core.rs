use super::*;

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

    pub(super) fn with_context(
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
            binding_ref(Value::Map(input.clone()), false),
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
            let flow = match self.execute(statement) {
                Ok(flow) => flow,
                Err(err) if err.exit_status.is_some() => {
                    self.io.borrow_mut().flush_all()?;
                    return Err(err.with_output(self.output.trim_end_matches('\n').to_string()));
                }
                Err(err) => return Err(err),
            };
            match flow {
                Flow::None => self.publish_statement(statement)?,
                Flow::Return(_) => return Err(DobraError::runtime("return outside function")),
                Flow::Break => return Err(DobraError::runtime("break outside loop")),
                Flow::Continue => return Err(DobraError::runtime("continue outside loop")),
            }
        }
        self.io.borrow_mut().flush_all()?;
        Ok(self.output.trim_end_matches('\n').to_string())
    }

    pub(super) fn execute_block(&mut self, statements: &[Stmt]) -> DobraResult<Flow> {
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

    pub(super) fn execute(&mut self, statement: &Stmt) -> DobraResult<Flow> {
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
                self.define(name, self.function_value(name, params, body), false)?;
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
                binding,
                iterable,
                body,
            } => {
                let iterable = self.eval(iterable)?;
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
}
