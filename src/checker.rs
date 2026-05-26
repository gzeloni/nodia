use crate::ast::{AssignTarget, Expr, ForBinding, Program, Stmt};
use crate::error::{DobraError, DobraResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::regex;
use crate::token::{Token, TokenKind};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Symbol {
    mutable: bool,
    kind: SymbolKind,
}

#[derive(Debug, Clone)]
enum SymbolKind {
    Unknown,
    Function { arity: usize },
    Map(HashMap<String, Symbol>),
    Namespace(HashMap<String, Symbol>),
}

#[derive(Debug, Clone)]
struct ModuleInfo {
    symbols: HashMap<String, Symbol>,
}

#[derive(Debug, Clone, Default)]
struct PositionIndex {
    identifiers: HashMap<String, Vec<(usize, usize)>>,
    keywords: HashMap<&'static str, Vec<(usize, usize)>>,
}

type Scope = HashMap<String, Symbol>;

pub fn check_program(program: &Program) -> DobraResult<()> {
    Checker::new().check_program(program, None, PositionIndex::default())
}

pub fn check_program_with_tokens(
    program: &Program,
    tokens: &[Token],
    base_dir: Option<PathBuf>,
) -> DobraResult<()> {
    Checker::new().check_program(program, base_dir, PositionIndex::from_tokens(tokens))
}

pub fn check_program_at_path(program: &Program, path: &Path) -> DobraResult<()> {
    Checker::new().check_program(
        program,
        path.parent().map(Path::to_path_buf),
        PositionIndex::default(),
    )
}

pub fn check_file(path: &Path) -> DobraResult<()> {
    let source = fs::read_to_string(path)
        .map_err(|err| DobraError::io(format!("cannot read '{}': {err}", path.display())))?;
    let tokens = Lexer::new(&source)
        .tokenize()
        .map_err(|err| err.with_file(path.display().to_string()))?;
    let positions = PositionIndex::from_tokens(&tokens);
    let program = Parser::new(tokens)
        .parse_program()
        .map_err(|err| err.with_file(path.display().to_string()))?;
    Checker::new()
        .check_program(&program, path.parent().map(Path::to_path_buf), positions)
        .map_err(|err| err.with_file_if_missing(path.display().to_string()))
}

struct Checker {
    modules: HashMap<PathBuf, ModuleInfo>,
    loading: HashSet<PathBuf>,
}

impl Checker {
    fn new() -> Self {
        Self {
            modules: HashMap::new(),
            loading: HashSet::new(),
        }
    }

    fn check_program(
        &mut self,
        program: &Program,
        base_dir: Option<PathBuf>,
        positions: PositionIndex,
    ) -> DobraResult<()> {
        let mut state = State::new(self, base_dir, positions);
        state.predeclare_top_level(program)?;
        state.check_statements(&program.statements, ScopeMode::Top)
    }

    fn load_module(&mut self, path: &str, base_dir: Option<&Path>) -> DobraResult<ModuleInfo> {
        let resolved = resolve_use(path, base_dir)?;
        if let Some(info) = self.modules.get(&resolved) {
            return Ok(info.clone());
        }

        let source = fs::read_to_string(&resolved).map_err(|err| {
            DobraError::io(format!("cannot read use '{}': {err}", resolved.display()))
        })?;
        let tokens = Lexer::new(&source)
            .tokenize()
            .map_err(|err| err.with_file(resolved.display().to_string()))?;
        let positions = PositionIndex::from_tokens(&tokens);
        let program = Parser::new(tokens)
            .parse_program()
            .map_err(|err| err.with_file(resolved.display().to_string()))?;

        let info = ModuleInfo {
            symbols: declared_exports(&program),
        };
        self.modules.insert(resolved.clone(), info.clone());

        if self.loading.insert(resolved.clone()) {
            let result = self
                .check_program(
                    &program,
                    resolved.parent().map(Path::to_path_buf),
                    positions,
                )
                .map_err(|err| err.with_file_if_missing(resolved.display().to_string()));
            self.loading.remove(&resolved);
            result?;
        }

        Ok(info)
    }
}

struct State<'a> {
    checker: &'a mut Checker,
    scopes: Vec<Scope>,
    base_dir: Option<PathBuf>,
    positions: PositionIndex,
    loop_depth: usize,
    function_depth: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeMode {
    Top,
    Nested,
}

enum FieldStatus {
    Found(Symbol),
    Missing,
    Unknown,
}

impl<'a> State<'a> {
    fn new(checker: &'a mut Checker, base_dir: Option<PathBuf>, positions: PositionIndex) -> Self {
        let mut root = HashMap::new();
        root.insert("input".to_string(), Symbol::unknown(false));
        root.insert("stdin".to_string(), Symbol::unknown(false));
        root.insert("stdout".to_string(), Symbol::unknown(false));
        root.insert("stderr".to_string(), Symbol::unknown(false));
        Self {
            checker,
            scopes: vec![root],
            base_dir,
            positions,
            loop_depth: 0,
            function_depth: 0,
        }
    }

    fn predeclare_top_level(&mut self, program: &Program) -> DobraResult<()> {
        for statement in &program.statements {
            match statement {
                Stmt::Use {
                    path,
                    alias,
                    pick,
                    hide,
                } => self.declare_use(path, alias.as_deref(), pick, hide)?,
                Stmt::Bind { name, mutable, .. } => {
                    self.declare(name, Symbol::unknown(*mutable))?
                }
                Stmt::Func { name, params, .. } => {
                    self.declare(name, Symbol::function(params.len(), false))?
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_statements(&mut self, statements: &[Stmt], mode: ScopeMode) -> DobraResult<()> {
        for statement in statements {
            self.check_statement(statement, mode)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, statement: &Stmt, mode: ScopeMode) -> DobraResult<()> {
        match statement {
            Stmt::Comment(_) => Ok(()),
            Stmt::Use {
                path,
                alias,
                pick,
                hide,
            } => {
                if mode != ScopeMode::Top {
                    self.declare_use(path, alias.as_deref(), pick, hide)?;
                }
                Ok(())
            }
            Stmt::Bind {
                name,
                value,
                mutable,
            } => {
                self.check_expr(value)?;
                let symbol = self.symbol_for_expr(value, *mutable);
                if mode == ScopeMode::Top {
                    self.update_symbol(name, symbol)?;
                } else {
                    self.declare(name, symbol)?;
                }
                Ok(())
            }
            Stmt::Assign { target, value } => {
                self.check_assign_target(target, true)?;
                self.check_expr(value)?;
                self.apply_assignment_symbol(target, value)
            }
            Stmt::Func { name, params, body } => {
                if mode != ScopeMode::Top {
                    self.declare(name, Symbol::function(params.len(), false))?;
                }
                self.check_function(params, body)
            }
            Stmt::Return(value) => {
                if self.function_depth == 0 {
                    return Err(self.error_keyword("E4103", "return outside function", "return"));
                }
                if let Some(value) = value {
                    self.check_expr(value)?;
                }
                Ok(())
            }
            Stmt::Emit(expr) | Stmt::Expr(expr) => self.check_expr(expr),
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_expr(condition)?;
                self.check_block(then_branch)?;
                self.check_block(else_branch)
            }
            Stmt::For {
                binding,
                iterable,
                body,
            } => {
                self.check_expr(iterable)?;
                self.loop_depth += 1;
                self.push_scope();
                match binding {
                    ForBinding::Single(name) => self.declare(name, Symbol::unknown(true))?,
                    ForBinding::Pair { key, value } => {
                        self.declare(key, Symbol::unknown(true))?;
                        self.declare(value, Symbol::unknown(true))?;
                    }
                }
                let result = self.check_block(body);
                self.pop_scope();
                self.loop_depth -= 1;
                result
            }
            Stmt::While { condition, body } => {
                self.check_expr(condition)?;
                self.loop_depth += 1;
                let result = self.check_block(body);
                self.loop_depth -= 1;
                result
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    return Err(self.error_keyword("E4103", "break outside loop", "break"));
                }
                Ok(())
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    return Err(self.error_keyword("E4103", "continue outside loop", "continue"));
                }
                Ok(())
            }
        }
    }

    fn check_block(&mut self, statements: &[Stmt]) -> DobraResult<()> {
        self.push_scope();
        let result = self.check_statements(statements, ScopeMode::Nested);
        self.pop_scope();
        result
    }

    fn check_function(&mut self, params: &[String], body: &[Stmt]) -> DobraResult<()> {
        self.function_depth += 1;
        self.push_scope();
        let mut seen = HashSet::new();
        for param in params {
            if !seen.insert(param) {
                return Err(self.error_name(
                    "E4102",
                    format!("duplicate parameter '{param}'"),
                    param,
                ));
            }
            self.declare(param, Symbol::unknown(true))?;
        }
        let result = self.check_block(body);
        self.pop_scope();
        self.function_depth -= 1;
        result
    }

    fn check_expr(&mut self, expr: &Expr) -> DobraResult<()> {
        match expr {
            Expr::Literal(value) => {
                if let crate::value::Value::String(value) = value {
                    self.check_interpolations(value)?;
                }
                Ok(())
            }
            Expr::Regex(pattern) => regex::validate(pattern),
            Expr::Identifier(name) => {
                if self.lookup(name).is_some() {
                    Ok(())
                } else {
                    Err(self.error_name("E4100", format!("undefined variable '{name}'"), name))
                }
            }
            Expr::Unary { expr, .. } => self.check_expr(expr),
            Expr::Binary { left, right, .. } => {
                self.check_expr(left)?;
                self.check_expr(right)
            }
            Expr::Call { callee, args } => self.check_call(callee, args),
            Expr::Get { object, field } => {
                match self.field_status(object, field) {
                    FieldStatus::Found(_) | FieldStatus::Unknown => {}
                    FieldStatus::Missing => {
                        return Err(self.error_name(
                            "E4105",
                            format!("field '{field}' not found"),
                            field,
                        ));
                    }
                }
                self.check_expr(object)
            }
            Expr::Index { object, index } => {
                self.check_expr(object)?;
                self.check_expr(index)
            }
            Expr::List(values) => {
                for value in values {
                    self.check_expr(value)?;
                }
                Ok(())
            }
            Expr::Map(pairs) => {
                for (_, value) in pairs {
                    self.check_expr(value)?;
                }
                Ok(())
            }
        }
    }

    fn check_call(&mut self, callee: &Expr, args: &[Expr]) -> DobraResult<()> {
        for arg in args {
            self.check_expr(arg)?;
        }

        if let Some(name) = direct_identifier(callee) {
            if let Some(arity) = builtin_arity(name) {
                self.check_arity(name, args.len(), &arity)?;
                return Ok(());
            }
            let Some(symbol) = self.lookup(name) else {
                return Err(self.error_name("E4100", format!("undefined variable '{name}'"), name));
            };
            if let SymbolKind::Function { arity } = &symbol.kind {
                self.check_arity(name, args.len(), &[*arity])?;
            }
            return Ok(());
        }

        if let Expr::Get { object, field } = callee {
            match self.field_status(object, field) {
                FieldStatus::Found(symbol) => {
                    if let SymbolKind::Function { arity } = symbol.kind {
                        self.check_arity(field, args.len(), &[arity])?;
                    }
                    self.check_expr(object)?;
                    return Ok(());
                }
                FieldStatus::Missing => {
                    return Err(self.error_name(
                        "E4105",
                        format!("field '{field}' not found"),
                        field,
                    ));
                }
                FieldStatus::Unknown => {}
            }
        }

        self.check_expr(callee)
    }

    fn check_assign_target(
        &mut self,
        target: &AssignTarget,
        final_step: bool,
    ) -> DobraResult<Option<Symbol>> {
        match target {
            AssignTarget::Identifier(name) => {
                let Some(symbol) = self.lookup(name).cloned() else {
                    return Err(self.error_name(
                        "E4100",
                        format!("undefined variable '{name}'"),
                        name,
                    ));
                };
                if !symbol.mutable
                    && (final_step || !matches!(symbol.kind, SymbolKind::Namespace(_)))
                {
                    return Err(self.error_name(
                        "E4101",
                        format!("cannot assign to val '{name}'"),
                        name,
                    ));
                }
                Ok(Some(symbol))
            }
            AssignTarget::Get { object, field } => {
                let object_symbol = self.check_assign_target(object, false)?;
                match object_symbol.map(|symbol| symbol.kind) {
                    Some(SymbolKind::Namespace(fields)) => {
                        let Some(field_symbol) = fields.get(field).cloned() else {
                            return Err(self.error_name(
                                "E4105",
                                format!("field '{field}' not found"),
                                field,
                            ));
                        };
                        if !field_symbol.mutable {
                            return Err(self.error_name(
                                "E4101",
                                format!("cannot assign to val '{field}'"),
                                field,
                            ));
                        }
                        Ok(Some(field_symbol))
                    }
                    Some(SymbolKind::Map(fields)) => {
                        if final_step {
                            Ok(fields.get(field).cloned())
                        } else {
                            let Some(field_symbol) = fields.get(field).cloned() else {
                                return Err(self.error_name(
                                    "E4105",
                                    format!("field '{field}' not found"),
                                    field,
                                ));
                            };
                            Ok(Some(field_symbol))
                        }
                    }
                    Some(SymbolKind::Unknown | SymbolKind::Function { .. }) | None => Ok(None),
                }
            }
            AssignTarget::Index { object, index } => {
                self.check_expr(index)?;
                self.check_assign_target(object, false)?;
                Ok(None)
            }
        }
    }

    fn apply_assignment_symbol(&mut self, target: &AssignTarget, value: &Expr) -> DobraResult<()> {
        let Some(root_name) = assign_target_root_name(target) else {
            return Ok(());
        };
        let Some(root_symbol) = self.lookup(root_name).cloned() else {
            return Err(self.error_name(
                "E4100",
                format!("undefined variable '{root_name}'"),
                root_name,
            ));
        };
        let value_symbol = self.symbol_for_expr(value, false);
        let steps = assignment_symbol_steps(target);
        let updated = self.update_symbol_after_assignment(root_symbol, &steps, value_symbol);
        self.update_symbol(root_name, updated)
    }

    fn update_symbol_after_assignment(
        &self,
        symbol: Symbol,
        steps: &[AssignmentSymbolStep],
        value_symbol: Symbol,
    ) -> Symbol {
        if steps.is_empty() {
            return Symbol {
                mutable: symbol.mutable,
                kind: value_symbol.kind,
            };
        }

        match &steps[0] {
            AssignmentSymbolStep::UnknownIndex => symbol,
            AssignmentSymbolStep::Field(field) => {
                self.update_symbol_field(symbol, field, &steps[1..], value_symbol)
            }
        }
    }

    fn update_symbol_field(
        &self,
        symbol: Symbol,
        field: &str,
        rest: &[AssignmentSymbolStep],
        value_symbol: Symbol,
    ) -> Symbol {
        let mutable = symbol.mutable;
        let kind = match symbol.kind {
            SymbolKind::Map(mut fields) => {
                let current = fields
                    .remove(field)
                    .unwrap_or_else(|| Symbol::unknown(false));
                let updated = self.update_symbol_after_assignment(current, rest, value_symbol);
                fields.insert(field.to_string(), updated);
                SymbolKind::Map(fields)
            }
            SymbolKind::Namespace(mut fields) => {
                if let Some(current) = fields.remove(field) {
                    let updated = self.update_symbol_after_assignment(current, rest, value_symbol);
                    fields.insert(field.to_string(), updated);
                }
                SymbolKind::Namespace(fields)
            }
            other => other,
        };
        Symbol { mutable, kind }
    }

    fn check_interpolations(&mut self, raw: &str) -> DobraResult<()> {
        let chars = raw.chars().collect::<Vec<_>>();
        let mut index = 0;
        while index < chars.len() {
            if chars[index] == '{' {
                if chars.get(index + 1) == Some(&'{') {
                    index += 2;
                    continue;
                }
                let start = index + 1;
                let mut end = start;
                while end < chars.len() && chars[end] != '}' {
                    end += 1;
                }
                if end == chars.len() {
                    return Err(semantic("E4106", "unterminated interpolation", None));
                }
                let expr_text = chars[start..end].iter().collect::<String>();
                let tokens = Lexer::new(&expr_text).tokenize()?;
                let expr = Parser::new(tokens).parse_expression_only()?;
                self.check_expr(&expr)?;
                index = end + 1;
            } else if chars[index] == '}' && chars.get(index + 1) == Some(&'}') {
                index += 2;
            } else {
                index += 1;
            }
        }
        Ok(())
    }

    fn declare_use(
        &mut self,
        path: &str,
        alias: Option<&str>,
        pick: &[String],
        hide: &[String],
    ) -> DobraResult<()> {
        let module = self.checker.load_module(path, self.base_dir.as_deref())?;

        if let Some(alias) = alias {
            let symbols = self.selected_symbols(&module, pick, hide)?;
            return self.declare(alias, Symbol::namespace(symbols));
        }

        for (name, symbol) in self.selected_symbols(&module, pick, hide)? {
            self.declare(&name, symbol)?;
        }
        Ok(())
    }

    fn selected_symbols(
        &self,
        module: &ModuleInfo,
        pick: &[String],
        hide: &[String],
    ) -> DobraResult<HashMap<String, Symbol>> {
        let mut selected = HashMap::new();
        if pick.is_empty() {
            for (name, symbol) in &module.symbols {
                if !hide.contains(name) {
                    selected.insert(name.clone(), symbol.clone());
                }
            }
            return Ok(selected);
        }

        for name in pick {
            let Some(symbol) = module.symbols.get(name) else {
                return Err(self.error_name(
                    "E4104",
                    format!("use does not expose '{name}'"),
                    name,
                ));
            };
            if !hide.contains(name) {
                selected.insert(name.clone(), symbol.clone());
            }
        }
        Ok(selected)
    }

    fn declare(&mut self, name: &str, symbol: Symbol) -> DobraResult<()> {
        let scope = self.scopes.last_mut().expect("checker always has a scope");
        if scope.contains_key(name) {
            return Err(self.error_name(
                "E4102",
                format!("'{name}' is already defined in this scope"),
                name,
            ));
        }
        scope.insert(name.to_string(), symbol);
        Ok(())
    }

    fn update_symbol(&mut self, name: &str, symbol: Symbol) -> DobraResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), symbol);
                return Ok(());
            }
        }
        Err(self.error_name("E4100", format!("undefined variable '{name}'"), name))
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn field_status(&self, object: &Expr, field: &str) -> FieldStatus {
        let Some(symbol) = self.symbol_from_access(object) else {
            return FieldStatus::Unknown;
        };
        match symbol.kind {
            SymbolKind::Map(fields) | SymbolKind::Namespace(fields) => fields
                .get(field)
                .cloned()
                .map(FieldStatus::Found)
                .unwrap_or(FieldStatus::Missing),
            SymbolKind::Unknown | SymbolKind::Function { .. } => FieldStatus::Unknown,
        }
    }

    fn symbol_from_access(&self, expr: &Expr) -> Option<Symbol> {
        match expr {
            Expr::Identifier(name) => self.lookup(name).cloned(),
            Expr::Get { object, field } => match self.field_status(object, field) {
                FieldStatus::Found(symbol) => Some(symbol),
                FieldStatus::Missing | FieldStatus::Unknown => None,
            },
            _ => None,
        }
    }

    fn symbol_for_expr(&self, expr: &Expr, mutable: bool) -> Symbol {
        let kind = match expr {
            Expr::Map(pairs) => {
                let mut fields = HashMap::new();
                for (key, value) in pairs {
                    fields.insert(key.clone(), self.symbol_for_expr(value, false));
                }
                SymbolKind::Map(fields)
            }
            Expr::Identifier(_) | Expr::Get { .. } => self
                .symbol_from_access(expr)
                .map(|symbol| symbol.kind)
                .unwrap_or(SymbolKind::Unknown),
            _ => SymbolKind::Unknown,
        };
        Symbol { mutable, kind }
    }

    fn check_arity(&self, name: &str, got: usize, expected: &[usize]) -> DobraResult<()> {
        if expected.contains(&got) {
            return Ok(());
        }
        let expected = expected
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(" or ");
        Err(self.error_name(
            "E4107",
            format!("{name}() expects {expected} argument(s), got {got}"),
            name,
        ))
    }

    fn error_name(&self, code: &'static str, message: impl Into<String>, name: &str) -> DobraError {
        semantic(code, message, self.positions.identifier(name))
    }

    fn error_keyword(
        &self,
        code: &'static str,
        message: impl Into<String>,
        keyword: &'static str,
    ) -> DobraError {
        semantic(code, message, self.positions.keyword(keyword))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

impl Symbol {
    fn unknown(mutable: bool) -> Self {
        Self {
            mutable,
            kind: SymbolKind::Unknown,
        }
    }

    fn function(arity: usize, mutable: bool) -> Self {
        Self {
            mutable,
            kind: SymbolKind::Function { arity },
        }
    }

    fn namespace(symbols: HashMap<String, Symbol>) -> Self {
        Self {
            mutable: false,
            kind: SymbolKind::Namespace(symbols),
        }
    }
}

impl PositionIndex {
    fn from_tokens(tokens: &[Token]) -> Self {
        let mut index = Self::default();
        for token in tokens {
            match &token.kind {
                TokenKind::Identifier(name) => index
                    .identifiers
                    .entry(name.clone())
                    .or_default()
                    .push((token.line, token.column)),
                kind => {
                    if let Some(keyword) = keyword_name(kind) {
                        index
                            .keywords
                            .entry(keyword)
                            .or_default()
                            .push((token.line, token.column));
                    }
                }
            }
        }
        index
    }

    fn identifier(&self, name: &str) -> Option<(usize, usize)> {
        self.identifiers
            .get(name)
            .and_then(|positions| positions.first().copied())
    }

    fn keyword(&self, keyword: &'static str) -> Option<(usize, usize)> {
        self.keywords
            .get(keyword)
            .and_then(|positions| positions.first().copied())
    }
}

fn declared_exports(program: &Program) -> HashMap<String, Symbol> {
    let mut symbols = HashMap::new();
    for statement in &program.statements {
        match statement {
            Stmt::Bind {
                name,
                value,
                mutable,
            } => {
                symbols.insert(name.clone(), static_symbol_for_expr(value, *mutable));
            }
            Stmt::Func { name, params, .. } => {
                symbols.insert(name.clone(), Symbol::function(params.len(), false));
            }
            _ => {}
        }
    }
    symbols
}

fn static_symbol_for_expr(expr: &Expr, mutable: bool) -> Symbol {
    let kind = match expr {
        Expr::Map(pairs) => {
            let mut fields = HashMap::new();
            for (key, value) in pairs {
                fields.insert(key.clone(), static_symbol_for_expr(value, false));
            }
            SymbolKind::Map(fields)
        }
        _ => SymbolKind::Unknown,
    };
    Symbol { mutable, kind }
}

fn resolve_use(path: &str, base_dir: Option<&Path>) -> DobraResult<PathBuf> {
    let raw = Path::new(path);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base_dir.unwrap_or_else(|| Path::new(".")).join(raw)
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

fn direct_identifier(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Identifier(name) => Some(name),
        _ => None,
    }
}

fn builtin_arity(name: &str) -> Option<Vec<usize>> {
    let arity = match name {
        "range" | "read" => vec![1, 2],
        "upper" | "uppercase" | "lower" | "lowercase" | "capitalize" | "trim" | "dedent"
        | "lines" | "unlines" | "words" | "keys" | "values" | "entries" | "len" | "int"
        | "float" | "string" | "bool" | "abs" | "floor" | "ceil" | "round" | "sqrt" | "sum"
        | "avg" | "pop" | "first" | "last" | "reverse" | "sort" | "unique" | "close" | "flush"
        | "eof" | "readln" => {
            vec![1]
        }
        "replace" | "replace_all" | "clamp" | "slice" => vec![3],
        "split" | "split_regex" | "join" | "contains" | "starts" | "starts_with" | "ends"
        | "ends_with" | "indent" | "pow" | "min" | "max" | "push" | "open" | "write"
        | "writeln" | "append" | "test" | "full_match" | "find" | "find_all" => {
            vec![2]
        }
        _ => return None,
    };
    Some(arity)
}

fn keyword_name(kind: &TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::Val => Some("val"),
        TokenKind::Var => Some("var"),
        TokenKind::Func => Some("func"),
        TokenKind::LegacyLet => Some("let"),
        TokenKind::LegacyConst => Some("const"),
        TokenKind::LegacyFn => Some("fn"),
        TokenKind::Return => Some("return"),
        TokenKind::Emit => Some("emit"),
        TokenKind::If => Some("if"),
        TokenKind::Else => Some("else"),
        TokenKind::For => Some("for"),
        TokenKind::In => Some("in"),
        TokenKind::While => Some("while"),
        TokenKind::Break => Some("break"),
        TokenKind::Continue => Some("continue"),
        TokenKind::LegacyImport => Some("import"),
        TokenKind::Use => Some("use"),
        TokenKind::Regex => Some("regex"),
        TokenKind::As => Some("as"),
        TokenKind::Pick => Some("pick"),
        TokenKind::LegacyShow => Some("show"),
        TokenKind::Hide => Some("hide"),
        _ => None,
    }
}

