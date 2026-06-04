use super::*;

impl Runtime {
    pub(super) fn execute_use(
        &mut self,
        target: &UseTarget,
        alias: Option<&str>,
        pick: &[String],
        hide: &[String],
    ) -> DobraResult<()> {
        match target {
            UseTarget::Path(path) => {
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
            UseTarget::Stdlib(name) => self.use_stdlib_module(name, alias, pick, hide),
        }
    }

    pub(super) fn use_stdlib_module(
        &mut self,
        name: &str,
        alias: Option<&str>,
        pick: &[String],
        hide: &[String],
    ) -> DobraResult<()> {
        let Some(items) = stdlib::module_items(name) else {
            return Err(DobraError::runtime(format!(
                "unknown stdlib module '{name}'"
            )));
        };
        let mut exports = BTreeMap::new();
        for (field, target, arities) in items {
            let value = match arities {
                Some(_) => Value::BuiltinFunction((*target).to_string()),
                None => self.stdlib_binding_value(target).ok_or_else(|| {
                    DobraError::runtime(format!(
                        "stdlib module '{name}' binding '{field}' is not available"
                    ))
                })?,
            };
            exports.insert((*field).to_string(), value);
        }
        let available = exports.keys().cloned().collect::<Vec<_>>();
        let selected = self.selected_export_names(&available, name, pick, hide)?;
        exports.retain(|field, _| selected.contains(field));
        self.define(alias.unwrap_or(name), Value::Map(exports), false)
    }

    pub(super) fn stdlib_binding_value(&self, name: &str) -> Option<Value> {
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
            _ => None,
        }
    }

    pub(super) fn selected_export_names(
        &self,
        available: &[String],
        source: &str,
        pick: &[String],
        hide: &[String],
    ) -> DobraResult<Vec<String>> {
        let mut names = if pick.is_empty() {
            available.to_vec()
        } else {
            for name in pick {
                if !available.contains(name) {
                    return Err(DobraError::runtime(format!(
                        "use '{source}' does not expose '{name}'"
                    )));
                }
            }
            pick.to_vec()
        };

        names.retain(|name| !hide.contains(name));
        Ok(names)
    }

    pub(super) fn load_module(&mut self, resolved: &Path) -> DobraResult<ModuleRef> {
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

    pub(super) fn selected_use_names(
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
        self.selected_export_names(&all, &module.path.display().to_string(), pick, hide)
    }

    pub(super) fn resolve_use(&self, path: &str) -> DobraResult<PathBuf> {
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

    pub(super) fn publish_statement(&mut self, statement: &Stmt) -> DobraResult<()> {
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

    pub(super) fn export_declared_bindings(&self) -> BTreeMap<String, Value> {
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

    pub(super) fn prepare_export(&self, value: Value) -> Value {
        match value {
            Value::Function(mut function) => {
                function.captures = self.capture_root_bindings();
                Value::Function(function)
            }
            other => other,
        }
    }

    pub(super) fn capture_root_bindings(&self) -> BTreeMap<String, BindingRef> {
        self.scopes
            .first()
            .into_iter()
            .flat_map(|scope| scope.iter())
            .filter(|(name, _)| name.as_str() != "input")
            .map(|(name, binding)| (name.clone(), binding.clone()))
            .collect()
    }

    pub(super) fn capture_visible_bindings(&self) -> BTreeMap<String, BindingRef> {
        let mut captures = BTreeMap::new();
        for scope in self.scopes.iter().rev() {
            for (name, binding) in scope {
                captures
                    .entry(name.clone())
                    .or_insert_with(|| binding.clone());
            }
        }
        captures
    }

    pub(super) fn function_value(&self, name: &str, params: &[String], body: &[Stmt]) -> Value {
        let mut captures = self.capture_visible_bindings();
        let self_binding = binding_ref(Value::Null, false);
        captures.insert(name.to_string(), self_binding.clone());

        let function = Value::Function(Function {
            params: params.to_vec(),
            body: body.to_vec(),
            captures,
        });
        self_binding.borrow_mut().value = function.clone();
        function
    }

    pub(super) fn lambda_value(&self, params: &[String], body: &[Stmt]) -> Value {
        Value::Function(Function {
            params: params.to_vec(),
            body: body.to_vec(),
            captures: self.capture_visible_bindings(),
        })
    }
}