fn semantic(
    code: &'static str,
    message: impl Into<String>,
    position: Option<(usize, usize)>,
) -> DobraError {
    let (line, column) = position.unwrap_or((0, 0));
    DobraError::semantic_at(message, line, column).with_code(code)
}

#[derive(Clone)]
enum AssignmentSymbolStep {
    Field(String),
    UnknownIndex,
}

fn assign_target_root_name(target: &AssignTarget) -> Option<&str> {
    match target {
        AssignTarget::Identifier(name) => Some(name),
        AssignTarget::Get { object, .. } | AssignTarget::Index { object, .. } => {
            assign_target_root_name(object)
        }
    }
}

fn assignment_symbol_steps(target: &AssignTarget) -> Vec<AssignmentSymbolStep> {
    let mut steps = Vec::new();
    collect_assignment_symbol_steps(target, &mut steps);
    steps
}

fn collect_assignment_symbol_steps(target: &AssignTarget, steps: &mut Vec<AssignmentSymbolStep>) {
    match target {
        AssignTarget::Identifier(_) => {}
        AssignTarget::Get { object, field } => {
            collect_assignment_symbol_steps(object, steps);
            steps.push(AssignmentSymbolStep::Field(field.clone()));
        }
        AssignTarget::Index { object, index } => {
            collect_assignment_symbol_steps(object, steps);
            if let Some(key) = static_map_key(index) {
                steps.push(AssignmentSymbolStep::Field(key));
            } else {
                steps.push(AssignmentSymbolStep::UnknownIndex);
            }
        }
    }
}

fn static_map_key(index: &Expr) -> Option<String> {
    match index {
        Expr::Literal(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::check_source;

    #[test]
    fn checker_accepts_text_builtins() {
        let source = r#"emit len(lines("a
b"))
emit unlines(["up", "down"])
emit len(words("one  two   three"))
emit test("abc", regex { one_or_more letter })
emit full_match("abc", regex { one_or_more letter })
emit find("abc", regex { one_or_more letter })
emit find_all("abc", regex { one_or_more letter })
emit replace("abc123", regex { one_or_more digit }, '#')
emit replace_all("abc123", regex { one_or_more digit }, '#')
emit split("ana   bruno", regex { one_or_more whitespace })
emit split_regex("ana   bruno", regex { one_or_more whitespace })
"#;

        assert!(check_source(source).is_ok());
    }

    #[test]
    fn checker_reports_first_identifier_occurrence() {
        let err = check_source("emit missing()\nemit missing()\n").unwrap_err();

        assert_eq!(err.code, "E4100");
        assert_eq!((err.line, err.column), (1, 6));
    }

    #[test]
    fn checker_accepts_map_field_assignment_that_builds_shape() {
        let source = r#"var user = {}
user.name = "Ana"
emit user.name
"#;

        assert!(check_source(source).is_ok());
    }

    #[test]
    fn checker_rejects_nested_assignment_through_val_root() {
        let err = check_source("val user = {}\nuser.name = \"Ana\"\n").unwrap_err();

        assert_eq!(err.code, "E4101");
        assert_eq!((err.line, err.column), (1, 5));
    }

    #[test]
    fn checker_reports_first_keyword_occurrence() {
        let err = check_source("return\nreturn\n").unwrap_err();

        assert_eq!(err.code, "E4103");
        assert_eq!((err.line, err.column), (1, 1));
    }
}
